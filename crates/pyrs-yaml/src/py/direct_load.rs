//! Direct granit-event → Python-object materialization for the value-only
//! load family (`safe_load` / `safe_loads` / `YAML().safe_load*`).
//!
//! The AST exists to serve round-trip editing; a value-only load builds it
//! only to walk it again in `convert.rs`. This fast path replaces both the
//! `CustomNode` materialization and the second traversal with a single
//! pipeline: collect the granit event stream (the same stream
//! `AstReceiver` consumes; events borrow from the source text and carry no
//! AST allocation), then build Python objects by recursive descent.
//!
//! Semantics mirror the AST path exactly:
//! - plain scalars resolve through the active schema; quoted scalars are
//!   always strings (YAML 1.2 rule, see `convert::scalar_to_pyobject`);
//! - mapping keys are the raw scalar text of the key node, never resolved;
//! - duplicate keys are last-wins and recorded as
//!   [`ParseError::DuplicateKey`] unless `allow_duplicate_keys` or the key
//!   resolves to null (the `is_null_key` exemption in `push_node`);
//! - depth is bounded by the same `max_depth` contract.
//!
//! Constructs outside the replicable surface — anchors, aliases, tags,
//! merge keys, non-scalar keys, multi-document streams, any granit parse
//! error — make the attempt `Bail`, and the caller reruns the AST
//! pipeline, which owns the authoritative diagnostics. The builder runs
//! under the GIL throughout: Python-object construction is the dominant
//! cost, so no `py.detach` here (AGENTS.md).

use granit_parser::{Event, Parser as SaphyrParser, Span, SpannedEventReceiver};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList};
use pyrs_yaml_core::parser::yaml::{Schema, YamlType};
use pyrs_yaml_core::parser::{DepthError, ParseError};

/// Collected event with the span position granit reports at.
struct Ev<'a> {
    event: Event<'a>,
    #[allow(dead_code)]
    span: Span,
}

/// Phase 1: drain the parser into a Vec, bailing on anything the direct
/// surface cannot reproduce. Parse errors also bail: the AST path reruns
/// and produces the authoritative, byte-identical diagnostic.
struct Collector<'a> {
    events: Vec<Ev<'a>>,
    bail: bool,
    docs: usize,
}

impl<'a> Collector<'a> {
    fn push(&mut self, event: Event<'a>, span: Span) {
        self.events.push(Ev { event, span });
    }
}

impl<'a> SpannedEventReceiver<'a> for Collector<'a> {
    fn on_event(&mut self, event: Event<'a>, span: Span) {
        if self.bail {
            return;
        }
        match &event {
            Event::StreamStart | Event::StreamEnd | Event::Comment(..) => {}
            Event::DocumentStart(..) => {
                self.docs += 1;
                if self.docs > 1 {
                    self.bail = true;
                }
            }
            Event::DocumentEnd => {}
            // Aliases, anchors and tags belong to the AST-only surface.
            Event::Alias(_) => self.bail = true,
            Event::Scalar(_, _, anchor_id, tag)
            | Event::MappingStart(_, anchor_id, tag)
            | Event::SequenceStart(_, anchor_id, tag) => {
                if *anchor_id != 0 || tag.is_some() {
                    self.bail = true;
                    return;
                }
                self.push(event, span);
            }
            Event::MappingEnd | Event::SequenceEnd => self.push(event, span),
            _ => self.bail = true, // granit Event is #[non_exhaustive]
        }
    }
}

/// Phase 2: recursive descent over the collected events.
struct Builder<'py, 'a> {
    py: Python<'py>,
    schema: &'py Schema,
    pos: usize,
    events: &'a [Ev<'a>],
    duplicate_key: Option<String>,
    error: Option<PyErr>,
    max_depth_hit: bool,
    allow_duplicate_keys: bool,
    max_depth: usize,
}

