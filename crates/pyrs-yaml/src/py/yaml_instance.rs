//! YAML — configured parser instance (rt / safe / full).

use pyo3::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::py::convert::{
    collect_anchors, format_i18n_error, node_to_pyobject_simple, node_to_pyobject_with_anchors,
    parse_schema,
};
use crate::py::document::{YamlDocument, parse_document};
use crate::py::streaming::{ChunkCharIter, DEFAULT_CHUNK_SIZE, InputSrc, YamlStream};
use crate::py::writing::{OutputSink, SinkWriter, dump_iterable, dump_options};

use crate::YamlDuplicateKeyError;
use crate::YamlMaxDepthError;
use crate::YamlParseError;
use crate::YamlTypeError;

/// Configured YAML parser instance (rt / safe / full).
#[pyclass]
#[allow(clippy::upper_case_acronyms)]
pub(crate) struct YAML {
    yaml_type: String,
    schema: String,
    max_depth: usize,
    allow_duplicate_keys: bool,
}

#[pymethods]
impl YAML {
    #[new]
    /// Create a YAML instance; `typ` can be `rt`/`safe`/`full`.
    #[pyo3(signature = (typ: "str" = "rt", schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false))]
    fn new(
        typ: &str,
        schema: &str,
        max_depth: usize,
        allow_duplicate_keys: bool,
    ) -> PyResult<Self> {
        let valid_types = ["rt", "safe", "full"];
        let valid_schemas = ["core", "yaml1.1", "failsafe", "json"];
        if !valid_types.contains(&typ) {
            return Err(YamlTypeError::new_err(format!(
                "Invalid YAML type: '{}'. Valid types: rt, safe, full",
                typ
            )));
        }
        if !valid_schemas.contains(&schema) {
            return Err(YamlTypeError::new_err(format!(
                "Invalid schema: '{}'. Valid schemas: core, yaml1.1, failsafe, json",
                schema
            )));
        }
        Ok(YAML {
            yaml_type: typ.to_string(),
            schema: schema.to_string(),
            max_depth,
            allow_duplicate_keys,
        })
    }

    /// Parse a YAML string and return a `YamlDocument`.
    #[pyo3(signature = (yaml: "str | bytes") -> "YamlDocument")]
    fn parse(&self, py: Python, yaml: &Bound<'_, PyAny>) -> PyResult<YamlDocument> {
        let resolve_merges = self.yaml_type == "rt" || self.yaml_type == "full";
        parse_document(
            py,
            yaml,
            resolve_merges,
            &self.schema,
            self.max_depth,
            self.allow_duplicate_keys,
        )
    }

    /// Parse YAML into a dict/list (resolves anchors and merges).
    #[pyo3(signature = (yaml: "str") -> "dict[str, Any] | list[Any]")]
    fn safe_load(&self, py: Python, yaml: &str) -> PyResult<Py<PyAny>> {
        let schema_enum = parse_schema(&self.schema)?;
        let ast = py.detach(|| {
            crate::parser::parse_with_options(
                yaml,
                true,
                schema_enum,
                self.max_depth,
                self.allow_duplicate_keys,
            )
            .map_err(|e| {
                if e.message.contains("duplicate key") {
                    let key = e.message.trim_start_matches("duplicate key: ");
                    YamlDuplicateKeyError::new_err(format_i18n_error(
                        "duplicate-key",
                        &[("key", key)],
                    ))
                } else if e.message.contains("max depth exceeded") {
                    YamlMaxDepthError::new_err(format_i18n_error(
                        "max-depth-exceeded",
                        &[("max_depth", &self.max_depth.to_string())],
                    ))
                } else if e.line > 0 {
                    let msg = format_source_snippet(yaml, e.line, e.col, &e.message);
                    YamlParseError::new_err(msg)
                } else {
                    YamlParseError::new_err(format_i18n_error(
                        "yaml-parse-error",
                        &[("detail", &e.message)],
                    ))
                }
            })
        })?;
        if yaml.bytes().any(|b| b == b'&') {
            let mut anchors = HashMap::new();
            collect_anchors(&ast, &mut anchors);
            let mut visited = HashSet::new();
            node_to_pyobject_with_anchors(&ast, py, &anchors, &mut visited, schema_enum)
        } else {
            node_to_pyobject_simple(&ast, py, schema_enum)
        }
    }

