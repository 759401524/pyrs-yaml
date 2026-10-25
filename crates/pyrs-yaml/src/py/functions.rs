//! Module-level Python-facing functions for `pyrs_yaml`.

use std::ops::ControlFlow;

use pyo3::prelude::*;

use crate::parser::EventSink;
use crate::py::convert::{
    directive_to_emit, format_i18n_error, negotiate_schema, node_to_pyobject_simple, parse_schema,
};
use crate::py::direct_dump::direct_dump;
use crate::py::document::{YamlDocument, parse_document, resolve_tags};
use crate::py::parse_error_to_py_err;
use crate::py::stream_events::stream_event_to_py_dict;
use crate::py::stream_iterator::StreamIterator;
use crate::py::tag_registry;
use crate::py::type_registry;

use crate::YamlParseError;
use crate::YamlSerializeError;
use crate::YamlTypeError;

#[pyfunction]
#[pyo3(signature = (yaml: "str | bytes", resolve_merges: "bool" = true, schema: "str | None" = None, max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false, honor_version_directive: "bool" = false) -> "YamlDocument")]
/// Parse a YAML string (str or bytes) and return an editable `YamlDocument`.
///
/// With `honor_version_directive=True` a document that declares `%YAML 1.1` is
/// resolved under the YAML 1.1 profile and re-emits the directive. An explicit
/// `schema` always wins over the declaration.
pub(crate) fn parse(
    py: Python,
    yaml: &Bound<'_, PyAny>,
    resolve_merges: bool,
    schema: Option<&str>,
    max_depth: usize,
    allow_duplicate_keys: bool,
    honor_version_directive: bool,
) -> PyResult<YamlDocument> {
    parse_document(
        py,
        yaml,
        resolve_merges,
        schema,
        max_depth,
        allow_duplicate_keys,
        honor_version_directive,
    )
}

#[pyfunction]
#[pyo3(signature = (path: "str", schema: "str | None" = None, max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false, honor_version_directive: "bool" = false) -> "YamlDocument")]
/// Parse a YAML file and return an editable `YamlDocument`.
pub(crate) fn parse_file(
    py: Python,
    path: &str,
    schema: Option<&str>,
    max_depth: usize,
    allow_duplicate_keys: bool,
    honor_version_directive: bool,
) -> PyResult<YamlDocument> {
    let explicit = schema.map(parse_schema).transpose()?;
    let parse_schema_value = explicit
        .clone()
        .unwrap_or(crate::parser::yaml::Schema::Core);
    // With the `mmap` feature this borrows a read-only map of a regular file
    // (no whole-file heap copy on the read side); otherwise it is the same
    // `read_to_string`. `text` stays alive for the whole body, so `content` and
    // the `Arc<str>` source built from it are valid.
    let text = pyrs_yaml_core::file::read_file_text(path)
        .map_err(|e| crate::py::file_read_error(&e.to_string(), path))?;
    let content = text
        .as_str()
        .map_err(|e| crate::py::file_read_error(&e.to_string(), path))?;
    let parsed = py.detach(|| {
        crate::parser::parse_document(
            content,
            true,
            parse_schema_value,
            max_depth,
            allow_duplicate_keys,
        )
        .map_err(|e| parse_error_to_py_err(e, content, max_depth))
    })?;
    let declared = parsed.version.as_deref();
    let effective = negotiate_schema(explicit, declared, honor_version_directive);
    let emit_version = directive_to_emit(honor_version_directive, declared, &effective);
    let mut ast = parsed.node;
    resolve_tags(&mut ast, py)?;
    let source: std::sync::Arc<str> = std::sync::Arc::from(content);
    Ok(YamlDocument::new(
        ast,
        effective,
        source,
        parsed.version,
        emit_version,
        honor_version_directive,
    ))
}

#[pyfunction]
#[pyo3(signature = (yaml: "str", resolve_merges: "bool" = true, schema: "str | None" = None, max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false, honor_version_directive: "bool" = false) -> "list[YamlDocument]")]
/// Parse a multi-document YAML stream and return all `YamlDocument` objects.
pub(crate) fn parse_all_docs(
    py: Python,
    yaml: &str,
    resolve_merges: bool,
    schema: Option<&str>,
    max_depth: usize,
    allow_duplicate_keys: bool,
    honor_version_directive: bool,
) -> PyResult<Vec<YamlDocument>> {
    let explicit = schema.map(parse_schema).transpose()?;
    let parse_schema_value = explicit
        .clone()
        .unwrap_or(crate::parser::yaml::Schema::Core);
    let docs = py.detach(|| {
        crate::parser::parse_all_documents(
            yaml,
            resolve_merges,
            parse_schema_value,
            max_depth,
            allow_duplicate_keys,
        )
        .map_err(|e| parse_error_to_py_err(e, yaml, max_depth))
    })?;
    let source: std::sync::Arc<str> = std::sync::Arc::from(yaml);
    Ok(docs
        .into_iter()
        .map(|doc| {
            let declared = doc.version.as_deref();
            let effective = negotiate_schema(explicit.clone(), declared, honor_version_directive);
            let emit_version = directive_to_emit(honor_version_directive, declared, &effective);
            YamlDocument::new(
                doc.node,
                effective,
                source.clone(),
                doc.version,
                emit_version,
                honor_version_directive,
            )
        })
        .collect())
}

