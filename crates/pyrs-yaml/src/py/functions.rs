//! Module-level Python-facing functions for `pyrs_yaml`.

use pyo3::prelude::*;

use crate::YamlValidateError;
use crate::py::convert::{format_i18n_error, node_to_pyobject_simple, parse_schema};
use crate::py::direct_dump::direct_dump;
use crate::py::document::{YamlDocument, parse_document, resolve_tags};
use crate::py::parse_error_to_py_err;
use crate::py::stream_events::stream_event_to_py_dict;
use crate::py::stream_iterator::StreamIterator;
use crate::py::tag_registry;
use crate::py::type_registry;

use crate::YamlParseError;
use crate::YamlSerializeError;

#[pyfunction]
#[pyo3(signature = (yaml: "str | bytes", resolve_merges: "bool" = true, schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false) -> "YamlDocument")]
/// Parse a YAML string (str or bytes) and return an editable `YamlDocument`.
pub(crate) fn parse(
    py: Python,
    yaml: &Bound<'_, PyAny>,
    resolve_merges: bool,
    schema: &str,
    max_depth: usize,
    allow_duplicate_keys: bool,
) -> PyResult<YamlDocument> {
    parse_document(
        py,
        yaml,
        resolve_merges,
        schema,
        max_depth,
        allow_duplicate_keys,
    )
}

#[pyfunction]
#[pyo3(signature = (path: "str", schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false) -> "YamlDocument")]
/// Parse a YAML file and return an editable `YamlDocument`.
pub(crate) fn parse_file(
    py: Python,
    path: &str,
    schema: &str,
    max_depth: usize,
    allow_duplicate_keys: bool,
) -> PyResult<YamlDocument> {
    let schema_enum = parse_schema(schema)?;
    let schema_clone = schema_enum.clone();
    let content = crate::py::read_file_to_string(path)?;
    let mut ast = py.detach(|| {
        crate::parser::parse_with_options(
            &content,
            true,
            schema_clone,
            max_depth,
            allow_duplicate_keys,
        )
        .map_err(|e| parse_error_to_py_err(e, &content, max_depth))
    })?;
    resolve_tags(&mut ast, py)?;
    let source: std::sync::Arc<str> = std::sync::Arc::from(content);
    Ok(YamlDocument::new(ast, schema_enum, source))
}

#[pyfunction]
#[pyo3(signature = (yaml: "str", resolve_merges: "bool" = true, schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false) -> "list[YamlDocument]")]
/// Parse a multi-document YAML stream and return all `YamlDocument` objects.
pub(crate) fn parse_all_docs(
    py: Python,
    yaml: &str,
    resolve_merges: bool,
    schema: &str,
    max_depth: usize,
    allow_duplicate_keys: bool,
) -> PyResult<Vec<YamlDocument>> {
    let schema_enum = parse_schema(schema)?;
    let schema_clone = schema_enum.clone();
    let asts = py.detach(|| {
        crate::parser::parse_all_with_options(
            yaml,
            resolve_merges,
            schema_clone,
            max_depth,
            allow_duplicate_keys,
        )
        .map_err(|e| parse_error_to_py_err(e, yaml, max_depth))
    })?;
    let source: std::sync::Arc<str> = std::sync::Arc::from(yaml);
    Ok(asts
        .into_iter()
        .map(|ast| YamlDocument::new(ast, schema_enum.clone(), source.clone()))
        .collect())
}

