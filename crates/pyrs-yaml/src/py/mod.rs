//! Python bindings for pyrs-yaml — YamlDocument, StreamIterator, and all
//! Python-facing functions exposed via the `pyrs_yaml` PyO3 module.

pub mod convert;
pub mod direct_dump;
pub mod direct_load;
pub mod document;
pub mod editing;
pub mod functions;
pub mod json_fast;
pub mod json_lines;
pub mod stream_events;
pub mod stream_iterator;
pub mod streaming;
pub mod tag_registry;
pub mod tags;
pub mod toml_direct;
pub mod type_registry;
pub mod walk_helpers;
pub mod writing;
pub mod yaml_instance;

#[cfg(feature = "numpy")]
pub mod ndarray;

pub mod python_types;

use pyo3::prelude::*;

use crate::error::ParseError;
use crate::py::convert::format_i18n_error;
use crate::{YamlDuplicateKeyError, YamlMaxDepthError, YamlParseError};

/// Map a core [`ParseError`] to the matching Python exception.
///
/// Preserves the historical Python-facing behavior:
/// - `DuplicateKey` → `YamlDuplicateKeyError`
/// - `MaxDepthExceeded` → `YamlMaxDepthError`
/// - `Syntax` with a source position → `YamlParseError` with a snippet
/// - fallback → `YamlParseError` with the i18n message
pub(crate) fn parse_error_to_py_err(e: ParseError, source: &str, max_depth: usize) -> PyErr {
    match e {
        ParseError::DuplicateKey(key) => {
            YamlDuplicateKeyError::new_err(format_i18n_error("duplicate-key", &[("key", &key)]))
        }
        ParseError::MaxDepthExceeded(_) => YamlMaxDepthError::new_err(format_i18n_error(
            "max-depth-exceeded",
            &[("max_depth", &max_depth.to_string())],
        )),
        ParseError::SingleDocumentExpected { line, col } => {
            YamlParseError::new_err(single_document_message(line, col))
        }
        ParseError::Syntax { message, line, col } if line > 0 => {
            YamlParseError::new_err(format_source_snippet(source, line, col, &message))
        }
        ParseError::Syntax { message, .. } => YamlParseError::new_err(format_i18n_error(
            "yaml-parse-error",
            &[("detail", &message)],
        )),
    }
}

/// The refusal for one document requested from an input that carries several.
///
/// `multi-document-input` leads the message so a caller can grep the reason out of an exception
/// string, the same way [`crate::error::SerializeError::UnsupportedValue`] reasons are keyed.
/// The position is reported one-based because that is how the reader counts lines, and it is the
/// position granit anchors the second document at — its own `---` (or that document's `%YAML`
/// directive), which is where a caller would cut the text. `line` and `col` arrive 0-indexed from
/// the engine, per [`crate::error::ParseError::SingleDocumentExpected`].
pub(crate) fn single_document_message(line: usize, col: usize) -> String {
    format!(
        "multi-document-input: this input carries a second document at line {}, column {} and \
         this route reads one. Read every document with safe_loads() or parse_all_docs(), or the \
         events with parse_stream(). PyYAML and ruamel.yaml raise at this same point.",
        line + 1,
        col + 1
    )
}

/// Format a YAML parse error with source context and caret marker.
///
/// Output:
/// ```text
/// YAML parse error at line 5, col 12: unexpected mapping value
///     |
///   5 | key: value: extra
///     |            ^^^^^^ unexpected mapping value
/// ```
pub(crate) fn format_source_snippet(
    source: &str,
    line: usize,
    col: usize,
    message: &str,
) -> String {
    let source_lines: Vec<&str> = source.lines().collect();
    let line_num = if line > 0 { line } else { 0 };
    let line_idx = line_num.saturating_sub(1);
    let source_line = source_lines.get(line_idx).copied().unwrap_or("");

    // Build caret: 5 chars or remaining width, whichever is smaller
    let caret_width = 5.min(source_line.len().saturating_sub(col));
    let caret = if caret_width > 0 {
        format!("{}{} {}", " ".repeat(col), "^".repeat(caret_width), message)
    } else {
        message.to_string()
    };

    format!(
        "YAML parse error at line {}, col {}: {}\n    |\n{:>4} | {}\n    | {}\n",
        line_num, col, message, line_num, source_line, caret
    )
}

/// Map an IO failure on a file read to `PyIOError` with the i18n
/// `file-read-error` message. One definition so the `read_to_string` path and
/// the memory-mapped path (which turns a bad byte into the same `InvalidData`
/// message) report a failure to Python identically.
pub(crate) fn file_read_error(detail: &str, path: &str) -> pyo3::PyErr {
    pyo3::exceptions::PyIOError::new_err(format_i18n_error(
        "file-read-error",
        &[("detail", detail), ("path", path)],
    ))
}

/// Read a UTF-8 file, mapping IO failures to `PyIOError` with the i18n
/// `file-read-error` message.
pub(crate) fn read_file_to_string(path: &str) -> PyResult<String> {
    std::fs::read_to_string(path).map_err(|e| file_read_error(&e.to_string(), path))
}