#[pyfunction]
#[pyo3(signature = (yaml: "str", schema: "str | None" = None, max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false, honor_version_directive: "bool" = false) -> "dict[Any, Any] | list[Any]")]
/// Parse YAML into a Python dict/list, resolving anchors and merges.
///
/// `honor_version_directive=True` lets an explicit `%YAML 1.1` declaration pick
/// the resolution profile; an explicit `schema` still wins.
pub(crate) fn safe_load(
    py: Python,
    yaml: &str,
    schema: Option<&str>,
    max_depth: usize,
    allow_duplicate_keys: bool,
    honor_version_directive: bool,
) -> PyResult<Py<PyAny>> {
    let explicit = schema.map(parse_schema).transpose()?;
    let direct_schema = explicit
        .clone()
        .unwrap_or(crate::parser::yaml::Schema::Core);
    // P3 fast path: one-pass event → Python materialization for value-only
    // documents; anything outside its surface (anchors/tags/merges/…)
    // falls through to the AST pipeline unchanged. A declared `%YAML` version
    // is only known after parsing, so honouring directives uses the AST path.
    if !honor_version_directive {
        match crate::py::direct_load::try_direct_load(
            py,
            yaml,
            &direct_schema,
            max_depth,
            allow_duplicate_keys,
        ) {
            crate::py::direct_load::DirectOutcome::Done(v) => return Ok(v),
            crate::py::direct_load::DirectOutcome::Fail(e) => {
                return Err(parse_error_to_py_err(e, yaml, max_depth));
            }
            crate::py::direct_load::DirectOutcome::Bail => {}
        }
    }
    let parsed = py.detach(|| {
        crate::parser::parse_document(yaml, true, direct_schema, max_depth, allow_duplicate_keys)
            .map_err(|e| parse_error_to_py_err(e, yaml, max_depth))
    })?;
    let effective = negotiate_schema(explicit, parsed.version.as_deref(), honor_version_directive);
    let mut ast = parsed.node;
    resolve_tags(&mut ast, py)?;
    crate::py::convert::node_to_pyobject_resolving_anchors(
        &ast,
        py,
        &effective,
        yaml.bytes().any(|b| b == b'&'),
    )
}

/// Feature-gated seam for the instruction-count harness (`crates/pyrs-yaml/benches/ir_gate.rs`).
///
/// Why a seam at all: the AST-to-Python conversion is the layer every user actually reaches
/// (`safe_load`, `to_dict`) and the layer PR #292 changed, yet the reproducible instruction-count gate
/// could not see it - the harness lived entirely inside `pyrs-yaml-core` and never linked this crate.
/// That gap was the registered hole `perf-coverage:binding-layer`, and this seam is the artefact that
/// closed it on 2026-10-08; what stays unfloored is the per-format loaders' own Python construction.
///
/// It mirrors `safe_load`'s AST path *without* the P3 direct-load fast path on purpose: measuring the
/// shortcut would report a different quantity than the one anchor- and tag-bearing data actually pays
/// for, and a direct-load regression could hide behind an anchor-free fixture. Not a `#[pyfunction]`:
/// Rust-only, invisible to Python, and compiled out of every build that does not ask for it.
///
/// The error is returned, never swallowed. A fixture that stopped parsing would otherwise fall into an
/// early return, the harness would count a fraction of its own work, and a one-sided tolerance reads
/// that as an enormous speedup - the gate flags growth, so the dangerous direction is the silent
/// improvement. The harness exits non-zero on `Err`, and `ir_gate.py` refuses a non-zero harness.
#[cfg(feature = "ir-gate")]
#[doc(hidden)]
pub fn bench_to_python(yaml: &str) -> Result<usize, pyo3::PyErr> {
    pyo3::Python::attach(|py| {
        let schema = parse_schema("core")?;
        let mut ast = crate::parser::parse_with_options(yaml, true, schema.clone(), 1000, false)
            .map_err(|e| parse_error_to_py_err(e, yaml, 1000))?;
        resolve_tags(&mut ast, py)?;
        let value = crate::py::convert::node_to_pyobject_resolving_anchors(
            &ast,
            py,
            &schema,
            yaml.bytes().any(|b| b == b'&'),
        )?;
        // An opaque read that keeps the object alive past the call without adding allocator-visible
        // work; the harness counts one per iteration, so the conversion cannot be elided and cannot
        // silently stop happening.
        Ok(usize::from(!value.as_ptr().is_null()))
    })
}