#[pyfunction]
#[pyo3(signature = (yaml: "str", schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false) -> "dict[str, Any] | list[Any]")]
/// Parse YAML into a Python dict/list, resolving anchors and merges.
pub(crate) fn safe_load(
    py: Python,
    yaml: &str,
    schema: &str,
    max_depth: usize,
    allow_duplicate_keys: bool,
) -> PyResult<Py<PyAny>> {
    let schema_enum = parse_schema(schema)?;
    // P3 fast path: one-pass event → Python materialization for value-only
    // documents; anything outside its surface (anchors/tags/merges/…)
    // falls through to the AST pipeline unchanged.
    match crate::py::direct_load::try_direct_load(
        py,
        yaml,
        &schema_enum,
        max_depth,
        allow_duplicate_keys,
    ) {
        crate::py::direct_load::DirectOutcome::Done(v) => return Ok(v),
        crate::py::direct_load::DirectOutcome::Fail(e) => {
            return Err(parse_error_to_py_err(e, yaml, max_depth));
        }
        crate::py::direct_load::DirectOutcome::Bail => {}
    }
    let schema_clone = schema_enum.clone();
    let mut ast = py.detach(|| {
        crate::parser::parse_with_options(yaml, true, schema_clone, max_depth, allow_duplicate_keys)
            .map_err(|e| parse_error_to_py_err(e, yaml, max_depth))
    })?;
    resolve_tags(&mut ast, py)?;
    crate::py::convert::node_to_pyobject_resolving_anchors(
        &ast,
        py,
        &schema_enum,
        yaml.bytes().any(|b| b == b'&'),
    )
}

