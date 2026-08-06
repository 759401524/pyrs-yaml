//! Python bindings for pyrs-yaml — YamlDocument, StreamIterator, and all
//! Python-facing functions exposed via the `pyrs_yaml` PyO3 module.

pub mod convert;
pub mod editing;
pub mod stream_events;
pub mod streaming;
pub mod tag_registry;
pub mod writing;

#[cfg(feature = "numpy")]
pub mod ndarray;

pub mod python_types;

use crate::ast::CustomNode;
use crate::parser::yaml::YamlSchema;
use crate::parser::StreamEvent;
use crate::serializer::{to_yaml, to_yaml_with_options, SerializeOptions};
use crate::splice::SpliceState;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use self::convert::{
    collect_anchors, format_i18n_error, node_to_pyobject, node_to_pyobject_with_anchors,
    parse_schema,
};
use self::python_types::{json_value_to_node, pyobject_to_node};
use self::stream_events::stream_event_to_py_dict;
use self::streaming::{ChunkCharIter, InputSrc, YamlStream, DEFAULT_CHUNK_SIZE};
use self::writing::{dump_iterable, dump_options, OutputSink, SinkWriter};

/// Format a YAML parse error with source context and caret marker.
///
/// Output:
/// ```text
/// YAML parse error at line 5, col 12: unexpected mapping value
///     |
///   5 | key: value: extra
///     |            ^^^^^^ unexpected mapping value
/// ```
fn format_source_snippet(source: &str, line: usize, col: usize, message: &str) -> String {
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

use crate::py::editing::segment_py::SegmentExt;

/// A Python module implemented in Rust.
///
/// pyrs-yaml: high-performance YAML parsing with perfect round-trip support.
#[pymodule(gil_used = false)]
mod pyrs_yaml {
    use super::*;

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

    // ---- Helper: shared parse logic ----
    fn parse_document(
        py: Python,
        yaml: &Bound<'_, PyAny>,
        resolve_merges: bool,
        schema: &str,
        max_depth: usize,
        allow_duplicate_keys: bool,
    ) -> PyResult<YamlDocument> {
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

        let schema_enum = parse_schema(schema)?;
        let mut ast = py.detach(|| {
            crate::parser::parse_with_options(
                &yaml_str,
                resolve_merges,
                schema_enum,
                max_depth,
                allow_duplicate_keys,
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
                        &[("max_depth", &max_depth.to_string())],
                    ))
                } else if e.line > 0 {
                    let msg = format_source_snippet(&yaml_str, e.line, e.col, &e.message);
                    YamlParseError::new_err(msg)
                } else {
                    YamlParseError::new_err(format_i18n_error(
                        "yaml-parse-error",
                        &[("detail", &e.message)],
                    ))
                }
            })
        })?;
        resolve_tags(&mut ast, py)?;
        let source: Arc<str> = Arc::from(yaml_str);
        Ok(YamlDocument {
            ast,
            schema: schema_enum,
            source: Some(source),
            version: "1.2".to_string(),
            revision: 0,
            source_dirty: false,
            splice: None,
            splice_checked: false,
            snapshot: vec![],
        })
    }

    fn resolve_tags(node: &mut crate::ast::CustomNode, py: Python<'_>) -> PyResult<()> {
        use self::tag_registry::get_handlers;
        // Fast path: no handlers registered, skip the whole AST traversal
        if self::tag_registry::is_empty() {
            return Ok(());
        }
        match node {
            crate::ast::CustomNode::Scalar {
                tag: Some(t),
                value,
                ..
            } => {
                let tag_name = t.to_string();
                if let Some(handlers) = get_handlers(&tag_name, py) {
                    for (_priority, handler) in handlers {
                        match handler.call1(py, (value.as_ref(),)) {
                            Ok(result) => match result.extract::<String>(py) {
                                Ok(s) => {
                                    *value = Arc::from(s);
                                    break;
                                }
                                Err(_) => {
                                    return Err(YamlTagError::new_err(format!(
                                        "Tag handler '{}' must return a string",
                                        tag_name
                                    )));
                                }
                            },
                            Err(e) => {
                                if e.is_instance_of::<crate::YamlTagSkip>(py) {
                                    continue;
                                }
                                return Err(YamlTagError::new_err(format!(
                                    "Tag handler '{}' failed: {}",
                                    tag_name, e
                                )));
                            }
                        }
                    }
                }
                Ok(())
            }
            crate::ast::CustomNode::Mapping { pairs, .. } => {
                for (_, v) in pairs.iter_mut() {
                    resolve_tags(v, py)?;
                }
                Ok(())
            }
            crate::ast::CustomNode::Sequence { items, .. } => {
                for item in items.iter_mut() {
                    resolve_tags(item, py)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    // ---- YAML instance API ----
    #[pyclass]
    /// Configured YAML parser instance (rt / safe / full).
    #[allow(clippy::upper_case_acronyms)]
    struct YAML {
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
            let mut anchors = HashMap::new();
            collect_anchors(&ast, &mut anchors);
            let mut visited = HashSet::new();
            node_to_pyobject_with_anchors(&ast, py, &anchors, &mut visited, schema_enum)
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
            let mut results = Vec::with_capacity(asts.len());
            for ast in asts {
                let mut anchors = HashMap::new();
                collect_anchors(&ast, &mut anchors);
                let mut visited = HashSet::new();
                let obj =
                    node_to_pyobject_with_anchors(&ast, py, &anchors, &mut visited, schema_enum)?;
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
                self::writing::DEFAULT_CHUNK_SIZE,
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
                self::writing::DEFAULT_CHUNK_SIZE,
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

    // ---- YamlDocument ----
    #[derive(Clone)]
    struct DocumentSnapshot {
        ast: CustomNode,
        splice: Option<SpliceState>,
        source: Option<Arc<str>>,
        revision: u64,
        source_dirty: bool,
        splice_checked: bool,
    }

    #[pyclass]
    /// Round-trip editable YAML document with transaction support, path editing, and source preservation.
    pub(crate) struct YamlDocument {
        ast: CustomNode,
        schema: YamlSchema,
        source: Option<Arc<str>>,
        version: String,
        revision: u64,      // bumped on every mutation (Node invalidation)
        source_dirty: bool, // lazy source re-serialization flag
        /// Segment-splice accumulator for default-layout documents; `None`
        /// when the doc is not splice-eligible or the state was consumed.
        splice: Option<SpliceState>,
        /// Whether splice eligibility has been computed (lazily, on first edit).
        splice_checked: bool,
        /// Transaction snapshot stack. `__enter__` pushes, `__exit__` pops.
        /// Grows only as deep as `with` nesting; `None` means empty stack.
        snapshot: Vec<DocumentSnapshot>,
    }

    /// Serialize a document's AST with the given options, flushing any pending
    /// splice edits first. Shared by `to_yaml_with_options` and streaming
    /// write. The serialize itself runs detached from the GIL.
    pub(crate) fn serialize_document(
        doc: &mut YamlDocument,
        py: Python,
        options: &SerializeOptions,
    ) -> PyResult<String> {
        doc.flush_source(py)?;
        py.detach(|| to_yaml_with_options(&doc.ast, options))
            .map_err(|e| {
                if e.contains("max depth exceeded") {
                    YamlMaxDepthError::new_err(format_i18n_error(
                        "max-depth-exceeded",
                        &[("max_depth", &options.max_depth.to_string())],
                    ))
                } else {
                    YamlSerializeError::new_err(format_i18n_error(
                        "yaml-serialize-error",
                        &[("detail", &e)],
                    ))
                }
            })
    }

    impl YamlDocument {
        /// Non-`#[pymethod]` constructor used by benches and integration tests.
        /// Splice eligibility is computed lazily on first edit via
        /// [`ensure_splice`], so `splice` starts `None`.
        #[allow(dead_code)] // pub but struct is not pub — used by benches (separate crate)
        pub fn from_ast(ast: CustomNode, source: Arc<str>) -> Self {
            YamlDocument {
                ast,
                schema: crate::parser::yaml::YamlSchema::Core,
                source: Some(source.clone()),
                version: "1.2".to_string(),
                revision: 0,
                source_dirty: false,
                splice: None,
                splice_checked: false,
                snapshot: vec![],
            }
        }

        /// Lazily compute splice eligibility on first edit. `splice_checked`
        /// distinguishes "not yet computed" from "consumed or ineligible" —
        /// the single-burst model must NOT re-create state after materialize.
        ///
        /// Field-reference form so edit closures keep disjoint field captures.
        fn ensure_splice(
            splice: &mut Option<SpliceState>,
            checked: &mut bool,
            ast: &CustomNode,
            source: &Option<Arc<str>>,
        ) {
            if *checked || splice.is_some() {
                return;
            }
            *checked = true;
            if let Some(src) = source {
                if crate::parser::check_default_layout(ast, src) {
                    *splice = Some(SpliceState::new(src.clone()));
                }
            }
        }
    }

    #[pymethods]
    impl YamlDocument {
        /// Lazily re-serialize the AST into `source` if edits have occurred.
        fn flush_source(&mut self, py: Python) -> PyResult<()> {
            if self.source_dirty {
                // Splice path: replay the accumulated segments against the
                // original source. Runs at most once — the state is consumed
                // (set to None) whether or not it materializes, so a later
                // burst falls back to a full serialize.
                let spliced = py.detach(|| self.splice.as_ref().and_then(|s| s.materialize()));
                let new_yaml = match spliced {
                    Some(text) => text,
                    None => py
                        .detach(|| {
                            crate::serializer::to_yaml_with_options(
                                &self.ast,
                                &SerializeOptions::default(),
                            )
                        })
                        .map_err(|e| {
                            YamlSerializeError::new_err(format_i18n_error(
                                "yaml-serialize-error",
                                &[("detail", &e)],
                            ))
                        })?,
                };
                self.source = Some(Arc::from(new_yaml));
                self.splice = None;
                self.source_dirty = false;
            }
            Ok(())
        }

        /// Serialize the document to a YAML string (default 2-space indent).
        #[allow(clippy::wrong_self_convention)] // needs &mut self to flush lazy source
        fn to_yaml(&mut self, py: Python) -> PyResult<String> {
            self.flush_source(py)?;
            self.to_yaml_with_options(py, 2, false, false, false, 1000, 80, None, None, None)
        }

        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::wrong_self_convention)]
        #[pyo3(signature = (indent_size: "int" = 2, explicit_start: "bool" = false, explicit_end: "bool" = false, sort_keys: "bool" = false, max_depth: "int" = 1000, width: "int" = 80, indent_mapping: "int | None" = None, indent_sequence: "int | None" = None, indent_offset: "int | None" = None) -> "str")]
        /// Serialize with customizable indent, sorting, and explicit start/end markers.
        fn to_yaml_with_options(
            &mut self,
            py: Python,
            indent_size: usize,
            explicit_start: bool,
            explicit_end: bool,
            sort_keys: bool,
            max_depth: usize,
            width: usize,
            indent_mapping: Option<usize>,
            indent_sequence: Option<usize>,
            indent_offset: Option<usize>,
        ) -> PyResult<String> {
            self.flush_source(py)?;
            let options = SerializeOptions {
                indent_size,
                explicit_start,
                explicit_end,
                sort_keys,
                max_depth,
                width,
                indent_mapping: indent_mapping.unwrap_or(indent_size),
                indent_sequence: indent_sequence.unwrap_or(indent_size),
                indent_offset: indent_offset.unwrap_or(0),
            };
            to_yaml_with_options(&self.ast, &options).map_err(|e| {
                if e.contains("max depth exceeded") {
                    YamlMaxDepthError::new_err(format_i18n_error(
                        "max-depth-exceeded",
                        &[("max_depth", &max_depth.to_string())],
                    ))
                } else {
                    YamlSerializeError::new_err(format_i18n_error(
                        "yaml-serialize-error",
                        &[("detail", &e)],
                    ))
                }
            })
        }

        /// Convert the document to a Python dict/list, resolving anchor references.
        fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
            let mut anchors = HashMap::new();
            collect_anchors(&self.ast, &mut anchors);
            let mut visited = HashSet::new();
            node_to_pyobject_with_anchors(&self.ast, py, &anchors, &mut visited, self.schema)
        }

        /// Access a value by key or path (e.g. `a.b[0]`), returning `default` if not found.
        #[pyo3(signature = (key: "str", default: "Any" = None) -> "Any")]
        fn get(&self, py: Python, key: &str, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
            let is_path = key.starts_with('$') || key.contains('.') || key.contains('[');
            if is_path {
                let segs = editing::parse_path_segments(key).map_err(|e| {
                    YamlPathError::new_err(format_i18n_error("path-error", &[("detail", &e)]))
                })?;
                return match editing::navigate(&self.ast, &segs) {
                    Ok(node) => Ok(node_to_pyobject(node, py, self.schema)?),
                    Err(_) => Ok(default.unwrap_or_else(|| py.None())),
                };
            }
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    let key_node = CustomNode::plain_scalar(key);
                    if let Some(value) = pairs.get(&key_node) {
                        Ok(node_to_pyobject(value, py, self.schema)?)
                    } else {
                        Ok(default.unwrap_or_else(|| py.None()))
                    }
                }
                _ => Ok(default.unwrap_or_else(|| py.None())),
            }
        }

        /// Return the root node type: `scalar`/`mapping`/`sequence`/`null`/`alias`.
        fn root_type(&self) -> String {
            match &self.ast {
                CustomNode::Scalar { .. } => "scalar".to_string(),
                CustomNode::Mapping { .. } => "mapping".to_string(),
                CustomNode::Sequence { .. } => "sequence".to_string(),
                CustomNode::Null { .. } => "null".to_string(),
                CustomNode::Alias { .. } => "alias".to_string(),
            }
        }

        /// Return a `YamlDocument(...)` representation.
        fn __repr__(&mut self, py: Python) -> String {
            format!("YamlDocument({})", self.to_yaml(py).unwrap_or_default())
        }

        /// Return the YAML string representation.
        fn __str__(&mut self, py: Python) -> String {
            self.to_yaml(py)
                .unwrap_or_else(|_| "YamlDocument(error)".to_string())
        }

        /// Check if a key exists in the mapping.
        fn __contains__(&self, key: &str) -> bool {
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    pairs.contains_key(&CustomNode::plain_scalar(key))
                }
                _ => false,
            }
        }

        /// Return the number of mapping entries or sequence length.
        fn __len__(&self) -> usize {
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => pairs.len(),
                CustomNode::Sequence { items, .. } => items.len(),
                _ => 0,
            }
        }

        /// Return an iterator over keys (mapping) or values (sequence).
        fn __iter__<'py>(&self, _py: Python<'py>) -> PyResult<Py<PyAny>> {
            let schema = self.schema;
            Python::attach(|py| match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    let keys: Vec<Py<PyAny>> = pairs
                        .keys()
                        .map(|k| node_to_pyobject(k, py, schema))
                        .collect::<PyResult<Vec<_>>>()?;
                    Ok(keys.into_pyobject(py)?.into_any().unbind())
                }
                CustomNode::Sequence { items, .. } => {
                    let values: Vec<Py<PyAny>> = items
                        .iter()
                        .map(|v| node_to_pyobject(v, py, schema))
                        .collect::<PyResult<Vec<_>>>()?;
                    Ok(values.into_pyobject(py)?.into_any().unbind())
                }
                _ => Ok(Vec::<Py<PyAny>>::new()
                    .into_pyobject(py)?
                    .into_any()
                    .unbind()),
            })
        }

        /// Access a child node by key (mapping) or index (sequence).
        fn __getitem__<'py>(&self, py: Python<'py>, key: Py<PyAny>) -> PyResult<Py<PyAny>> {
            let schema = self.schema;
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    if let Ok(key_str) = key.bind(py).extract::<String>() {
                        let key_node = CustomNode::plain_scalar(key_str.clone());
                        if let Some(value) = pairs.get(&key_node) {
                            Ok(node_to_pyobject(value, py, schema)?)
                        } else {
                            Err(pyo3::exceptions::PyKeyError::new_err(format_i18n_error(
                                "key-not-found",
                                &[("key", key_str.as_str())],
                            )))
                        }
                    } else {
                        Err(YamlTypeError::new_err(format_i18n_error(
                            "key-not-string",
                            &[],
                        )))
                    }
                }
                CustomNode::Sequence { items, .. } => {
                    if let Ok(idx) = key.bind(py).extract::<usize>() {
                        if idx < items.len() {
                            Ok(node_to_pyobject(&items[idx], py, schema)?)
                        } else {
                            Err(pyo3::exceptions::PyIndexError::new_err(format_i18n_error(
                                "index-out-of-range",
                                &[
                                    ("index", &idx.to_string()),
                                    ("len", &items.len().to_string()),
                                ],
                            )))
                        }
                    } else {
                        Err(YamlTypeError::new_err(format_i18n_error(
                            "index-not-integer",
                            &[],
                        )))
                    }
                }
                _ => Err(YamlTypeError::new_err(format_i18n_error(
                    "not-subscriptable",
                    &[],
                ))),
            }
        }

        /// Return the current YAML source string.
        fn source(&mut self, py: Python) -> PyResult<String> {
            self.flush_source(py)?;
            Ok(self.source.as_deref().unwrap_or("").to_string())
        }

        /// Walk the AST depth-first, returning a list of path tuples.
        /// Each path tuple contains strings (mapping keys) and ints (sequence indices).
        /// The first element is always the root node (empty path).
        fn _walk_paths(&self, py: Python) -> PyResult<Vec<Py<PyAny>>> {
            let mut paths = Vec::new();
            let mut path = Vec::new();
            walk_ast(&self.ast, &mut path, &mut paths, py)?;
            Ok(paths)
        }

        /// Walk only scalar/null nodes, returning their path tuples.
        fn _scalar_paths(&self, py: Python) -> PyResult<Vec<Py<PyAny>>> {
            let mut paths = Vec::new();
            let mut path = Vec::new();
            walk_scalars(&self.ast, &mut path, &mut paths, py)?;
            Ok(paths)
        }

        /// Set a value by path (internal, called by `__setitem__`).
        #[pyo3(signature = (segments: "list", value: "Any", create_missing: "bool" = false) -> "None")]
        fn _set_path(
            &mut self,
            py: Python,
            segments: Vec<Py<PyAny>>,
            value: Py<PyAny>,
            create_missing: bool,
        ) -> PyResult<()> {
            let segs: Vec<editing::Segment<'_>> = segments
                .iter()
                .map(|s| editing::Segment::from_py(s.bind(py)))
                .collect::<Result<Vec<_>, pyo3::PyErr>>()
                .map_err(|e| YamlEditError::new_err(e.to_string()))?;
            let new_node = pyobject_to_node(py, &value)?;
            py.detach(|| -> Result<(), String> {
                let src = self.source.as_deref().unwrap_or("");
                Self::ensure_splice(
                    &mut self.splice,
                    &mut self.splice_checked,
                    &self.ast,
                    &self.source,
                );
                let unit = {
                    let offsets = self.splice.as_mut().map(|s| s.line_offsets());
                    editing::set_path(
                        &mut self.ast,
                        &segs,
                        new_node,
                        true,
                        src,
                        offsets,
                        create_missing,
                    )?
                };
                if let Some(state) = self.splice.as_mut() {
                    if state.apply(&unit).is_err() {
                        self.splice = None;
                    }
                }
                Ok(())
            })
            .map_err(|e| {
                YamlEditError::new_err(format_i18n_error("edit-error", &[("detail", &e)]))
            })?;
            self.revision = self.revision.wrapping_add(1);
            self.source_dirty = true;
            Ok(())
        }

        /// Insert a value by path (internal, inserts at a sequence position).
        #[pyo3(signature = (segments: "list", index: "int", value: "Any") -> "None")]
        fn _insert_path(
            &mut self,
            py: Python,
            segments: Vec<Py<PyAny>>,
            index: i64,
            value: Py<PyAny>,
        ) -> PyResult<()> {
            let segs: Vec<editing::Segment<'_>> = segments
                .iter()
                .map(|s| editing::Segment::from_py(s.bind(py)))
                .collect::<Result<Vec<_>, pyo3::PyErr>>()
                .map_err(|e| YamlEditError::new_err(e.to_string()))?;
            let new_node = pyobject_to_node(py, &value)?;
            py.detach(|| -> Result<(), String> {
                let src = self.source.as_deref().unwrap_or("");
                Self::ensure_splice(
                    &mut self.splice,
                    &mut self.splice_checked,
                    &self.ast,
                    &self.source,
                );
                let unit = {
                    let offsets = self.splice.as_mut().map(|s| s.line_offsets());
                    editing::insert_path(&mut self.ast, &segs, index, new_node, src, offsets)?
                };
                if let Some(state) = self.splice.as_mut() {
                    if state.apply(&unit).is_err() {
                        self.splice = None;
                    }
                }
                Ok(())
            })
            .map_err(|e| {
                YamlEditError::new_err(format_i18n_error("edit-error", &[("detail", &e)]))
            })?;
            self.revision = self.revision.wrapping_add(1);
            self.source_dirty = true;
            Ok(())
        }

        /// Append a value by path (internal, appends to a sequence).
        #[pyo3(signature = (segments: "list", value: "Any") -> "None")]
        fn _append_path(
            &mut self,
            py: Python,
            segments: Vec<Py<PyAny>>,
            value: Py<PyAny>,
        ) -> PyResult<()> {
            let segs: Vec<editing::Segment<'_>> = segments
                .iter()
                .map(|s| editing::Segment::from_py(s.bind(py)))
                .collect::<Result<Vec<_>, pyo3::PyErr>>()
                .map_err(|e| YamlEditError::new_err(e.to_string()))?;
            let new_node = pyobject_to_node(py, &value)?;
            py.detach(|| -> Result<(), String> {
                let src = self.source.as_deref().unwrap_or("");
                Self::ensure_splice(
                    &mut self.splice,
                    &mut self.splice_checked,
                    &self.ast,
                    &self.source,
                );
                let unit = {
                    let offsets = self.splice.as_mut().map(|s| s.line_offsets());
                    editing::append_path(&mut self.ast, &segs, new_node, src, offsets)?
                };
                if let Some(state) = self.splice.as_mut() {
                    if state.apply(&unit).is_err() {
                        self.splice = None;
                    }
                }
                Ok(())
            })
            .map_err(|e| {
                YamlEditError::new_err(format_i18n_error("edit-error", &[("detail", &e)]))
            })?;
            self.revision = self.revision.wrapping_add(1);
            self.source_dirty = true;
            Ok(())
        }

        /// Delete a node by path (internal, called by `__delitem__`).
        #[pyo3(signature = (segments: "list") -> "None")]
        fn _delete_path(&mut self, py: Python, segments: Vec<Py<PyAny>>) -> PyResult<()> {
            let segs: Vec<editing::Segment<'_>> = segments
                .iter()
                .map(|s| editing::Segment::from_py(s.bind(py)))
                .collect::<Result<Vec<_>, pyo3::PyErr>>()
                .map_err(|e| YamlEditError::new_err(e.to_string()))?;
            py.detach(|| -> Result<(), String> {
                let src = self.source.as_deref().unwrap_or("");
                Self::ensure_splice(
                    &mut self.splice,
                    &mut self.splice_checked,
                    &self.ast,
                    &self.source,
                );
                let unit = {
                    let offsets = self.splice.as_mut().map(|s| s.line_offsets());
                    editing::delete_path(&mut self.ast, &segs, src, offsets)?
                };
                if let Some(state) = self.splice.as_mut() {
                    if state.apply(&unit).is_err() {
                        self.splice = None;
                    }
                }
                Ok(())
            })
            .map_err(|e| {
                YamlEditError::new_err(format_i18n_error("edit-error", &[("detail", &e)]))
            })?;
            self.revision = self.revision.wrapping_add(1);
            self.source_dirty = true;
            Ok(())
        }

        /// Rename a mapping key by path (internal).
        #[pyo3(signature = (segments: "list", new_key: "str") -> "None")]
        fn _rename_path(
            &mut self,
            py: Python,
            segments: Vec<Py<PyAny>>,
            new_key: &str,
        ) -> PyResult<()> {
            let segs: Vec<editing::Segment<'_>> = segments
                .iter()
                .map(|s| editing::Segment::from_py(s.bind(py)))
                .collect::<Result<Vec<_>, pyo3::PyErr>>()
                .map_err(|e| YamlEditError::new_err(e.to_string()))?;
            py.detach(|| -> Result<(), String> {
                let src = self.source.as_deref().unwrap_or("");
                Self::ensure_splice(
                    &mut self.splice,
                    &mut self.splice_checked,
                    &self.ast,
                    &self.source,
                );
                let unit = {
                    let offsets = self.splice.as_mut().map(|s| s.line_offsets());
                    editing::rename_path(&mut self.ast, &segs, new_key, src, offsets)?
                };
                if let Some(state) = self.splice.as_mut() {
                    if state.apply(&unit).is_err() {
                        self.splice = None;
                    }
                }
                Ok(())
            })
            .map_err(|e| {
                YamlEditError::new_err(format_i18n_error("edit-error", &[("detail", &e)]))
            })?;
            self.revision = self.revision.wrapping_add(1);
            self.source_dirty = true;
            Ok(())
        }

        /// Set the value for a mapping key, `doc['key'] = value`.
        fn __setitem__(&mut self, py: Python, key: String, value: Py<PyAny>) -> PyResult<()> {
            self._set_path(
                py,
                vec![key.into_pyobject(py)?.into_any().unbind()],
                value,
                false,
            )
        }

        /// Delete a mapping key, `del doc['key']`.
        fn __delitem__(&mut self, py: Python, key: String) -> PyResult<()> {
            self._delete_path(py, vec![key.into_pyobject(py)?.into_any().unbind()])
        }

        /// Return the current revision number (incremented on each edit).
        fn _revision(&self) -> u64 {
            self.revision
        }

        /// Return the YAML version string.
        fn version(&self) -> &str {
            &self.version
        }

        /// Reparse the current source, optionally changing merge behavior and schema.
        #[pyo3(signature = (resolve_merges: "bool" = true, schema: "str" = "core") -> "None")]
        fn reparse(&mut self, py: Python, resolve_merges: bool, schema: &str) -> PyResult<()> {
            self.flush_source(py)?;
            let source = self.source.as_ref().ok_or_else(|| {
                YamlTypeError::new_err(format_i18n_error("no-source-to-reparse", &[]))
            })?;
            let schema_enum = parse_schema(schema)?;
            let new_ast = py.detach(|| {
                crate::parser::parse_with_options(source, resolve_merges, schema_enum, 1000, false)
                    .map_err(|e| {
                        if e.line > 0 {
                            let msg = format_source_snippet(source, e.line, e.col, &e.message);
                            YamlParseError::new_err(msg)
                        } else {
                            YamlParseError::new_err(format_i18n_error(
                                "yaml-parse-error",
                                &[("detail", &e.message)],
                            ))
                        }
                    })
            })?;
            self.ast = new_ast;
            self.schema = schema_enum;
            self.splice = None;
            self.splice_checked = false;
            Ok(())
        }

        /// Serialize to a JSON string (via Python `json.dumps`).
        #[pyo3(signature = (indent: "int" = 2) -> "str")]
        fn to_json(&self, py: Python, indent: usize) -> PyResult<String> {
            let obj = self.to_dict(py)?;
            let json_module = py.import("json")?;
            let kw = PyDict::new(py);
            kw.set_item("indent", indent)?;
            let s = json_module
                .call_method("dumps", (obj,), Some(&kw))?
                .extract()?;
            Ok(s)
        }

        /// Validate the document against a JSON Schema.
        #[pyo3(signature = (schema: "str | dict[str, Any]") -> "None")]
        fn validate(&self, py: Python, schema: &Bound<'_, PyAny>) -> PyResult<()> {
            let instance = self.to_dict(py)?;
            let schema_obj: Bound<'_, PyAny> = if let Ok(schema_str) = schema.extract::<String>() {
                let json_module = py.import("json")?;
                json_module.call_method("loads", (schema_str,), None)?
            } else {
                schema.clone()
            };
            let jsonschema = py.import("jsonschema")?;
            let validate_fn = jsonschema.getattr("validate")?;
            let kw = PyDict::new(py);
            kw.set_item("instance", instance)?;
            kw.set_item("schema", schema_obj)?;
            validate_fn.call((), Some(&kw)).map_err(|e| {
                let msg = e
                    .value(py)
                    .getattr("message")
                    .ok()
                    .and_then(|m| m.extract::<String>().ok())
                    .unwrap_or_else(|| e.to_string());
                YamlValidateError::new_err(msg)
            })?;
            Ok(())
        }

        /// Enter a transaction scope: snapshot AST + splice state. `with doc:` exits cleanly
        /// preserving edits; exceptions roll back the snapshot.
        fn __enter__(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
            let snap = DocumentSnapshot {
                ast: slf.ast.clone(),
                splice: slf.splice.clone(),
                source: slf.source.clone(),
                revision: slf.revision,
                source_dirty: slf.source_dirty,
                splice_checked: slf.splice_checked,
            };
            slf.snapshot.push(snap);
            slf
        }

        #[pyo3(signature = (exc_type: "Any | None" = None, exc_value: "Any | None" = None, tb: "Any | None" = None) -> "bool")]
        fn __exit__(
            &mut self,
            exc_type: Option<Bound<'_, PyAny>>,
            exc_value: Option<Bound<'_, PyAny>>,
            tb: Option<Bound<'_, PyAny>>,
        ) -> bool {
            let _ = (exc_value, tb);
            if let Some(snap) = self.snapshot.pop() {
                if exc_type.is_some() {
                    self.ast = snap.ast;
                    self.splice = snap.splice;
                    self.source = snap.source;
                    self.revision = snap.revision;
                    self.source_dirty = snap.source_dirty;
                    self.splice_checked = snap.splice_checked;
                }
            }
            false // never swallow exceptions
        }
    }

    // ---- Python-facing functions ----

    #[pyfunction]
    #[pyo3(signature = (yaml: "str | bytes", resolve_merges: "bool" = true, schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false) -> "YamlDocument")]
    /// Parse a YAML string (str or bytes) and return an editable `YamlDocument`.
    fn parse(
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
    fn parse_file(
        py: Python,
        path: &str,
        schema: &str,
        max_depth: usize,
        allow_duplicate_keys: bool,
    ) -> PyResult<YamlDocument> {
        let schema_enum = parse_schema(schema)?;
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
                schema_enum,
                max_depth,
                allow_duplicate_keys,
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
                        &[("max_depth", &max_depth.to_string())],
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
        resolve_tags(&mut ast, py)?;
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

    #[pyfunction]
    #[pyo3(signature = (yaml: "str", resolve_merges: "bool" = true, schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false) -> "list[YamlDocument]")]
    /// Parse a multi-document YAML stream and return all `YamlDocument` objects.
    fn parse_all_docs(
        py: Python,
        yaml: &str,
        resolve_merges: bool,
        schema: &str,
        max_depth: usize,
        allow_duplicate_keys: bool,
    ) -> PyResult<Vec<YamlDocument>> {
        let schema_enum = parse_schema(schema)?;
        let asts = py.detach(|| {
            crate::parser::parse_all_with_options(
                yaml,
                resolve_merges,
                schema_enum,
                max_depth,
                allow_duplicate_keys,
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
                        &[("max_depth", &max_depth.to_string())],
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

    #[pyfunction]
    #[pyo3(signature = (yaml: "str", schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false) -> "dict[str, Any] | list[Any]")]
    /// Parse YAML into a Python dict/list, resolving anchors and merges.
    fn safe_load(
        py: Python,
        yaml: &str,
        schema: &str,
        max_depth: usize,
        allow_duplicate_keys: bool,
    ) -> PyResult<Py<PyAny>> {
        let schema_enum = parse_schema(schema)?;
        let mut ast = py.detach(|| {
            crate::parser::parse_with_options(
                yaml,
                true,
                schema_enum,
                max_depth,
                allow_duplicate_keys,
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
                        &[("max_depth", &max_depth.to_string())],
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
        resolve_tags(&mut ast, py)?;
        let mut anchors = HashMap::new();
        collect_anchors(&ast, &mut anchors);
        let mut visited = HashSet::new();
        node_to_pyobject_with_anchors(&ast, py, &anchors, &mut visited, schema_enum)
    }

    #[pyfunction]
    #[pyo3(signature = (yaml: "str", schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false) -> "list[dict[str, Any] | list[Any]]")]
    /// Parse a multi-document YAML stream into a list of dicts/lists.
    fn safe_loads(
        py: Python,
        yaml: &str,
        schema: &str,
        max_depth: usize,
        allow_duplicate_keys: bool,
    ) -> PyResult<Vec<Py<PyAny>>> {
        let schema_enum = parse_schema(schema)?;
        let asts = py.detach(|| {
            crate::parser::parse_all_with_options(
                yaml,
                true,
                schema_enum,
                max_depth,
                allow_duplicate_keys,
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
                        &[("max_depth", &max_depth.to_string())],
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
        asts.iter()
            .map(|ast| {
                let mut anchors = HashMap::new();
                collect_anchors(ast, &mut anchors);
                let mut visited = HashSet::new();
                node_to_pyobject_with_anchors(ast, py, &anchors, &mut visited, schema_enum)
            })
            .collect()
    }

    #[pyfunction]
    #[pyo3(signature = (yaml: "str | bytes", on_event: "Callable[[dict[str, Any]], bool] | None" = None) -> "StreamIterator | None")]
    /// Event-stream parsing. With `on_event` callback, consumes events and returns `None`. Otherwise returns a lazy `StreamIterator`.
    fn parse_stream(
        py: Python,
        yaml: &Bound<'_, PyAny>,
        on_event: Option<Py<PyAny>>,
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
                crate::parser::parse_stream(&yaml_str).map_err(|e| {
                    if e.line > 0 {
                        let msg = format_source_snippet(&yaml_str, e.line, e.col, &e.message);
                        YamlParseError::new_err(msg)
                    } else {
                        YamlParseError::new_err(format_i18n_error(
                            "yaml-parse-error",
                            &[("detail", &e.message)],
                        ))
                    }
                })
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
                crate::parser::parse_stream(&yaml_str).map_err(|e| {
                    if e.line > 0 {
                        let msg = format_source_snippet(&yaml_str, e.line, e.col, &e.message);
                        YamlParseError::new_err(msg)
                    } else {
                        YamlParseError::new_err(format_i18n_error(
                            "yaml-parse-error",
                            &[("detail", &e.message)],
                        ))
                    }
                })
            })?;

            let iter = StreamIterator { events, index: 0 };
            Ok(iter.into_pyobject(py)?.into_any().unbind())
        }
    }

    #[pyclass]
    /// YAML event stream iterator, yielding parsed events one by one.
    struct StreamIterator {
        events: Vec<StreamEvent>,
        index: usize,
    }

    #[pymethods]
    impl StreamIterator {
        /// Return self (iterator protocol).
        fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

        /// Yield the next event dict; return `None` when the stream ends.
        fn __next__<'a>(&mut self, py: Python<'a>) -> PyResult<Option<Bound<'a, PyDict>>> {
            if self.index < self.events.len() {
                let event = &self.events[self.index];
                self.index += 1;
                Ok(Some(stream_event_to_py_dict(py, event)?))
            } else {
                Ok(None)
            }
        }
    }

    #[pyfunction]
    #[pyo3(signature = (data: "dict[str, Any] | list[Any]") -> "str")]
    /// Serialize a Python dict/list to a YAML string.
    fn safe_dump(py: Python, data: Py<PyAny>) -> PyResult<String> {
        let node = pyobject_to_node(py, &data)?;
        Ok(to_yaml(&node))
    }

    #[pyfunction]
    #[pyo3(signature = (data: "dict[str, Any] | list[Any]") -> "str")]
    /// Convert a Python dict/list to a YAML string (auto-selects block/flow style).
    fn from_dict(py: Python, data: Py<PyAny>) -> PyResult<String> {
        let node = pyobject_to_node(py, &data)?;
        Ok(to_yaml(&node))
    }

    #[pyfunction]
    #[pyo3(signature = (json_str: "str") -> "str")]
    /// Convert a JSON string to a YAML string.
    fn from_json(_py: Python, json_str: &str) -> PyResult<String> {
        let json_value: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            YamlParseError::new_err(format_i18n_error(
                "json-parse-error",
                &[("detail", &e.to_string())],
            ))
        })?;
        let node = json_value_to_node(&json_value)?;
        Ok(to_yaml(&node))
    }

    #[pyfunction]
    #[pyo3(signature = (data: "Any", path: "str") -> "None")]
    /// Serialize a Python object to YAML and write to a file.
    fn dump_file(py: Python, data: Py<PyAny>, path: &str) -> PyResult<()> {
        let node = pyobject_to_node(py, &data)?;
        let yaml = to_yaml(&node);
        std::fs::write(path, yaml).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                "file-write-error",
                &[("detail", &e.to_string()), ("path", path)],
            ))
        })?;
        Ok(())
    }

    #[pyfunction]
    #[pyo3(signature = (path: "str", schema: "str" = "core") -> "tuple[dict[str, Any] | None, str]")]
    /// Read a Markdown file and extract YAML front matter, returning `(frontmatter, body)`.
    fn read_markdown(
        py: Python,
        path: &str,
        schema: &str,
    ) -> PyResult<(Option<Py<PyAny>>, String)> {
        let content = py.detach(|| {
            std::fs::read_to_string(path).map_err(|e| {
                pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                    "file-read-error",
                    &[("detail", &e.to_string()), ("path", path)],
                ))
            })
        })?;
        read_markdown_str(py, &content, schema)
    }

    #[pyfunction]
    #[pyo3(signature = (content: "str", schema: "str" = "core") -> "tuple[dict[str, Any] | None, str]")]
    /// Extract YAML front matter from a Markdown string, returning `(frontmatter, body)`.
    fn read_markdown_str(
        _py: Python,
        content: &str,
        schema: &str,
    ) -> PyResult<(Option<Py<PyAny>>, String)> {
        let content = content.trim_start();
        let schema_enum = parse_schema(schema)?;

        if let Some(rest) = content.strip_prefix("---") {
            if let Some(end_idx) = rest.find("---") {
                let frontmatter = rest[..end_idx].trim();
                let markdown_content = rest[end_idx + 3..].trim();

                if !frontmatter.is_empty() {
                    return Python::attach(|py| {
                        let ast = crate::parser::parse(frontmatter, schema_enum).map_err(|e| {
                            if e.line > 0 {
                                let msg =
                                    format_source_snippet(frontmatter, e.line, e.col, &e.message);
                                YamlParseError::new_err(msg)
                            } else {
                                YamlParseError::new_err(format_i18n_error(
                                    "yaml-parse-error",
                                    &[("detail", &e.message)],
                                ))
                            }
                        })?;
                        Ok((
                            Some(node_to_pyobject(&ast, py, schema_enum)?),
                            markdown_content.to_string(),
                        ))
                    });
                }
            }
        }

        Ok((None, content.to_string()))
    }

    #[pyfunction]
    #[pyo3(signature = (lang: "str") -> "None")]
    /// Set the error message language.
    fn set_language(lang: &str) -> PyResult<()> {
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
    fn get_language() -> &'static str {
        crate::i18n::get_language_static()
    }

    #[pyfunction]
    #[pyo3(signature = () -> "list[str]")]
    /// List supported language codes.
    fn list_languages() -> Vec<&'static str> {
        crate::i18n::list_languages()
    }

    #[pyfunction]
    #[pyo3(signature = () -> "str")]
    /// Detect the system default language.
    fn detect_language() -> String {
        crate::i18n::detect_language()
    }

    #[pyfunction]
    #[pyo3(signature = (user_locales: "list[str]", default: "str" = "en") -> "str")]
    /// Negotiate a language from user locale list and default.
    fn negotiate_language(user_locales: &Bound<'_, PyAny>, default: &str) -> PyResult<String> {
        let locales: Vec<String> = user_locales.extract()?;
        let refs: Vec<&str> = locales.iter().map(|s| s.as_str()).collect();
        Ok(crate::i18n::negotiate_language(&refs, default).to_string())
    }

    #[pyfunction]
    #[pyo3(signature = (name: "str", handler: "Py<PyAny>", priority: "u32" = 0))]
    /// Register a custom tag handler.
    fn register_tag(name: &str, handler: Py<PyAny>, priority: u32) {
        tag_registry::register(name, handler, priority);
    }

    #[pyfunction]
    /// Clear all tag handlers.
    fn clear_tag_handlers() {
        tag_registry::clear_all();
    }

    #[pyfunction]
    #[pyo3(signature = (name: "str"))]
    /// Remove a specific tag handler.
    fn remove_tag(name: &str) {
        tag_registry::remove(name);
    }

    // ---- D4: Rust-backed AST traversal ----

    fn walk_ast<'a>(
        node: &CustomNode,
        path: &mut Vec<Bound<'a, PyAny>>,
        paths: &mut Vec<Py<PyAny>>,
        py: Python<'a>,
    ) -> PyResult<()> {
        paths.push(
            PyTuple::new(py, path.iter().map(|p| p as &Bound<'_, PyAny>))?
                .into_any()
                .unbind(),
        );
        match node {
            CustomNode::Mapping { pairs, .. } => {
                for (k, v) in pairs.iter() {
                    let key_str = match k {
                        CustomNode::Scalar { value, .. } => value.as_ref(),
                        _ => continue,
                    };
                    let item = key_str.into_pyobject(py)?.into_any();
                    path.push(item);
                    walk_ast(v, path, paths, py)?;
                    path.pop();
                }
            }
            CustomNode::Sequence { items, .. } => {
                for (i, item) in items.iter().enumerate() {
                    let idx = (i as i64).into_pyobject(py)?.into_any();
                    path.push(idx);
                    walk_ast(item, path, paths, py)?;
                    path.pop();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn walk_scalars<'a>(
        node: &CustomNode,
        path: &mut Vec<Bound<'a, PyAny>>,
        paths: &mut Vec<Py<PyAny>>,
        py: Python<'a>,
    ) -> PyResult<()> {
        match node {
            CustomNode::Scalar { .. } | CustomNode::Null { .. } => {
                paths.push(
                    PyTuple::new(py, path.iter().map(|p| p as &Bound<'_, PyAny>))?
                        .into_any()
                        .unbind(),
                );
            }
            CustomNode::Mapping { pairs, .. } => {
                for (k, v) in pairs.iter() {
                    let key_str = match k {
                        CustomNode::Scalar { value, .. } => value.as_ref(),
                        _ => continue,
                    };
                    let item = key_str.into_pyobject(py)?.into_any();
                    path.push(item);
                    walk_scalars(v, path, paths, py)?;
                    path.pop();
                }
            }
            CustomNode::Sequence { items, .. } => {
                for (i, item) in items.iter().enumerate() {
                    let idx = (i as i64).into_pyobject(py)?.into_any();
                    path.push(idx);
                    walk_scalars(item, path, paths, py)?;
                    path.pop();
                }
            }
            _ => {}
        }
        Ok(())
    }
}