#[pyfunction]
#[pyo3(signature = (yaml: "str", schema: "str | None" = None, max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false, honor_version_directive: "bool" = false) -> "list[dict[Any, Any] | list[Any]]")]
/// Parse a multi-document YAML stream into a list of dicts/lists.
///
/// Each document negotiates its own `%YAML` declaration when
/// `honor_version_directive=True`; an explicit `schema` wins for every one.
pub(crate) fn safe_loads(
    py: Python,
    yaml: &str,
    schema: Option<&str>,
    max_depth: usize,
    allow_duplicate_keys: bool,
    honor_version_directive: bool,
) -> PyResult<Vec<Py<PyAny>>> {
    let explicit = schema.map(parse_schema).transpose()?;
    let direct_schema = explicit
        .clone()
        .unwrap_or(crate::parser::yaml::Schema::Core);
    // Single-document streams take the direct path (multi-doc bails back to
    // the AST pipeline); empty input mirrors parse_all's empty list. A
    // declared version is only known after parsing, so honouring it skips the
    // direct path.
    if !honor_version_directive && !yaml.trim().is_empty() {
        match crate::py::direct_load::try_direct_load(
            py,
            yaml,
            &direct_schema,
            max_depth,
            allow_duplicate_keys,
        ) {
            crate::py::direct_load::DirectOutcome::Done(v) => return Ok(vec![v]),
            crate::py::direct_load::DirectOutcome::Fail(e) => {
                return Err(parse_error_to_py_err(e, yaml, max_depth));
            }
            crate::py::direct_load::DirectOutcome::Bail => {}
        }
    }
    let docs = py.detach(|| {
        crate::parser::parse_all_documents(
            yaml,
            true,
            direct_schema,
            max_depth,
            allow_duplicate_keys,
        )
        .map_err(|e| parse_error_to_py_err(e, yaml, max_depth))
    })?;
    let has_anchors = yaml.bytes().any(|b| b == b'&');
    docs.iter()
        .map(|doc| {
            let effective = negotiate_schema(
                explicit.clone(),
                doc.version.as_deref(),
                honor_version_directive,
            );
            crate::py::convert::node_to_pyobject_resolving_anchors(
                &doc.node,
                py,
                &effective,
                has_anchors,
            )
        })
        .collect()
}

/// The callback path's sink: one event in, one callback out, with the GIL re-attached for the call.
///
/// Ruling B asks for one event at a time, and a batch of them would delay the callback's `False`
/// until the batch filled — long enough for the producer to read input the consumer already said it
/// did not want. So the stop is observed at the event it answers, not at a delivery boundary.
struct CallbackSink<'cb> {
    callback: &'cb Py<PyAny>,
    error: Option<PyErr>,
}

impl<'cb> CallbackSink<'cb> {
    fn new(callback: &'cb Py<PyAny>) -> Self {
        Self {
            callback,
            error: None,
        }
    }

    /// Hand one event to Python. `Break` means the producer must stop: the callback answered
    /// `False`, or it raised — in which case the error is kept and outranks the producer's verdict.
    fn deliver(&mut self, event: &crate::parser::StreamEvent<'_>) -> ControlFlow<()> {
        let answer = {
            let callback = self.callback;
            Python::attach(move |py| -> PyResult<bool> {
                let py_event = stream_event_to_py_dict(py, event)?;
                let should_continue: bool = callback.bind(py).call1((py_event,))?.extract()?;
                Ok(should_continue)
            })
        };
        match answer {
            Ok(true) => ControlFlow::Continue(()),
            Ok(false) => ControlFlow::Break(()),
            Err(error) => {
                self.error = Some(error);
                ControlFlow::Break(())
            }
        }
    }
}

impl<'src, 'cb> EventSink<'src> for CallbackSink<'cb> {
    fn event(&mut self, event: crate::parser::StreamEvent<'src>) -> ControlFlow<()> {
        self.deliver(&event)
    }
}

fn stream_parse_error(error: crate::parser::ParseError, key: &str) -> PyErr {
    let detail = error.to_string();
    YamlParseError::new_err(format_i18n_error(key, &[("detail", &detail)]))
}

/// Hand over a stream that was collected before delivery: YAML's push receiver cannot abort, and
/// the no-callback iterator form outlives the call.
fn finish_stream<'src>(
    py: Python<'_>,
    events: Vec<crate::parser::StreamEvent<'src>>,
    on_event: Option<Py<PyAny>>,
) -> PyResult<Py<PyAny>> {
    if let Some(callback) = on_event {
        let cb = callback.bind(py);
        for event in &events {
            let py_event = stream_event_to_py_dict(py, event)?;
            let should_continue: bool = cb.call1((py_event,))?.extract()?;
            if !should_continue {
                break;
            }
        }
        Ok(py.None())
    } else {
        // The iterator outlives this call and a `#[pyclass]` cannot borrow, so the stream detaches
        // here — the one path that still copies every scalar out of the source.
        let events: Vec<crate::parser::StreamEvent<'static>> =
            events.into_iter().map(|event| event.into_owned()).collect();

        let iter = StreamIterator { events, index: 0 };
        Ok(iter.into_pyobject(py)?.into_any().unbind())
    }
}

