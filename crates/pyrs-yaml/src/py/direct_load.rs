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
//! The collection, and the decision that a document is eligible at all, live
//! in the engine (`pyrs_yaml_core::parser::yaml::direct`): eligibility is a
//! property of the document and the grammar, not of the Python object the
//! value becomes. What stays here is the materialization, the only part that
//! needs a Python object to exist.
//!
//! Semantics mirror the AST path exactly:
//! - plain scalars resolve through the active schema; quoted scalars are
//!   always strings (YAML 1.2 rule, see `convert::scalar_to_pyobject`);
//! - a mapping key is resolved by exactly that rule too, because it is the same
//!   kind of node as a value (`convert.rs` reuses its value path outright);
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

use granit_parser::Event;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList};
use pyrs_yaml_core::parser::yaml::{DirectEvent, Schema, YamlType, replicable_events};
use pyrs_yaml_core::parser::{DepthError, ParseError};

/// Phase 2: recursive descent over the collected events.
struct Builder<'py, 'a> {
    py: Python<'py>,
    schema: &'py Schema,
    pos: usize,
    events: &'a [DirectEvent<'a>],
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
        events: &'a [DirectEvent<'a>],
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
                    // A key resolves like any other scalar. Leaving it as raw text
                    // made one text mean two things depending on which side of the
                    // `:` it sat (`1: a` -> `{'1': 'a'}` while `a: 1` -> `{'a': 1}`,
                    // `~: 1` -> `{'~': 1}` while `a: ~` -> `{'a': None}`), and put the
                    // object view out of step with PyYAML and ruamel on both.
                    let plain = matches!(style, granit_parser::ScalarStyle::Plain);
                    let key_object = self.scalar_to_py(key_text, plain).ok()?;
                    self.pos += 1;
                    let value = self.build(depth + 1)?;
                    if !self.allow_duplicate_keys
                        && !self.is_null_key_text(key_text)
                        && dict.contains(&key_object).unwrap_or(false)
                        && self.duplicate_key.is_none()
                    {
                        self.duplicate_key = Some(key_text.to_string());
                    }
                    // Last-wins at the original position = IndexMap::insert. An
                    // unhashable conversion bails to the AST pipeline, which keeps
                    // the pair under its source text rather than dropping it.
                    if dict.set_item(&key_object, &value).is_err() {
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
    // The engine decides eligibility - the two cheap byte/line pre-vetoes and the event
    // layer's own answer - and hands back the collected events when the document is
    // inside the surface. Its reason for refusing is not ours to report: the AST path
    // owns the authoritative diagnostic.
    let Some(events) = replicable_events(yaml_src) else {
        return DirectOutcome::Bail;
    };
    let mut builder = Builder::new(py, schema, &events, max_depth, allow_duplicate_keys);
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
