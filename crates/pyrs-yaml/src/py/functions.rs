//! Module-level Python-facing functions for `pyrs_yaml`.

use pyo3::prelude::*;

use crate::py::convert::{format_i18n_error, node_to_pyobject, parse_schema};
use crate::py::direct_dump::direct_dump;
use crate::py::document::{YamlDocument, parse_document, resolve_tags};
use crate::py::parse_error_to_py_err;
use crate::py::python_types::json_value_to_node;
use crate::py::stream_events::stream_event_to_py_dict;
use crate::py::stream_iterator::StreamIterator;
use crate::py::tag_registry;

use crate::YamlParseError;
use crate::YamlTypeError;

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
    let content = std::fs::read_to_string(path).map_err(|e| {
        pyo3::exceptions::PyIOError::new_err(format_i18n_error(
            "file-read-error",
            &[("detail", &e.to_string()), ("path", path)],
        ))
    })?;
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
    Ok(YamlDocument {
        ast,
        schema: schema_enum,
        source: Some(source.clone()),
        version: "1.2".to_string(),
        revision: 0,
        source_dirty: false,
        splice: None,
        splice_checked: false,
        snapshot: vec![],
    })
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
        .map(|ast| YamlDocument {
            ast,
            schema: schema_enum.clone(),
            source: Some(source.clone()),
            version: "1.2".to_string(),
            revision: 0,
            source_dirty: false,
            splice: None,
            splice_checked: false,
            snapshot: vec![],
        })
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
    let schema_clone = schema_enum.clone();
    let mut ast = py.detach(|| {
        crate::parser::parse_with_options(yaml, true, schema_clone, max_depth, allow_duplicate_keys)
            .map_err(|e| parse_error_to_py_err(e, yaml, max_depth))
    })?;
    resolve_tags(&mut ast, py)?;
    if yaml.bytes().any(|b| b == b'&') {
        let mut anchors = std::collections::HashMap::new();
        crate::py::convert::collect_anchors(&ast, &mut anchors);
        let mut visited = std::collections::HashSet::new();
        crate::py::convert::node_to_pyobject_with_anchors(
            &ast,
            py,
            &anchors,
            &mut visited,
            &schema_enum,
        )
    } else {
        crate::py::convert::node_to_pyobject_simple(&ast, py, &schema_enum)
    }
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
            if has_anchors {
                let mut anchors = std::collections::HashMap::new();
                crate::py::convert::collect_anchors(ast, &mut anchors);
                let mut visited = std::collections::HashSet::new();
                crate::py::convert::node_to_pyobject_with_anchors(
                    ast,
                    py,
                    &anchors,
                    &mut visited,
                    &schema_enum,
                )
            } else {
                crate::py::convert::node_to_pyobject_simple(ast, py, &schema_enum)
            }
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
    let yaml_str: String = if let Ok(s) = yaml.extract::<String>() {
        s
    } else if let Ok(bytes) = yaml.extract::<Vec<u8>>() {
        String::from_utf8(bytes).map_err(|e| {
            YamlParseError::new_err(format_i18n_error(
                "invalid-utf8",
                &[("detail", &e.to_string())],
            ))
        })?
    } else {
        return Err(YamlTypeError::new_err(format_i18n_error(
            "expected-str-or-bytes",
            &[],
        )));
    };

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
    let json_value: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        YamlParseError::new_err(format_i18n_error(
            "json-parse-error",
            &[("detail", &e.to_string())],
        ))
    })?;
    let node = json_value_to_node(&json_value)?;
    Ok(crate::serializer::to_yaml(&node))
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
    let content = py.detach(|| {
        std::fs::read_to_string(path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                "file-read-error",
                &[("detail", &e.to_string()), ("path", path)],
            ))
        })
    })?;
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
                    Some(node_to_pyobject(&ast, py, &schema_enum)?),
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