#[pyfunction]
#[pyo3(signature = (yaml: "str | bytes", on_event: "Callable[[dict[str, Any]], bool] | None" = None, max_depth: "int" = 1000, *, format: "str" = "yaml") -> "StreamIterator | None")]
/// Event-stream parsing. With `on_event` callback, consumes events and returns `None`. Otherwise returns a lazy `StreamIterator`.
///
/// `format` chooses the producer: `yaml` (the default), `json`, `jsonc`, `json5` or `toml`. Every
/// producer reports the one vocabulary, so the dicts keep the same keys and only the fields a
/// format has something to say about change. `style` is whatever that engine remembers: a JSON
/// string reads `double_quoted` and a JSON number `plain`, JSON5 adds `single_quoted`, and a TOML
/// literal `'…'` reads the same as a basic `"…"` because which quote was typed is not in its IR.
/// `anchor` and `alias` stay YAML's own and never appear elsewhere. A `comment` event carries two
/// keys about the note: `style` says whether the source gave the note a line of its own, and
/// `owner` says which node the note is filed under and in which of its two slots — the node the
/// consumer builds next (`following_leading`, `following_inline`) or the one it built last
/// (`preceding_leading`, `preceding_inline`). `max_depth` bounds the YAML and
/// JSON readers, which carry that knob; TOML's nesting limit is the intermediate representation's
/// own. The accept/reject verdict is the same one the tree route reaches with the same detail,
/// because it is the same scanner: a document `load_json` refuses raises here too.
///
/// With `on_event` the events arrive as they are produced and returning `False` stops the producer,
/// not merely the delivery loop. What can stop is the format's own fact: `json`/`jsonc`/`json5`
/// stop scanning before the bytes of the next event, so a syntax error past that point is never
/// reached; `toml` stops emitting from the intermediate representation its scan already built, so a
/// refused document still raises — before any event is delivered. YAML's text producer is a push
/// adapter over a scanner that cannot be aborted, so there `False` stops delivery after the whole
/// document was parsed; `load_stream` is YAML's incremental surface. Without `on_event` the
/// iterator materialises the document first.
pub(crate) fn parse_stream(
    py: Python,
    yaml: &Bound<'_, PyAny>,
    on_event: Option<Py<PyAny>>,
    max_depth: usize,
    format: &str,
) -> PyResult<Py<PyAny>> {
    let yaml_str: String = crate::py::document::coerce_str_or_bytes(yaml)?;

    match format {
        "yaml" => {
            let events = py.detach(|| {
                crate::parser::parse_stream_with_options(&yaml_str, max_depth)
                    .map_err(|e| parse_error_to_py_err(e, &yaml_str, max_depth))
            })?;
            finish_stream(py, events, on_event)
        }
        "json" | "jsonc" | "json5" => {
            let dialect = match format {
                "json" => pyrs_json::JsonParseOptions::STRICT,
                "jsonc" => pyrs_json::JsonParseOptions::JSONC,
                _ => pyrs_json::JsonParseOptions::JSON5,
            };
            let opts = pyrs_json::JsonParseOptions {
                max_depth,
                ..dialect
            };
            match on_event {
                Some(callback) => {
                    let mut sink = CallbackSink::new(&callback);
                    let parsed = py.detach(|| {
                        pyrs_json::parse_stream_with_options_to(&yaml_str, opts, &mut sink)
                    });
                    if let Some(error) = sink.error.take() {
                        return Err(error);
                    }
                    parsed.map_err(|e| stream_parse_error(e, "json-parse-error"))?;
                    Ok(py.None())
                }
                None => {
                    let events = py
                        .detach(|| pyrs_json::parse_stream_with_options(&yaml_str, opts))
                        .map_err(|e| stream_parse_error(e, "json-parse-error"))?;
                    finish_stream(py, events, None)
                }
            }
        }
        "toml" => match on_event {
            Some(callback) => {
                let mut sink = CallbackSink::new(&callback);
                let parsed = py.detach(|| pyrs_toml::parse_stream_to(&yaml_str, &mut sink));
                if let Some(error) = sink.error.take() {
                    return Err(error);
                }
                parsed.map_err(|e| stream_parse_error(e, "toml-parse-error"))?;
                Ok(py.None())
            }
            None => {
                let events = py
                    .detach(|| pyrs_toml::parse_stream(&yaml_str))
                    .map_err(|e| stream_parse_error(e, "toml-parse-error"))?;
                finish_stream(py, events, None)
            }
        },
        other => Err(YamlTypeError::new_err(format!(
            "Unsupported format '{}'. Supported: yaml, json, jsonc, json5, toml",
            other
        ))),
    }
}

