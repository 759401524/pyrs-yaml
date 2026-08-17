//! Python bindings for pyrs-yaml — YamlDocument, StreamIterator, and all
//! Python-facing functions exposed via the `pyrs_yaml` PyO3 module.

pub mod convert;
pub mod direct_dump;
pub mod document;
pub mod editing;
pub mod functions;
pub mod stream_events;
pub mod stream_iterator;
pub mod streaming;
pub mod tag_registry;
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
        ParseError::Syntax { message, line, col } if line > 0 => {
            YamlParseError::new_err(format_source_snippet(source, line, col, &message))
        }
        ParseError::Syntax { message, .. } => YamlParseError::new_err(format_i18n_error(
            "yaml-parse-error",
            &[("detail", &message)],
        )),
    }
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

/// A Python module implemented in Rust.
///
/// pyrs-yaml: high-performance YAML parsing with perfect round-trip support.
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
    pub(crate) use super::functions::load_schema;
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
    pub(crate) use super::functions::validate_against_registered_schema;
    #[pymodule_export]
    pub(crate) use super::functions::validate_against_schema;
    #[pymodule_export]
    pub(crate) use super::functions::validate_custom_types;
}