impl<'py, 'a> Builder<'py, 'a> {
    fn new(
        py: Python<'py>,
        schema: &'py Schema,
        events: &'a [Ev<'a>],
        max_depth: usize,
        allow_duplicate_keys: bool,
    ) -> Self {
        Self {
            py,
            schema,
            pos: 0,
            events,
            duplicate_key: None,
            error: None,
            max_depth_hit: false,
            allow_duplicate_keys,
            max_depth,
        }
    }

    fn next_event(&mut self) -> Option<&'a Event<'a>> {
        self.events.get(self.pos).map(|e| {
            self.pos += 1;
            &e.event
        })
    }

    fn scalar_to_py(&self, value: &str, plain: bool) -> PyResult<Py<PyAny>> {
        if plain {
            Ok(match self.schema.resolve(value) {
                YamlType::Null => self.py.None().into_any(),
                YamlType::Bool(b) => PyBool::new(self.py, b).to_owned().into_any().unbind(),
                YamlType::Int(n) => n.into_pyobject(self.py)?.into_any().unbind(),
                YamlType::Float(f) => f.into_pyobject(self.py)?.into_any().unbind(),
                YamlType::Str(s) => s.into_pyobject(self.py)?.into_any().unbind(),
            })
        } else {
            Ok(value.into_pyobject(self.py)?.into_any().unbind())
        }
    }

    fn is_null_key_text(&self, text: &str) -> bool {
        matches!(self.schema.resolve(text), YamlType::Null)
    }

    /// Build one content node at `depth`. Returns `None` on bail/error
    /// (state flags carry the reason). `depth` counts open containers
    /// above this node, matching the AST's `stack.len()` contract.
    fn build(&mut self, depth: usize) -> Option<Py<PyAny>> {
        match self.next_event()? {
            Event::Scalar(value, style, _, _) => {
                let plain = matches!(style, granit_parser::ScalarStyle::Plain);
                self.scalar_to_py(value, plain)
                    .map_err(|e| self.error = Some(e))
                    .ok()
            }
            Event::MappingStart(..) => {
                if depth >= self.max_depth {
                    self.max_depth_hit = true;
                    return None;
                }
                self.build_map(depth)
            }
            Event::SequenceStart(..) => {
                if depth >= self.max_depth {
                    self.max_depth_hit = true;
                    return None;
                }
                self.build_seq(depth)
            }
            _ => {
                // DocumentStart arrives before root content; skip wrappers.
                self.skip_doc_wrappers();
                self.build(depth)
            }
        }
    }

    fn skip_doc_wrappers(&mut self) {
        while let Some(ev) = self.events.get(self.pos) {
            if matches!(
                ev.event,
                Event::StreamStart
                    | Event::StreamEnd
                    | Event::DocumentStart(..)
                    | Event::Comment(..)
            ) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn build_map(&mut self, depth: usize) -> Option<Py<PyAny>> {
        let dict = PyDict::new(self.py);
        loop {
            match self.events.get(self.pos).map(|e| &e.event) {
                Some(Event::MappingEnd) => {
                    self.pos += 1;
                    return Some(dict.into_any().unbind());
                }
                Some(Event::Scalar(key_text, style, _, _)) => {
                    // Keys are raw text; merge keys were rejected during
                    // collection, so every scalar here is a legal key.
                    let _ = style;
                    let key = key_text.to_string();
                    self.pos += 1;
                    let value = self.build(depth + 1)?;
                    if !self.allow_duplicate_keys
                        && !self.is_null_key_text(&key)
                        && dict.contains(&key).unwrap_or(false)
                        && self.duplicate_key.is_none()
                    {
                        self.duplicate_key = Some(key.clone());
                    }
                    // Last-wins at the original position = IndexMap::insert.
                    if dict.set_item(key.as_str(), value).is_err() {
                        return None;
                    }
                }
                _ => {
                    // Non-scalar key (or malformed): outside the surface.
                    return None;
                }
            }
        }
    }

    fn build_seq(&mut self, depth: usize) -> Option<Py<PyAny>> {
        let list = PyList::empty(self.py);
        loop {
            match self.events.get(self.pos).map(|e| &e.event) {
                Some(Event::SequenceEnd) => {
                    self.pos += 1;
                    return Some(list.into_any().unbind());
                }
                _ => {
                    let item = self.build(depth + 1)?;
                    if list.append(item).is_err() {
                        return None;
                    }
                }
            }
        }
    }
}

/// Outcome of a direct load attempt.
pub(crate) enum DirectOutcome {
    /// Fully materialized Python value.
    Done(Py<PyAny>),
    /// Something outside the direct surface: rerun via the AST path.
    Bail,
    /// A genuine, AST-identical error detected during building.
    Fail(ParseError),
}

pub(crate) fn try_direct_load(
    py: Python<'_>,
    yaml_src: &str,
    schema: &Schema,
    max_depth: usize,
    allow_duplicate_keys: bool,
) -> DirectOutcome {
    // Merge keys need a byte-level veto: "<<: x" is structurally a plain
    // scalar key the collector would otherwise accept... cheap one-pass.
    if yaml_src.contains("<<") {
        return DirectOutcome::Bail;
    }
    let mut collector = Collector {
        events: Vec::new(),
        bail: false,
        docs: 0,
    };
    let mut parser = SaphyrParser::new_from_str(yaml_src);
    match parser.load(&mut collector, true) {
        Ok(()) if !collector.bail => {}
        _ => return DirectOutcome::Bail,
    }
    let mut builder = Builder::new(
        py,
        schema,
        &collector.events,
        max_depth,
        allow_duplicate_keys,
    );
    // Root: skip stream/document wrappers, build exactly one content node,
    // then require the rest to be document-end/stream-end.
    builder.skip_doc_wrappers();
    let root = if builder.pos >= builder.events.len() {
        // Empty stream: the AST path returns plain null for this input.
        py.None()
    } else {
        match builder.build(0) {
            Some(v) => v,
            None if builder.max_depth_hit => {
                return DirectOutcome::Fail(ParseError::MaxDepthExceeded(DepthError(max_depth)));
            }
            None => return DirectOutcome::Bail,
        }
    };
    builder.skip_doc_wrappers();
    // Trailing content (multi-doc survived the collector count check, or a
    // second root node) is out of surface.
    if builder.pos < builder.events.len()
        && builder
            .events
            .iter()
            .skip(builder.pos)
            .any(|e| !matches!(e.event, Event::DocumentEnd | Event::StreamEnd))
    {
        return DirectOutcome::Bail;
    }
    if let Some(e) = builder.error {
        let _ = e;
        return DirectOutcome::Bail; // reproduce Python-side failures via AST path
    }
    if let Some(key) = builder.duplicate_key {
        return DirectOutcome::Fail(ParseError::DuplicateKey(key));
    }
    DirectOutcome::Done(root)
}