#[pyfunction]
#[pyo3(signature = (data: "dict[str, Any] | list[Any]") -> "str")]
/// Serialize a Python dict/list to a YAML string.
pub(crate) fn safe_dump(py: Python, data: Py<PyAny>) -> PyResult<String> {
    direct_dump(py, &data)
}

#[pyfunction]
#[pyo3(signature = (data: "dict[str, Any] | list[Any]") -> "str")]
/// Convert a Python dict/list to a YAML string (auto-selects block/flow style).
pub(crate) fn from_dict(py: Python, data: Py<PyAny>) -> PyResult<String> {
    direct_dump(py, &data)
}

#[pyfunction]
#[pyo3(signature = (json_str: "str") -> "str")]
/// Convert a JSON string to a YAML string.
pub(crate) fn from_json(_py: Python, json_str: &str) -> PyResult<String> {
    let node = pyrs_json::from_json(json_str).map_err(|e| {
        YamlParseError::new_err(format_i18n_error(
            "json-parse-error",
            &[("detail", &e.to_string())],
        ))
    })?;
    Ok(crate::serializer::to_yaml(&node))
}

#[pyfunction]
#[pyo3(signature = (json_str: "str") -> "str")]
/// Convert a JSONC string (JSON with `//` and `/* ... */` comments) to
/// a YAML string. Comments are NOT discarded on this projection: since
/// #112/#115 they ride the AST's `comment` / `leading_comment` slots (so
/// `to_jsonc` can reproduce them), and the YAML writer renders those slots
/// as `#` notes, which keeps the conversion lossless. Strictly a
/// `//`-to-`#` change of syntax, not a change of language: everything else
/// matches `from_json` semantics exactly.
pub(crate) fn from_jsonc(_py: Python, json_str: &str) -> PyResult<String> {
    let node = pyrs_json::from_jsonc(json_str).map_err(|e| {
        YamlParseError::new_err(format_i18n_error(
            "json-parse-error",
            &[("detail", &e.to_string())],
        ))
    })?;
    Ok(crate::serializer::to_yaml(&node))
}

#[pyfunction]
#[pyo3(signature = (json_str: "str") -> "dict[str, Any] | list[Any]")]
/// Parse a strict JSON document directly into Python values (dict / list /
/// scalar), mirroring the `load_jsonc` / `load_json5` / `load_toml` family.
/// Unlike `load_jsonc` this REJECTS the JSONC/JSON5 extensions — `//` and
/// `/* … */` comments, trailing commas, single-quoted strings, bare
/// `Infinity`/`NaN` and `0x…` forms all raise a parse error, exactly like
/// `json.loads` / `orjson.loads`. Canonical strict JSON takes the same
/// AST-free fast path `load_jsonc` uses (bytes → PyList/PyDict/scalars);
/// any non-canonical shape (floats with exotic spellings, `\u` escapes,
/// out-of-range ints) falls back to the strict AST parser that produces the
/// identical value or the proper `json-parse-error`.
pub(crate) fn load_json(py: Python, json_str: &str) -> PyResult<Py<PyAny>> {
    // Fast path is a strict-JSON-only subset (comments/escapes/trailing
    // commas make it bail), so routing through it never widens the accepted
    // grammar beyond what `from_json` itself allows.
    if let Some(v) = crate::py::json_fast::try_load(py, json_str) {
        return Ok(v);
    }
    let mut ast = pyrs_json::from_json(json_str).map_err(|e| {
        YamlParseError::new_err(format_i18n_error(
            "json-parse-error",
            &[("detail", &e.to_string())],
        ))
    })?;
    crate::py::document::resolve_tags(&mut ast, py)?;
    crate::py::convert::node_to_pyobject_resolving_anchors(&ast, py, &parse_schema("json")?, false)
}

#[pyfunction]
#[pyo3(signature = (json_str: "str") -> "dict[str, Any] | list[Any]")]
/// Parse a JSONC document directly into Python values (dict / list /
/// scalar). Handy for TypeScript `tsconfig.json`, VS Code
/// `settings.json`, and similar dialects without a pre-processing step.
pub(crate) fn load_jsonc(py: Python, json_str: &str) -> PyResult<Py<PyAny>> {
    // Fast path: canonical strict JSON (arrays/objects, i64 integers, booleans,
    // null, unescaped strings) builds Python objects directly, skipping the
    // `CustomNode` round-trip. Anything non-canonical (floats, escapes,
    // comments, trailing commas, out-of-range ints, bad grammar) bails and the
    // AST path below produces the identical value or the proper error.
    if let Some(v) = crate::py::json_fast::try_load(py, json_str) {
        return Ok(v);
    }
    let mut ast = pyrs_json::from_jsonc(json_str).map_err(|e| {
        YamlParseError::new_err(format_i18n_error(
            "json-parse-error",
            &[("detail", &e.to_string())],
        ))
    })?;
    crate::py::document::resolve_tags(&mut ast, py)?;
    crate::py::convert::node_to_pyobject_resolving_anchors(&ast, py, &parse_schema("json")?, false)
}