    /// Parse multi-document YAML into a list of dicts/lists.
    #[pyo3(signature = (yaml: "str") -> "list[dict[str, Any] | list[Any]]")]
    fn safe_loads(&self, py: Python, yaml: &str) -> PyResult<Vec<Py<PyAny>>> {
        let schema_enum = parse_schema(&self.schema)?;
        let asts = py.detach(|| {
            crate::parser::parse_all_with_options(
                yaml,
                true,
                schema_enum,
                self.max_depth,
                self.allow_duplicate_keys,
            )
            .map_err(|e| {
                if e.message.contains("duplicate key") {
                    let key = e.message.trim_start_matches("duplicate key: ");
                    YamlDuplicateKeyError::new_err(format_i18n_error(
                        "duplicate-key",
                        &[("key", key)],
                    ))
                } else if e.message.contains("max depth exceeded") {
                    YamlMaxDepthError::new_err(format_i18n_error(
                        "max-depth-exceeded",
                        &[("max_depth", &self.max_depth.to_string())],
                    ))
                } else if e.line > 0 {
                    let msg = format_source_snippet(yaml, e.line, e.col, &e.message);
                    YamlParseError::new_err(msg)
                } else {
                    YamlParseError::new_err(format_i18n_error(
                        "yaml-parse-error",
                        &[("detail", &e.message)],
                    ))
                }
            })
        })?;
        let has_anchors = yaml.bytes().any(|b| b == b'&');
        let mut results = Vec::with_capacity(asts.len());
        for ast in asts {
            let obj = if has_anchors {
                let mut anchors = HashMap::new();
                collect_anchors(&ast, &mut anchors);
                let mut visited = HashSet::new();
                node_to_pyobject_with_anchors(&ast, py, &anchors, &mut visited, schema_enum)?
            } else {
                node_to_pyobject_simple(&ast, py, schema_enum)?
            };
            results.push(obj);
        }
        Ok(results)
    }