/// Feature-gated seam for the instruction-count harness (`crates/pyrs-yaml/benches/ir_gate.rs`).
///
/// Why a seam at all: the AST-to-Python conversion is the layer every user actually reaches
/// (`safe_load`, `to_dict`) and the layer PR #292 changed, yet the reproducible instruction-count gate
/// could not see it - the harness lived entirely inside `pyrs-yaml-core` and never linked this crate.
/// That gap is the registered hole `perf-coverage:binding-layer`.
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
#[pyo3(signature = (yaml: "str", schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false) -> "list[dict[str, Any] | list[Any]]")]
/// Parse a multi-document YAML stream into a list of dicts/lists.
pub(crate) fn safe_loads(
    py: Python,
    yaml: &str,
    schema: &str,
    max_depth: usize,
    allow_duplicate_keys: bool,
) -> PyResult<Vec<Py<PyAny>>> {
    let schema_enum = parse_schema(schema)?;
    // Single-document streams take the direct path (multi-doc bails back to
    // the AST pipeline); empty input mirrors parse_all's empty list.
    if !yaml.trim().is_empty() {
        match crate::py::direct_load::try_direct_load(
            py,
            yaml,
            &schema_enum,
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
    let schema_clone = schema_enum.clone();
    let asts = py.detach(|| {
        crate::parser::parse_all_with_options(
            yaml,
            true,
            schema_clone,
            max_depth,
            allow_duplicate_keys,
        )
        .map_err(|e| parse_error_to_py_err(e, yaml, max_depth))
    })?;
    let has_anchors = yaml.bytes().any(|b| b == b'&');
    asts.iter()
        .map(|ast| {
            crate::py::convert::node_to_pyobject_resolving_anchors(
                ast,
                py,
                &schema_enum,
                has_anchors,
            )
        })
        .collect()
}

#[pyfunction]
#[pyo3(signature = (yaml: "str | bytes", on_event: "Callable[[dict[str, Any]], bool] | None" = None, max_depth: "int" = 1000) -> "StreamIterator | None")]
/// Event-stream parsing. With `on_event` callback, consumes events and returns `None`. Otherwise returns a lazy `StreamIterator`.
pub(crate) fn parse_stream(
    py: Python,
    yaml: &Bound<'_, PyAny>,
    on_event: Option<Py<PyAny>>,
    max_depth: usize,
) -> PyResult<Py<PyAny>> {
    let yaml_str: String = crate::py::document::coerce_str_or_bytes(yaml)?;

    if let Some(callback) = on_event {
        let events = py.detach(|| {
            crate::parser::parse_stream_with_options(&yaml_str, max_depth)
                .map_err(|e| parse_error_to_py_err(e, &yaml_str, max_depth))
        })?;

        Python::attach(|py| -> PyResult<()> {
            let cb = callback.bind(py);
            for event in &events {
                let py_event = stream_event_to_py_dict(py, event)?;
                let should_continue: bool = cb.call1((py_event,))?.extract()?;
                if !should_continue {
                    break;
                }
            }
            Ok(())
        })?;
        Ok(py.None())
    } else {
        let events = py.detach(|| {
            crate::parser::parse_stream_with_options(&yaml_str, max_depth)
                .map_err(|e| parse_error_to_py_err(e, &yaml_str, max_depth))
        })?;

        let iter = StreamIterator { events, index: 0 };
        Ok(iter.into_pyobject(py)?.into_any().unbind())
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
/// step, mirroring `load_jsonc` on the wider dialect.
pub(crate) fn load_json5(py: Python, json_str: &str) -> PyResult<Py<PyAny>> {
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
    let mut ast = pyrs_toml::from_toml(toml_str).map_err(|e| {
        YamlParseError::new_err(format_i18n_error(
            "toml-parse-error",
            &[("detail", &e.to_string())],
        ))
    })?;
    crate::py::document::resolve_tags(&mut ast, py)?;
    crate::py::convert::node_to_pyobject_resolving_anchors(&ast, py, &parse_schema("core")?, false)
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
#[pyo3(signature = (path: "str", schema: "str" = "core", max_depth: "int" = 1000) -> "tuple[dict[str, Any] | None, str]")]
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
#[pyo3(signature = (content: "str", schema: "str" = "core", max_depth: "int" = 1000) -> "tuple[dict[str, Any] | None, str]")]
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
        resolver.clone() as std::sync::Arc<dyn crate::parser::yaml::types::SchemaResolver>,
    );
    crate::parser::yaml::registry::register_rule_resolver(
        name,
        resolver as std::sync::Arc<dyn std::any::Any + Send + Sync>,
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

#[pyfunction]
#[pyo3(signature = (data: "str", schema_yaml: "str") -> "None")]
/// Validate a YAML document against a schema definition's `validate` rules.
///
/// `data` is a YAML string; `schema_yaml` is a schema definition (the same
/// format passed to `register_schema`). Raises `YamlValidateError` listing
/// each structural validation failure (path + reason) when the document does
/// not conform to the schema's `validate` section.
pub(crate) fn validate_against_schema(data: &str, schema_yaml: &str) -> PyResult<()> {
    use pyrs_yaml_core::parser::yaml::Schema;
    use pyrs_yaml_core::parser::yaml::schema_language::{parse_schema_yaml, validate_node};

    let ast = pyrs_yaml_core::parser::parse(data, Schema::Core)
        .map_err(|e| YamlParseError::new_err(format!("failed to parse data: {}", e)))?;
    let resolver = parse_schema_yaml(schema_yaml)
        .map_err(|e| YamlParseError::new_err(format!("Schema parse error: {}", e)))?;
    match validate_node(&ast, &resolver, data) {
        Ok(()) => Ok(()),
        Err(errors) => {
            let msg = errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            Err(YamlValidateError::new_err(msg))
        }
    }
}

#[pyfunction]
#[pyo3(signature = (data: "str", name: "str") -> "None")]
/// Validate a YAML document against a **registered** schema by name.
///
/// `data` is a YAML string; `name` is the name of a previously registered
/// schema (via `register_schema` / `load_schema`). Raises `YamlValidateError`
/// when the document does not conform to the schema's structural `validate`
/// section.
pub(crate) fn validate_against_registered_schema(data: &str, name: &str) -> PyResult<()> {
    use pyrs_yaml_core::parser::yaml::schema_language::{RuleResolver, validate_node};
    use pyrs_yaml_core::parser::yaml::{Schema, registry};

    let resolver = registry::get_rule_resolver(name)
        .ok_or_else(|| YamlValidateError::new_err(format!("unknown schema '{}'", name)))?;
    let Some(rr) = resolver.downcast_ref::<RuleResolver>() else {
        return Ok(());
    };
    let ast = pyrs_yaml_core::parser::parse(data, Schema::Core)
        .map_err(|e| YamlParseError::new_err(format!("failed to parse data: {}", e)))?;
    match validate_node(&ast, rr, data) {
        Ok(()) => Ok(()),
        Err(errors) => {
            let msg = errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            Err(YamlValidateError::new_err(msg))
        }
    }
}