#[pyfunction]
#[pyo3(signature = (json_str: "str") -> "str")]
/// Convert a JSON5 string to a YAML string. JSON5 supersedes JSONC with
/// trailing commas, single-quoted strings, unquoted identifier keys and
/// the `0x…` / `.5` / `Infinity` / `NaN` numeric forms; like `from_jsonc`,
/// comments survive the projection as YAML notes rather than being dropped.
pub(crate) fn from_json5(_py: Python, json_str: &str) -> PyResult<String> {
    let node = pyrs_json::from_json5(json_str).map_err(|e| {
        YamlParseError::new_err(format_i18n_error(
            "json-parse-error",
            &[("detail", &e.to_string())],
        ))
    })?;
    Ok(crate::serializer::to_yaml(&node))
}

#[pyfunction]
#[pyo3(signature = (json_str: "str") -> "dict[str, Any] | list[Any]")]
/// Parse a JSON5 document directly into Python values (dict / list /
/// scalar). Accepts the full JSON5 grammar without a pre-processing
/// step, mirroring `load_jsonc` on the wider dialect. Canonical strict
/// JSON takes the same AST-free fast path `load_json` and `load_jsonc`
/// use; every JSON5-only form falls through to the AST parser that owns
/// the dialect's number and string semantics.
pub(crate) fn load_json5(py: Python, json_str: &str) -> PyResult<Py<PyAny>> {
    // The fast path accepts only the canonical strict-JSON subset, which is a
    // legal JSON5 document with the same resolved value, so routing through it
    // cannot widen or shift the grammar: single-quoted strings, unquoted keys,
    // `.5`, `0x…`, `+1`, `Infinity`/`NaN`, comments and trailing commas are all
    // outside what it accepts, and it bails on the whole document — never
    // partway — leaving the path below to resolve them.
    if let Some(value) = crate::py::json_fast::try_load(py, json_str) {
        return Ok(value);
    }
    let mut ast = pyrs_json::from_json5(json_str).map_err(|e| {
        YamlParseError::new_err(format_i18n_error(
            "json-parse-error",
            &[("detail", &e.to_string())],
        ))
    })?;
    crate::py::document::resolve_tags(&mut ast, py)?;
    // JSON5 numeric grammar (hex, leading `+`, `Infinity`/`NaN`) resolves
    // to real numbers under the Json5 schema; strict JSON/JSONC keep the
    // stricter Json resolver that treats those forms as strings.
    crate::py::convert::node_to_pyobject_resolving_anchors(&ast, py, &parse_schema("json5")?, false)
}

#[pyfunction]
#[pyo3(signature = (toml_str: "str") -> "str")]
/// Convert a TOML string to a YAML string (hub-and-spoke exchange).
/// TOML strings keep quoting so values never re-resolve; datetimes gain
/// the `!timestamp` tag consumed by the built-in plugin.
pub(crate) fn from_toml(toml_str: &str) -> PyResult<String> {
    let node = pyrs_toml::from_toml(toml_str).map_err(|e| {
        YamlParseError::new_err(format_i18n_error(
            "toml-parse-error",
            &[("detail", &e.to_string())],
        ))
    })?;
    Ok(crate::serializer::to_yaml(&node))
}

#[pyfunction]
#[pyo3(signature = (yaml: "str", schema: "str" = "core") -> "str")]
/// Render a YAML document as TOML text. Rejects shapes TOML cannot hold
/// (non-table root, null values, aliases, non-scalar keys) with stable
/// `toml-serialize-error` messages.
pub(crate) fn to_toml(py: Python, yaml: &str, schema: &str) -> PyResult<String> {
    let schema_enum = parse_schema(schema)?;
    let schema_clone = schema_enum.clone();
    let node = py.detach(|| {
        crate::parser::parse_with_options(yaml, true, schema_clone, 1000, false)
            .map_err(|e| parse_error_to_py_err(e, yaml, 1000))
    })?;
    pyrs_toml::to_toml(&node).map_err(|e| {
        YamlSerializeError::new_err(format_i18n_error(
            "toml-serialize-error",
            &[("detail", &e.to_string())],
        ))
    })
}

