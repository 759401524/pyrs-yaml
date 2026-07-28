//! Python bindings for pyyaml-rs — YamlDocument, StreamIterator, and all
//! Python-facing functions exposed via the `pyyaml_rs` PyO3 module.

pub mod convert;
pub mod ndarray;
pub mod python_types;
pub mod stream_events;

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

/// A Python module implemented in Rust.
///
/// pyyaml-rs: high-performance YAML parsing with perfect round-trip support.
#[pymodule]
mod pyyaml_rs {
    use super::*;

    // ---- exceptions ----
    #[pymodule_export]
    use crate::YamlParseError;
    #[pymodule_export]
    use crate::YamlSerializeError;
    #[pymodule_export]
    use crate::YamlTypeError;
    #[pymodule_export]
    use crate::YamlValidateError;

    // ---- YamlDocument ----
    #[pyclass]
    struct YamlDocument {
        ast: CustomNode,
        schema: YamlSchema,
        source: Option<Arc<str>>,
    }

    #[pymethods]
    impl YamlDocument {
        /// 将文档序列化为 YAML 字符串（默认 2 空格缩进）。
        fn to_yaml(&self) -> String {
            to_yaml(&self.ast)
        }

        #[pyo3(signature = (indent_size: "int" = 2, explicit_start: "bool" = false, explicit_end: "bool" = false, sort_keys: "bool" = false) -> "str")]
        fn to_yaml_with_options(
            &self,
            indent_size: usize,
            explicit_start: bool,
            explicit_end: bool,
            sort_keys: bool,
        ) -> String {
            let options = SerializeOptions {
                indent_size,
                explicit_start,
                explicit_end,
                sort_keys,
            };
            to_yaml_with_options(&self.ast, &options)
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
            format!("YamlDocument({})", self.to_yaml())
        }

        fn __str__(&self) -> String {
            self.to_yaml()
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

        #[pyo3(signature = (resolve_merges: "bool" = true, schema: "str" = "core") -> "None")]
        fn reparse(&mut self, py: Python, resolve_merges: bool, schema: &str) -> PyResult<()> {
            let source = self.source.as_ref().ok_or_else(|| {
                YamlTypeError::new_err(format_i18n_error("no-source-to-reparse", &[]))
            })?;
            let schema_enum = parse_schema(schema)?;
            let new_ast = py.detach(|| {
                crate::parser::parse_with_options(source, resolve_merges, schema_enum).map_err(
                    |e| {
                        YamlParseError::new_err(format_i18n_error(
                            "yaml-parse-error",
                            &[("detail", &e.to_string())],
                        ))
                    },
                )
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
    #[pyo3(signature = (yaml: "str | bytes", resolve_merges: "bool" = true, schema: "str" = "core") -> "YamlDocument")]
    fn parse(
        py: Python,
        yaml: &Bound<'_, PyAny>,
        resolve_merges: bool,
        schema: &str,
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
        let ast = py.detach(|| {
            crate::parser::parse_with_options(&yaml_str, resolve_merges, schema_enum).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        Ok(YamlDocument {
            ast,
            schema: schema_enum,
            source: Some(Arc::from(yaml_str)),
        })
    }

    #[pyfunction]
    #[pyo3(signature = (path: "str", schema: "str" = "core") -> "YamlDocument")]
    fn parse_file(py: Python, path: &str, schema: &str) -> PyResult<YamlDocument> {
        let schema_enum = parse_schema(schema)?;
        let content = std::fs::read_to_string(path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                "file-read-error",
                &[("detail", &e.to_string()), ("path", path)],
            ))
        })?;
        let ast = py.detach(|| {
            crate::parser::parse_with_options(&content, true, schema_enum).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        Ok(YamlDocument {
            ast,
            schema: schema_enum,
            source: Some(Arc::from(content)),
        })
    }

    #[pyfunction]
    #[pyo3(signature = (yaml: "str", resolve_merges: "bool" = true, schema: "str" = "core") -> "list[YamlDocument]")]
    fn parse_all_docs(
        py: Python,
        yaml: &str,
        resolve_merges: bool,
        schema: &str,
    ) -> PyResult<Vec<YamlDocument>> {
        let schema_enum = parse_schema(schema)?;
        let asts = py.detach(|| {
            crate::parser::parse_all_with_options(yaml, resolve_merges, schema_enum).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        Ok(asts
            .into_iter()
            .map(|ast| YamlDocument {
                ast,
                schema: schema_enum,
                source: Some(Arc::from(yaml)),
            })
            .collect())
    }

    #[pyfunction]
    #[pyo3(signature = (yaml: "str", schema: "str" = "core") -> "dict[str, Any] | list[Any]")]
    fn safe_load(py: Python, yaml: &str, schema: &str) -> PyResult<Py<PyAny>> {
        let schema_enum = parse_schema(schema)?;
        let ast = py.detach(|| {
            crate::parser::parse(yaml, schema_enum).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        let mut anchors = HashMap::new();
        collect_anchors(&ast, &mut anchors);
        let mut visited = HashSet::new();
        node_to_pyobject_with_anchors(&ast, py, &anchors, &mut visited, schema_enum)
    }

    #[pyfunction]
    #[pyo3(signature = (yaml: "str", schema: "str" = "core") -> "list[dict[str, Any] | list[Any]]")]
    fn safe_loads(py: Python, yaml: &str, schema: &str) -> PyResult<Vec<Py<PyAny>>> {
        let schema_enum = parse_schema(schema)?;
        let asts = py.detach(|| {
            crate::parser::parse_all(yaml, schema_enum).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
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
                    YamlParseError::new_err(format_i18n_error(
                        "yaml-parse-error",
                        &[("detail", &e.to_string())],
                    ))
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
                    YamlParseError::new_err(format_i18n_error(
                        "yaml-parse-error",
                        &[("detail", &e.to_string())],
                    ))
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
                            YamlParseError::new_err(format_i18n_error(
                                "yaml-parse-error",
                                &[("detail", &e.to_string())],
                            ))
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