/// Open a file for reading, mapping IO failures to `PyIOError` with the same
/// i18n message as [`read_file_to_string`].
pub(crate) fn open_file(path: &str) -> PyResult<std::fs::File> {
    std::fs::File::open(path).map_err(|e| {
        pyo3::exceptions::PyIOError::new_err(format_i18n_error(
            "file-read-error",
            &[("detail", &e.to_string()), ("path", path)],
        ))
    })
}

/// A Python module implemented in Rust.
///
/// pyrs-yaml: high-performance YAML, TOML, JSON, JSONC and JSON5 parsing with
/// perfect round-trip support.
#[pymodule(gil_used = false)]
mod pyrs_yaml {
    // ---- exceptions ----
    #[pymodule_export]
    use crate::YamlDuplicateKeyError;
    #[pymodule_export]
    use crate::YamlEditError;
    #[pymodule_export]
    use crate::YamlMaxDepthError;
    #[pymodule_export]
    use crate::YamlParseError;
    #[pymodule_export]
    use crate::YamlPathError;
    #[pymodule_export]
    use crate::YamlSerializeError;
    #[pymodule_export]
    use crate::YamlTagError;
    #[pymodule_export]
    use crate::YamlTagSkip;
    #[pymodule_export]
    use crate::YamlTypeError;
    #[pymodule_export]
    use crate::YamlValidateError;

    // ---- YamlDocument ----
    #[pymodule_export]
    pub(crate) use super::document::YamlDocument;

    // ---- serialize_document helper (not a pyfunction) ----
    pub(crate) use super::document::serialize_document;

    // ---- YAML class ----
    #[pymodule_export]
    pub(crate) use super::yaml_instance::YAML;

    // ---- StreamIterator ----
    #[pymodule_export]
    pub(crate) use super::stream_iterator::StreamIterator;

    // ---- YamlStream (lazy load_stream event iterator) ----
    #[pymodule_export]
    pub(crate) use super::streaming::YamlStream;

    // ---- JsonLines (lazy line-delimited JSON value iterator) ----
    #[pymodule_export]
    pub(crate) use super::json_lines::JsonLines;

    // ---- Module-level functions ----
    #[pymodule_export]
    pub(crate) use super::functions::clear_tag_handlers;
    #[pymodule_export]
    pub(crate) use super::functions::clear_type_handlers;
    #[pymodule_export]
    pub(crate) use super::functions::detect_language;
    #[pymodule_export]
    pub(crate) use super::functions::dump_file;
    #[pymodule_export]
    pub(crate) use super::functions::from_dict;
    #[pymodule_export]
    pub(crate) use super::functions::from_json;
    #[pymodule_export]
    pub(crate) use super::functions::from_json5;
    #[pymodule_export]
    pub(crate) use super::functions::from_jsonc;
    #[pymodule_export]
    pub(crate) use super::functions::from_toml;
    #[pymodule_export]
    pub(crate) use super::functions::get_language;
    #[pymodule_export]
    pub(crate) use super::functions::get_plugin;
    #[pymodule_export]
    pub(crate) use super::functions::list_languages;
    #[pymodule_export]
    pub(crate) use super::functions::list_plugins;
    #[pymodule_export]
    pub(crate) use super::functions::list_schemas;
    #[pymodule_export]
    pub(crate) use super::functions::load_json;
    #[pymodule_export]
    pub(crate) use super::functions::load_json_lines;
    #[pymodule_export]
    pub(crate) use super::functions::load_json5;
    #[pymodule_export]
    pub(crate) use super::functions::load_jsonc;
    #[pymodule_export]
    pub(crate) use super::functions::load_schema;
    #[pymodule_export]
    pub(crate) use super::functions::load_toml;
    #[pymodule_export]
    pub(crate) use super::functions::negotiate_language;
    #[pymodule_export]
    pub(crate) use super::functions::parse;
    #[pymodule_export]
    pub(crate) use super::functions::parse_all_docs;
    #[pymodule_export]
    pub(crate) use super::functions::parse_file;
    #[pymodule_export]
    pub(crate) use super::functions::parse_stream;
    #[pymodule_export]
    pub(crate) use super::functions::read_markdown;
    #[pymodule_export]
    pub(crate) use super::functions::read_markdown_str;
    #[pymodule_export]
    pub(crate) use super::functions::register_schema;
    #[pymodule_export]
    pub(crate) use super::functions::register_tag;
    #[pymodule_export]
    pub(crate) use super::functions::register_type;
    #[pymodule_export]
    pub(crate) use super::functions::remove_tag;
    #[pymodule_export]
    pub(crate) use super::functions::remove_type;
    #[pymodule_export]
    pub(crate) use super::functions::safe_dump;
    #[pymodule_export]
    pub(crate) use super::functions::safe_load;
    #[pymodule_export]
    pub(crate) use super::functions::safe_loads;
    #[pymodule_export]
    pub(crate) use super::functions::set_language;
    #[pymodule_export]
    pub(crate) use super::functions::to_toml;
    #[pymodule_export]
    pub(crate) use super::functions::validate_custom_types;
}