#[pyfunction]
#[pyo3(signature = (toml_str: "str") -> "dict[str, Any]")]
/// Parse TOML directly into a Python dict (values, not a document).
/// Anchors cannot occur in TOML, so no alias resolution pass is needed.
pub(crate) fn load_toml(py: Python, toml_str: &str) -> PyResult<Py<PyAny>> {
    let schema = parse_schema("core")?;
    // A registered tag handler rewrites a tagged scalar's text on the node it is filed under, and
    // only the AST route has that node, so a reader with handlers installed takes that route.
    if crate::py::tag_registry::is_empty() {
        match crate::py::toml_direct::read_values(py, toml_str, &schema) {
            Ok(values) => return Ok(values),
            Err(pyrs_toml::TomlReadError::Parse(e)) => {
                return Err(YamlParseError::new_err(format_i18n_error(
                    "toml-parse-error",
                    &[("detail", &e.to_string())],
                )));
            }
            Err(pyrs_toml::TomlReadError::Sink(e)) => return Err(e),
        }
    }
    let mut ast = pyrs_toml::from_toml(toml_str).map_err(|e| {
        YamlParseError::new_err(format_i18n_error(
            "toml-parse-error",
            &[("detail", &e.to_string())],
        ))
    })?;
    crate::py::document::resolve_tags(&mut ast, py)?;
    crate::py::convert::node_to_pyobject_resolving_anchors(&ast, py, &schema, false)
}

#[pyfunction]
#[pyo3(signature = (data: "Any", path: "str") -> "None")]
/// Serialize a Python object to YAML and write to a file.
pub(crate) fn dump_file(py: Python, data: Py<PyAny>, path: &str) -> PyResult<()> {
    let yaml = direct_dump(py, &data)?;
    std::fs::write(path, yaml).map_err(|e| {
        pyo3::exceptions::PyIOError::new_err(format_i18n_error(
            "file-write-error",
            &[("detail", &e.to_string()), ("path", path)],
        ))
    })?;
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (path: "str", schema: "str" = "core", max_depth: "int" = 1000) -> "tuple[dict[Any, Any] | None, str]")]
/// Read a Markdown file and extract YAML front matter, returning `(frontmatter, body)`.
pub(crate) fn read_markdown(
    py: Python,
    path: &str,
    schema: &str,
    max_depth: usize,
) -> PyResult<(Option<Py<PyAny>>, String)> {
    let content = py.detach(|| crate::py::read_file_to_string(path))?;
    read_markdown_str(py, &content, schema, max_depth)
}

#[pyfunction]
#[pyo3(signature = (content: "str", schema: "str" = "core", max_depth: "int" = 1000) -> "tuple[dict[Any, Any] | None, str]")]
/// Extract YAML front matter from a Markdown string, returning `(frontmatter, body)`.
pub(crate) fn read_markdown_str(
    _py: Python,
    content: &str,
    schema: &str,
    max_depth: usize,
) -> PyResult<(Option<Py<PyAny>>, String)> {
    let content = content.trim_start();
    let schema_enum = parse_schema(schema)?;

    if let Some(rest) = content.strip_prefix("---")
        && let Some(end_idx) = rest.find("---")
    {
        let frontmatter = rest[..end_idx].trim();
        let markdown_content = rest[end_idx + 3..].trim();

        if !frontmatter.is_empty() {
            return Python::attach(|py| {
                let ast = crate::parser::parse_with_options(
                    frontmatter,
                    true,
                    schema_enum.clone(),
                    max_depth,
                    false,
                )
                .map_err(|e| parse_error_to_py_err(e, frontmatter, max_depth))?;
                Ok((
                    Some(node_to_pyobject_simple(&ast, py, &schema_enum)?),
                    markdown_content.to_string(),
                ))
            });
        }
    }

    Ok((None, content.to_string()))
}

#[pyfunction]
#[pyo3(signature = (lang: "str") -> "None")]
/// Set the error message language.
pub(crate) fn set_language(lang: &str) -> PyResult<()> {
    crate::i18n::set_language(lang).map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(format_i18n_error(
            "unsupported-language",
            &[
                ("lang", lang),
                (
                    "supported",
                    &format!("{:?}", crate::i18n::SUPPORTED_LANGUAGES),
                ),
            ],
        ))
    })
}

#[pyfunction]
#[pyo3(signature = () -> "str")]
/// Return the current error message language.
pub(crate) fn get_language() -> &'static str {
    crate::i18n::get_language_static()
}

#[pyfunction]
#[pyo3(signature = () -> "list[str]")]
/// List supported language codes.
pub(crate) fn list_languages() -> Vec<&'static str> {
    crate::i18n::list_languages()
}

#[pyfunction]
#[pyo3(signature = () -> "str")]
/// Detect the system default language.
pub(crate) fn detect_language() -> String {
    crate::i18n::detect_language()
}