    /// Parse a YAML file and return a `YamlDocument`.
    #[pyo3(signature = (path: "str") -> "YamlDocument")]
    fn parse_file(&self, py: Python, path: &str) -> PyResult<YamlDocument> {
        let resolve_merges = self.yaml_type == "rt" || self.yaml_type == "full";
        let schema_enum = parse_schema(&self.schema)?;
        let content = std::fs::read_to_string(path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                "file-read-error",
                &[("detail", &e.to_string()), ("path", path)],
            ))
        })?;
        let ast = py.detach(|| {
            crate::parser::parse_with_options(
                &content,
                resolve_merges,
                schema_enum,
                self.max_depth,
                self.allow_duplicate_keys,
            )
            .map_err(|e| {
                if e.message.contains("duplicate key") {
                    let key = e.message.trim_start_matches("duplicate key: ");
                    YamlDuplicateKeyError::new_err(format_i18n_error(
                        "duplicate-key",
                        &[("key", key)],
                    ))
                } else if e.message.contains("max depth exceeded") {
                    YamlMaxDepthError::new_err(format_i18n_error(
                        "max-depth-exceeded",
                        &[("max_depth", &self.max_depth.to_string())],
                    ))
                } else if e.line > 0 {
                    let msg = format_source_snippet(&content, e.line, e.col, &e.message);
                    YamlParseError::new_err(msg)
                } else {
                    YamlParseError::new_err(format_i18n_error(
                        "yaml-parse-error",
                        &[("detail", &e.message)],
                    ))
                }
            })
        })?;
        let source: Arc<str> = Arc::from(content);
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

    /// Parse multi-document YAML and return a list of `YamlDocument`.
    #[pyo3(signature = (yaml: "str") -> "list[YamlDocument]")]
    fn parse_all_docs(&self, py: Python, yaml: &str) -> PyResult<Vec<YamlDocument>> {
        let resolve_merges = self.yaml_type == "rt" || self.yaml_type == "full";
        let schema_enum = parse_schema(&self.schema)?;
        let asts = py.detach(|| {
            crate::parser::parse_all_with_options(
                yaml,
                resolve_merges,
                schema_enum,
                self.max_depth,
                self.allow_duplicate_keys,
            )
            .map_err(|e| {
                if e.message.contains("duplicate key") {
                    let key = e.message.trim_start_matches("duplicate key: ");
                    YamlDuplicateKeyError::new_err(format_i18n_error(
                        "duplicate-key",
                        &[("key", key)],
                    ))
                } else if e.message.contains("max depth exceeded") {
                    YamlMaxDepthError::new_err(format_i18n_error(
                        "max-depth-exceeded",
                        &[("max_depth", &self.max_depth.to_string())],
                    ))
                } else if e.line > 0 {
                    let msg = format_source_snippet(yaml, e.line, e.col, &e.message);
                    YamlParseError::new_err(msg)
                } else {
                    YamlParseError::new_err(format_i18n_error(
                        "yaml-parse-error",
                        &[("detail", &e.message)],
                    ))
                }
            })
        })?;
        let source: Arc<str> = Arc::from(yaml);
        Ok(asts
            .into_iter()
            .map(|ast| YamlDocument {
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
            .collect())
    }

    /// Lazy event iterator: incrementally read from `file_obj` (read() returns str or bytes).
    #[pyo3(signature = (file_obj: "Any") -> "YamlStream")]
    fn load_stream(&self, _py: Python, file_obj: Bound<'_, PyAny>) -> PyResult<YamlStream> {
        if file_obj.getattr("read").is_err() {
            return Err(YamlTypeError::new_err(format_i18n_error(
                "expected-readable",
                &[],
            )));
        }
        let src = InputSrc::PyObj(file_obj.unbind());
        Ok(YamlStream::new(ChunkCharIter::new(src, DEFAULT_CHUNK_SIZE)))
    }

    /// Lazy event iterator: incrementally read from file path (Rust File, no GIL blocking).
    #[pyo3(signature = (path: "str") -> "YamlStream")]
    fn load_stream_file(&self, _py: Python, path: &str) -> PyResult<YamlStream> {
        let file = std::fs::File::open(path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                "file-read-error",
                &[("detail", &e.to_string()), ("path", path)],
            ))
        })?;
        let src = InputSrc::File(std::io::BufReader::new(file));
        Ok(YamlStream::new(ChunkCharIter::new(src, DEFAULT_CHUNK_SIZE)))
    }

    /// Streaming writer: serialize documents to `file_obj` (write(str)), constant memory.
    #[pyo3(signature = (file_obj: "Any", iterable: "Any", explicit_start: "bool" = false, explicit_end: "bool" = false, sort_keys: "bool" = false) -> "None")]
    fn dump_stream(
        &self,
        py: Python,
        file_obj: Bound<'_, PyAny>,
        iterable: Bound<'_, PyAny>,
        explicit_start: bool,
        explicit_end: bool,
        sort_keys: bool,
    ) -> PyResult<()> {
        if file_obj.getattr("write").is_err() {
            return Err(YamlTypeError::new_err(format_i18n_error(
                "expected-writable",
                &[],
            )));
        }
        let mut writer = SinkWriter::new(
            OutputSink::PyObj(file_obj.unbind()),
            crate::py::writing::DEFAULT_CHUNK_SIZE,
        );
        dump_iterable(
            py,
            &mut writer,
            iterable,
            &dump_options(sort_keys),
            explicit_start,
            explicit_end,
        )
    }

    /// Streaming writer: serialize documents to `path` (Rust File, no GIL blocking).
    #[pyo3(signature = (path: "str", iterable: "Any", explicit_start: "bool" = false, explicit_end: "bool" = false, sort_keys: "bool" = false) -> "None")]
    fn dump_file(
        &self,
        py: Python,
        path: &str,
        iterable: Bound<'_, PyAny>,
        explicit_start: bool,
        explicit_end: bool,
        sort_keys: bool,
    ) -> PyResult<()> {
        let file = std::fs::File::create(path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                "file-write-error",
                &[("detail", &e.to_string()), ("path", path)],
            ))
        })?;
        let mut writer = SinkWriter::new(
            OutputSink::File(std::io::BufWriter::new(file)),
            crate::py::writing::DEFAULT_CHUNK_SIZE,
        );
        dump_iterable(
            py,
            &mut writer,
            iterable,
            &dump_options(sort_keys),
            explicit_start,
            explicit_end,
        )
    }
}

use crate::py::format_source_snippet;
