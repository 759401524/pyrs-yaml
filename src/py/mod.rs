//! Python bindings for pyrs-yaml — YamlDocument, StreamIterator, and all
//! Python-facing functions exposed via the `pyrs_yaml` PyO3 module.

pub mod convert;
pub mod stream_events;
pub mod tag_registry;

#[cfg(feature = "numpy")]
pub mod ndarray;

pub mod python_types;

use crate::ast::CustomNode;
use crate::parser::yaml::YamlSchema;
use crate::parser::StreamEvent;
use crate::serializer::{to_yaml, to_yaml_with_options, SerializeOptions};

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use self::convert::{
    collect_anchors, format_i18n_error, node_to_pyobject, node_to_pyobject_with_anchors,
    parse_schema,
};
use self::python_types::{json_value_to_node, pyobject_to_node};
use self::stream_events::stream_event_to_py_dict;

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
    use crate::YamlMaxDepthError;
    #[pymodule_export]
    use crate::YamlParseError;
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
                    YamlDuplicateKeyError::new_err(e.message.clone())
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
        Ok(YamlDocument {
            ast,
            schema: schema_enum,
            source: Some(Arc::from(yaml_str)),
            version: "1.2".to_string(),
        })
    }

    fn resolve_tags(node: &mut crate::ast::CustomNode, py: Python<'_>) -> PyResult<()> {
        use self::tag_registry::get_handlers;
        match node {
            crate::ast::CustomNode::Scalar {
                tag: Some(t),
                value,
                ..
            } => {
                let tag_name = t.to_string();
                if let Some(handlers) = get_handlers(&tag_name, py) {
                    for (_priority, handler) in handlers {
                        match handler.call1(py, (value.clone(),)) {
                            Ok(result) => {
                                if let Ok(s) = result.extract::<String>(py) {
                                    *value = s;
                                }
                                break;
                            }
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
                        YamlDuplicateKeyError::new_err(e.message.clone())
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
                        YamlDuplicateKeyError::new_err(e.message.clone())
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
                        YamlDuplicateKeyError::new_err(e.message.clone())
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
            Ok(YamlDocument {
                ast,
                schema: schema_enum,
                source: Some(Arc::from(content)),
                version: "1.2".to_string(),
            })
        }

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
                        YamlDuplicateKeyError::new_err(e.message.clone())
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
            Ok(asts
                .into_iter()
                .map(|ast| YamlDocument {
                    ast,
                    schema: schema_enum,
                    source: Some(Arc::from(yaml)),
                    version: "1.2".to_string(),
                })
                .collect())
        }
    }

    // ---- YamlDocument ----
    #[pyclass]
    struct YamlDocument {
        ast: CustomNode,
        schema: YamlSchema,
        source: Option<Arc<str>>,
        version: String,
    }

    #[pymethods]
    impl YamlDocument {
        /// 将文档序列化为 YAML 字符串（默认 2 空格缩进）。
        fn to_yaml(&self) -> PyResult<String> {
            self.to_yaml_with_options(2, false, false, false, 1000, 80, 2, 2, 0)
        }

        #[allow(clippy::too_many_arguments)]
        #[pyo3(signature = (indent_size: "int" = 2, explicit_start: "bool" = false, explicit_end: "bool" = false, sort_keys: "bool" = false, max_depth: "int" = 1000, width: "int" = 80, indent_mapping: "int" = 2, indent_sequence: "int" = 2, indent_offset: "int" = 0) -> "str")]
        fn to_yaml_with_options(
            &self,
            indent_size: usize,
            explicit_start: bool,
            explicit_end: bool,
            sort_keys: bool,
            max_depth: usize,
            width: usize,
            indent_mapping: usize,
            indent_sequence: usize,
            indent_offset: usize,
        ) -> PyResult<String> {
            let options = SerializeOptions {
                indent_size,
                explicit_start,
                explicit_end,
                sort_keys,
                max_depth,
                width,
                indent_mapping,
                indent_sequence,
                indent_offset,
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

        /// 将文档转换为 Python 字典/列表，自动解析锚点引用。
        fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
            let mut anchors = HashMap::new();
            collect_anchors(&self.ast, &mut anchors);
            let mut visited = HashSet::new();
            node_to_pyobject_with_anchors(&self.ast, py, &anchors, &mut visited, self.schema)
        }

        #[pyo3(signature = (key: "str", default: "Any" = None) -> "Any")]
        fn get(&self, py: Python, key: &str, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
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

        fn root_type(&self) -> String {
            match &self.ast {
                CustomNode::Scalar { .. } => "scalar".to_string(),
                CustomNode::Mapping { .. } => "mapping".to_string(),
                CustomNode::Sequence { .. } => "sequence".to_string(),
                CustomNode::Null { .. } => "null".to_string(),
                CustomNode::Alias { .. } => "alias".to_string(),
            }
        }

        fn __repr__(&self) -> String {
            format!("YamlDocument({})", self.to_yaml().unwrap_or_default())
        }

        fn __str__(&self) -> String {
            self.to_yaml()
                .unwrap_or_else(|_| "YamlDocument(error)".to_string())
        }

        fn __contains__(&self, key: &str) -> bool {
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    pairs.contains_key(&CustomNode::plain_scalar(key))
                }
                _ => false,
            }
        }

        fn __len__(&self) -> usize {
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => pairs.len(),
                CustomNode::Sequence { items, .. } => items.len(),
                _ => 0,
            }
        }

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

        fn source(&self) -> Option<&str> {
            self.source.as_deref()
        }

        fn version(&self) -> &str {
            &self.version
        }

        #[pyo3(signature = (resolve_merges: "bool" = true, schema: "str" = "core") -> "None")]
        fn reparse(&mut self, py: Python, resolve_merges: bool, schema: &str) -> PyResult<()> {
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
            Ok(())
        }

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
    }

    // ---- Python-facing functions ----

    #[pyfunction]
    #[pyo3(signature = (yaml: "str | bytes", resolve_merges: "bool" = true, schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false) -> "YamlDocument")]
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
                    YamlDuplicateKeyError::new_err(e.message.clone())
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
        Ok(YamlDocument {
            ast,
            schema: schema_enum,
            source: Some(Arc::from(content)),
            version: "1.2".to_string(),
        })
    }

    #[pyfunction]
    #[pyo3(signature = (yaml: "str", resolve_merges: "bool" = true, schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false) -> "list[YamlDocument]")]
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
                    YamlDuplicateKeyError::new_err(e.message.clone())
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
        Ok(asts
            .into_iter()
            .map(|ast| YamlDocument {
                ast,
                schema: schema_enum,
                source: Some(Arc::from(yaml)),
                version: "1.2".to_string(),
            })
            .collect())
    }

    #[pyfunction]
    #[pyo3(signature = (yaml: "str", schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = false) -> "dict[str, Any] | list[Any]")]
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
                    YamlDuplicateKeyError::new_err(e.message.clone())
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
                    YamlDuplicateKeyError::new_err(e.message.clone())
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
    struct StreamIterator {
        events: Vec<StreamEvent>,
        index: usize,
    }

    #[pymethods]
    impl StreamIterator {
        fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

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
    fn safe_dump(py: Python, data: Py<PyAny>) -> PyResult<String> {
        let node = pyobject_to_node(py, &data)?;
        Ok(to_yaml(&node))
    }

    #[pyfunction]
    #[pyo3(signature = (data: "dict[str, Any] | list[Any]") -> "str")]
    fn from_dict(py: Python, data: Py<PyAny>) -> PyResult<String> {
        let node = pyobject_to_node(py, &data)?;
        Ok(to_yaml(&node))
    }

    #[pyfunction]
    #[pyo3(signature = (json_str: "str") -> "str")]
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
    fn get_language() -> &'static str {
        crate::i18n::get_language_static()
    }

    #[pyfunction]
    #[pyo3(signature = () -> "list[str]")]
    fn list_languages() -> Vec<&'static str> {
        crate::i18n::list_languages()
    }

    #[pyfunction]
    #[pyo3(signature = () -> "str")]
    fn detect_language() -> String {
        crate::i18n::detect_language()
    }

    #[pyfunction]
    #[pyo3(signature = (user_locales: "list[str]", default: "str" = "en") -> "str")]
    fn negotiate_language(user_locales: &Bound<'_, PyAny>, default: &str) -> PyResult<String> {
        let locales: Vec<String> = user_locales.extract()?;
        let refs: Vec<&str> = locales.iter().map(|s| s.as_str()).collect();
        Ok(crate::i18n::negotiate_language(&refs, default).to_string())
    }

    #[pyfunction]
    #[pyo3(signature = (name: "str", handler: "Py<PyAny>", priority: "u32" = 0))]
    fn register_tag(name: &str, handler: Py<PyAny>, priority: u32) {
        tag_registry::register(name, handler, priority);
    }

    #[pyfunction]
    fn clear_tag_handlers() {
        tag_registry::clear_all();
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::{parse, yaml::YamlSchema};

    #[test]
    fn test_parse_and_serialize() {
        let yaml = "key: value";
        let ast = parse(yaml, YamlSchema::Core).unwrap();
        let output = crate::serializer::to_yaml(&ast);
        assert_eq!(output, "key: value\n");
    }
}