#[pyfunction]
#[pyo3(signature = (user_locales: "list[str]", default: "str" = "en") -> "str")]
/// Negotiate a language from user locale list and default.
pub(crate) fn negotiate_language(
    user_locales: &Bound<'_, PyAny>,
    default: &str,
) -> PyResult<String> {
    let locales: Vec<String> = user_locales.extract()?;
    let refs: Vec<&str> = locales.iter().map(|s| s.as_str()).collect();
    Ok(crate::i18n::negotiate_language(&refs, default).to_string())
}

#[pyfunction]
#[pyo3(signature = (name: "str", handler: "Py<PyAny>", priority: "u32" = 0))]
/// Register a custom tag handler.
pub(crate) fn register_tag(name: &str, handler: Py<PyAny>, priority: u32) {
    tag_registry::register(name, handler, priority);
}

#[pyfunction]
/// Clear all tag handlers.
pub(crate) fn clear_tag_handlers() {
    tag_registry::clear_all();
}

#[pyfunction]
#[pyo3(signature = (name: "str"))]
/// Remove a specific tag handler.
pub(crate) fn remove_tag(name: &str) {
    tag_registry::remove(name);
}

#[pyfunction]
#[pyo3(signature = (name: "str", handler: "Py<PyAny>"))]
/// Register a custom type handler (Community Plugins).
pub(crate) fn register_type(name: &str, handler: Py<PyAny>) {
    type_registry::register(name, handler);
}

#[pyfunction]
/// Clear all custom type handlers.
pub(crate) fn clear_type_handlers() {
    type_registry::clear_all();
}

#[pyfunction]
#[pyo3(signature = (name: "str"))]
/// Remove a specific custom type handler.
pub(crate) fn remove_type(name: &str) {
    type_registry::remove(name);
}

#[pyfunction]
#[pyo3(signature = (obj: "Py<PyAny>"))]
/// Validate a Python object against all registered CustomType validators.
///
/// Recursively walks dicts and lists. For each value that matches a
/// registered type's `python_type`, calls the handler's `validate` method.
/// Raises `ValueError` if any value fails validation.
pub(crate) fn validate_custom_types(py: Python, obj: Py<PyAny>) -> PyResult<()> {
    type_registry::validate_custom_types(py, &obj)
}

#[pyfunction]
#[pyo3(signature = (name: "str", schema_yaml: "str"))]
/// Register a YAML Schema Language schema under a name.
///
/// `schema_yaml` is a schema definition in YAML format:
/// ```yaml
/// name: myapp
/// extends: core
/// rules:
///   - pattern: "^0x[0-9a-fA-F]+$"
///     type: int
/// ```
/// Once registered, the schema can be used as `YAML(schema="myapp")`.
pub(crate) fn register_schema(name: &str, schema_yaml: &str) -> PyResult<()> {
    let resolver = crate::parser::yaml::schema_language::parse_schema_yaml(schema_yaml)
        .map_err(|e| YamlParseError::new_err(format!("Schema parse error: {}", e)))?;
    let resolver = std::sync::Arc::new(resolver);
    crate::parser::yaml::registry::register_boxed(
        name,
        resolver as std::sync::Arc<dyn crate::parser::yaml::types::SchemaResolver>,
    );
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (name: "str", path: "str") -> "None")]
/// Register a YAML Schema Language schema from a file.
///
/// Reads the schema definition from `path` (a YAML file with `name`/`extends`/`rules`
/// structure) and registers it under `name`. Equivalent to calling
/// `register_schema(name, open(path).read())` but handles file I/O in Rust.
pub(crate) fn load_schema(name: &str, path: &str) -> PyResult<()> {
    let schema_yaml = std::fs::read_to_string(path).map_err(|e| {
        YamlParseError::new_err(format!("failed to read schema file '{}': {}", path, e))
    })?;
    register_schema(name, &schema_yaml)
}

#[pyfunction]
#[pyo3(signature = () -> "list[str]")]
/// List all registered schema names (built-in + custom).
///
/// Returns the four built-in schemas (`failsafe`, `json`, `core`, `yaml1.1`)
/// plus any schemas registered via `register_schema()` / `load_schema()`.
pub(crate) fn list_schemas() -> Vec<String> {
    crate::parser::yaml::registry::names()
}

#[pyfunction]
#[pyo3(signature = () -> "list[tuple[str, str]]")]
/// List all registered custom type plugins with their metadata.
///
/// Returns a list of ``(tag, python_type)`` tuples for every registered
/// `CustomType` handler.
pub(crate) fn list_plugins(py: Python<'_>) -> Vec<(String, String)> {
    crate::py::type_registry::list(py)
}

#[pyfunction]
#[pyo3(signature = (tag: "str") -> "tuple[str, str] | None")]
/// Get metadata for a registered plugin by tag.
///
/// Returns ``(tag, python_type)`` or ``None`` if the tag is not registered.
pub(crate) fn get_plugin(py: Python<'_>, tag: &str) -> Option<(String, String)> {
    crate::py::type_registry::get_plugin(py, tag)
}
