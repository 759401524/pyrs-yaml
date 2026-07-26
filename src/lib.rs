//! # pyyaml-rs
//!
//! A high-performance Python YAML library with perfect round-trip support.

pub mod ast;
pub mod i18n;
pub mod parser;
pub mod serializer;

#[cfg(test)]
mod integration;

// rust-i18n 初始化
rust_i18n::i18n!();

use ast::CustomNode;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::collections::HashSet;

// 自定义 Python 异常类型
pyo3::create_exception!(pyyaml_rs, YamlParseError, pyo3::exceptions::PyValueError);
pyo3::create_exception!(
    pyyaml_rs,
    YamlSerializeError,
    pyo3::exceptions::PyValueError
);
pyo3::create_exception!(pyyaml_rs, YamlTypeError, pyo3::exceptions::PyTypeError);

/// A Python module implemented in Rust.
///
/// pyyaml-rs: high-performance YAML parsing with perfect round-trip support.
#[pymodule]
mod pyyaml_rs {
    use super::*;

    #[pymodule_export]
    use super::YamlParseError;
    #[pymodule_export]
    use super::YamlSerializeError;
    #[pymodule_export]
    use super::YamlTypeError;

    /// 格式化 i18n 错误消息，将翻译模板中的占位符替换为实际值。
    fn format_i18n_error(key: &str, args: &[(&str, &str)]) -> String {
        super::i18n::format_message(key, args)
    }

    /// Python wrapper for the parsed YAML document.
    #[pyclass]
    struct YamlDocument {
        ast: CustomNode,
    }

    #[pymethods]
    impl YamlDocument {
        /// 将文档序列化为 YAML 字符串（默认 2 空格缩进）。
        ///
        /// # Returns
        /// 完整的 YAML 文档字符串，末尾包含换行符。
        fn to_yaml(&self) -> String {
            super::serializer::to_yaml(&self.ast)
        }

        /// 使用自定义选项将文档序列化为 YAML 字符串。
        ///
        /// # Arguments
        /// * `indent_size` - 每层缩进的空格数，默认 2。
        /// * `explicit_start` - 是否在文档开头添加 `---`。
        /// * `explicit_end` - 是否在文档末尾添加 `...`。
        /// * `sort_keys` - 是否按键名排序（不保留原始顺序）。
        ///
        /// # Returns
        /// 完整的 YAML 文档字符串。
        #[pyo3(signature = (indent_size: "int" = 2, explicit_start: "bool" = false, explicit_end: "bool" = false, sort_keys: "bool" = false) -> "str")]
        fn to_yaml_with_options(
            &self,
            indent_size: usize,
            explicit_start: bool,
            explicit_end: bool,
            sort_keys: bool,
        ) -> String {
            let options = super::serializer::SerializeOptions {
                indent_size,
                explicit_start,
                explicit_end,
                sort_keys,
            };
            super::serializer::to_yaml_with_options(&self.ast, &options)
        }

        /// 将文档转换为 Python 字典/列表，自动解析锚点引用。
        ///
        /// 锚点（`&name`）指向的节点会被内联展开，别名（`*name`）会被替换为实际值。
        /// 标量值会根据 YAML 1.2 类型规则自动转换为 Python 原生类型（bool/int/float/str/None）。
        fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
            let mut anchors = HashMap::new();
            collect_anchors(&self.ast, &mut anchors);
            let mut visited = HashSet::new();
            node_to_pyobject_with_anchors(&self.ast, py, &anchors, &mut visited)
        }

        /// 获取映射中指定键的值。
        ///
        /// # Arguments
        /// * `key` - 要查找的键名（字符串）。
        /// * `default` - 键不存在时返回的默认值，默认为 `None`。
        ///
        /// # Returns
        /// 键对应的值，或 `default`（若未提供则返回 `None`）。
        /// 如果根节点不是映射，也返回 `default`。
        #[pyo3(signature = (key: "str", default: "Any" = None) -> "Any")]
        fn get(&self, py: Python, key: &str, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    let key_node = CustomNode::plain_scalar(key);
                    if let Some(value) = pairs.get(&key_node) {
                        Ok(node_to_pyobject(value, py)?)
                    } else {
                        Ok(default.unwrap_or_else(|| py.None()))
                    }
                }
                _ => Ok(default.unwrap_or_else(|| py.None())),
            }
        }

        /// 返回文档根节点的类型名称。
        ///
        /// # Returns
        /// 可能的返回值：`"scalar"`, `"mapping"`, `"sequence"`, `"null"`, `"alias"`。
        fn root_type(&self) -> String {
            match &self.ast {
                CustomNode::Scalar { .. } => "scalar".to_string(),
                CustomNode::Mapping { .. } => "mapping".to_string(),
                CustomNode::Sequence { .. } => "sequence".to_string(),
                CustomNode::Null { .. } => "null".to_string(),
                CustomNode::Alias { .. } => "alias".to_string(),
            }
        }

        /// 返回调试表示字符串，格式为 `YamlDocument(<yaml>)`。
        fn __repr__(&self) -> String {
            format!("YamlDocument({})", self.to_yaml())
        }

        /// 返回 YAML 字符串表示。
        fn __str__(&self) -> String {
            self.to_yaml()
        }

        /// 检查映射是否包含指定键。
        ///
        /// # Arguments
        /// * `key` - 要检查的键名。
        ///
        /// # Returns
        /// 如果根节点是映射且包含该键返回 `true`，否则返回 `false`。
        fn __contains__(&self, key: &str) -> bool {
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    let key_node = CustomNode::plain_scalar(key);
                    pairs.contains_key(&key_node)
                }
                _ => false,
            }
        }

        /// 返回映射的键值对数量或序列的元素数量。
        ///
        /// # Returns
        /// 对于映射和序列返回元素数，其他类型返回 0。
        fn __len__(&self) -> usize {
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => pairs.len(),
                CustomNode::Sequence { items, .. } => items.len(),
                _ => 0,
            }
        }

        /// 迭代文档内容。映射迭代键，序列迭代值，其他类型返回空列表。
        fn __iter__<'py>(&self, _py: Python<'py>) -> PyResult<Py<PyAny>> {
            Python::attach(|py| match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    let keys: Vec<Py<PyAny>> = pairs
                        .keys()
                        .map(|k| node_to_pyobject(k, py))
                        .collect::<PyResult<Vec<_>>>()?;
                    Ok(keys.into_pyobject(py)?.into_any().unbind())
                }
                CustomNode::Sequence { items, .. } => {
                    let values: Vec<Py<PyAny>> = items
                        .iter()
                        .map(|v| node_to_pyobject(v, py))
                        .collect::<PyResult<Vec<_>>>()?;
                    Ok(values.into_pyobject(py)?.into_any().unbind())
                }
                _ => Ok(Vec::<Py<PyAny>>::new()
                    .into_pyobject(py)?
                    .into_any()
                    .unbind()),
            })
        }

        /// 通过下标访问文档内容。
        ///
        /// # Arguments
        /// * `key` - 映射使用字符串键，序列使用整数索引。
        ///
        /// # Errors
        /// 返回 `KeyError`（映射键不存在）、`IndexError`（索引越界）或 `TypeError`（类型不支持下标访问）。
        fn __getitem__<'py>(&self, py: Python<'py>, key: Py<PyAny>) -> PyResult<Py<PyAny>> {
            match &self.ast {
                CustomNode::Mapping { pairs, .. } => {
                    if let Ok(key_str) = key.bind(py).extract::<String>() {
                        let key_node = CustomNode::plain_scalar(key_str.clone());
                        if let Some(value) = pairs.get(&key_node) {
                            Ok(node_to_pyobject(value, py)?)
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
                            Ok(node_to_pyobject(&items[idx], py)?)
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
    }

    // ---- helper functions (not exposed to Python) ----

    /// 递归遍历 AST，收集所有锚点到节点的映射，用于别名解析。
    fn collect_anchors<'a>(node: &'a CustomNode, anchors: &mut HashMap<String, &'a CustomNode>) {
        if let Some(name) = node.anchor() {
            anchors.insert(name.to_string(), node);
        }
        match node {
            CustomNode::Mapping { pairs, .. } => {
                for (key, value) in pairs {
                    collect_anchors(key, anchors);
                    collect_anchors(value, anchors);
                }
            }
            CustomNode::Sequence { items, .. } => {
                for item in items {
                    collect_anchors(item, anchors);
                }
            }
            _ => {}
        }
    }

    /// 将 `CustomNode` 转换为 Python 对象，解析别名引用（`*alias`）为实际值。
    fn node_to_pyobject_with_anchors(
        node: &CustomNode,
        py: Python,
        anchors: &HashMap<String, &CustomNode>,
        visited: &mut HashSet<usize>,
    ) -> PyResult<Py<PyAny>> {
        match node {
            CustomNode::Alias { name } => {
                if let Some(target) = anchors.get(name) {
                    let addr = std::ptr::addr_of!(*target) as usize;
                    if visited.contains(&addr) {
                        return Ok(py.None());
                    }
                    visited.insert(addr);
                    node_to_pyobject_with_anchors(target, py, anchors, visited)
                } else {
                    Ok(py.None())
                }
            }
            _ => node_to_pyobject_inner(node, py, anchors, visited),
        }
    }

    /// 将 `CustomNode` 转换为 Python 对象，不解析别名（别名节点返回 `None`）。
    fn node_to_pyobject(node: &CustomNode, py: Python) -> PyResult<Py<PyAny>> {
        let anchors = HashMap::new();
        let mut visited = HashSet::new();
        node_to_pyobject_inner(node, py, &anchors, &mut visited)
    }

    /// 内部转换逻辑，被 `node_to_pyobject` 和 `node_to_pyobject_with_anchors` 共用。
    fn node_to_pyobject_inner(
        node: &CustomNode,
        py: Python,
        anchors: &HashMap<String, &CustomNode>,
        visited: &mut HashSet<usize>,
    ) -> PyResult<Py<PyAny>> {
        match node {
            CustomNode::Scalar { value, style, .. } => match style {
                ast::ScalarStyle::Plain => {
                    use parser::yaml::{resolve_yaml_type, YamlType};
                    match resolve_yaml_type(value) {
                        YamlType::Null => Ok(py.None()),
                        YamlType::Bool(b) => Ok(pyo3::types::PyBool::new(py, b)
                            .to_owned()
                            .into_any()
                            .unbind()),
                        YamlType::Int(n) => Ok(n.into_pyobject(py)?.into_any().unbind()),
                        YamlType::Float(f) => Ok(f.into_pyobject(py)?.into_any().unbind()),
                        YamlType::Str(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
                    }
                }
                _ => Ok(value.clone().into_pyobject(py)?.into_any().unbind()),
            },
            CustomNode::Mapping { pairs, .. } => {
                let dict = PyDict::new(py);
                for (key, value) in pairs {
                    let key_str = match key {
                        CustomNode::Scalar { value, .. } => value.clone(),
                        _ => format!("{:?}", key),
                    };
                    let val = node_to_pyobject_with_anchors(value, py, anchors, visited)?;
                    dict.set_item(key_str, val).ok();
                }
                Ok(dict.into_any().unbind())
            }
            CustomNode::Sequence { items, .. } => {
                let list = pyo3::types::PyList::empty(py);
                for item in items {
                    let val = node_to_pyobject_with_anchors(item, py, anchors, visited)?;
                    list.append(val).ok();
                }
                Ok(list.into_any().unbind())
            }
            CustomNode::Null { .. } => Ok(py.None()),
            CustomNode::Alias { .. } => Ok(py.None()),
        }
    }

    /// 将 Python 对象递归转换为 `CustomNode` AST 节点，支持 dict/list/str/int/float/bool/None。
    fn pyobject_to_node(py: Python, obj: &Py<PyAny>) -> PyResult<CustomNode> {
        let obj = obj.bind(py);

        if obj.is_none() {
            return Ok(CustomNode::plain_null());
        }

        if let Ok(dict) = obj.cast::<PyDict>() {
            let mut pairs = IndexMap::new();
            for (key, value) in dict.iter() {
                let key_node = if let Ok(key_str) = key.extract::<String>() {
                    CustomNode::plain_scalar(key_str)
                } else {
                    pyobject_to_node(py, &key.into_any().unbind())?
                };
                let value_node = pyobject_to_node(py, &value.into_any().unbind())?;
                pairs.insert(key_node, value_node);
            }
            return Ok(CustomNode::plain_mapping(pairs));
        }

        if let Ok(list) = obj.cast::<pyo3::types::PyList>() {
            let items: Vec<CustomNode> = list
                .iter()
                .map(|item| pyobject_to_node(py, &item.into_any().unbind()))
                .collect::<Result<_, _>>()?;
            return Ok(CustomNode::plain_sequence(items));
        }

        if let Ok(b) = obj.extract::<bool>() {
            return Ok(CustomNode::plain_scalar(if b { "true" } else { "false" }));
        }

        if let Ok(n) = obj.extract::<i64>() {
            return Ok(CustomNode::plain_scalar(n.to_string()));
        }

        if let Ok(f) = obj.extract::<f64>() {
            return Ok(CustomNode::plain_scalar(f.to_string()));
        }

        if let Ok(s) = obj.extract::<String>() {
            return Ok(CustomNode::plain_scalar(s));
        }

        Err(YamlTypeError::new_err(format_i18n_error(
            "unsupported-type",
            &[],
        )))
    }

    /// 将 `serde_json::Value` 转换为 `CustomNode` AST 节点。
    fn json_value_to_node(value: &serde_json::Value) -> PyResult<CustomNode> {
        match value {
            serde_json::Value::Null => Ok(CustomNode::plain_null()),
            serde_json::Value::Bool(b) => {
                Ok(CustomNode::plain_scalar(if *b { "true" } else { "false" }))
            }
            serde_json::Value::Number(n) => {
                let s = if let Some(i) = n.as_i64() {
                    i.to_string()
                } else if let Some(f) = n.as_f64() {
                    f.to_string()
                } else {
                    n.to_string()
                };
                Ok(CustomNode::plain_scalar(s))
            }
            serde_json::Value::String(s) => Ok(CustomNode::plain_scalar(s.clone())),
            serde_json::Value::Array(arr) => {
                let items: Vec<CustomNode> = arr
                    .iter()
                    .map(json_value_to_node)
                    .collect::<Result<_, _>>()?;
                Ok(CustomNode::plain_sequence(items))
            }
            serde_json::Value::Object(map) => {
                let mut pairs = IndexMap::new();
                for (key, value) in map {
                    let key_node = CustomNode::plain_scalar(key.clone());
                    let value_node = json_value_to_node(value)?;
                    pairs.insert(key_node, value_node);
                }
                Ok(CustomNode::plain_mapping(pairs))
            }
        }
    }

    // ---- Python-facing functions ----

    /// 解析 YAML 字符串或字节流，返回 `YamlDocument` 对象。
    #[pyfunction]
    #[pyo3(signature = (yaml: "str | bytes", resolve_merges: "bool" = true) -> "YamlDocument")]
    fn parse(py: Python, yaml: &Bound<'_, PyAny>, resolve_merges: bool) -> PyResult<YamlDocument> {
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

        let ast = py.detach(|| {
            super::parser::parse_with_options(&yaml_str, resolve_merges).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        Ok(YamlDocument { ast })
    }

    /// 解析 YAML 文件，返回 `YamlDocument` 对象。
    #[pyfunction]
    #[pyo3(signature = (path: "str") -> "YamlDocument")]
    fn parse_file(py: Python, path: &str) -> PyResult<YamlDocument> {
        let ast = py.detach(|| {
            let content = std::fs::read_to_string(path).map_err(|e| {
                pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                    "file-read-error",
                    &[("detail", &e.to_string()), ("path", path)],
                ))
            })?;
            super::parser::parse_with_options(&content, true).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        Ok(YamlDocument { ast })
    }

    /// 解析包含多个 YAML 文档的字符串（以 `---` 分隔）。
    #[pyfunction]
    #[pyo3(signature = (yaml: "str", resolve_merges: "bool" = true) -> "list[YamlDocument]")]
    fn parse_all_docs(py: Python, yaml: &str, resolve_merges: bool) -> PyResult<Vec<YamlDocument>> {
        let asts = py.detach(|| {
            super::parser::parse_all_with_options(yaml, resolve_merges).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        Ok(asts.into_iter().map(|ast| YamlDocument { ast }).collect())
    }

    /// PyYAML 兼容接口：解析 YAML 字符串并返回原生 Python 对象。
    #[pyfunction]
    #[pyo3(signature = (yaml: "str") -> "dict[str, Any] | list[Any]")]
    fn safe_load(py: Python, yaml: &str) -> PyResult<Py<PyAny>> {
        let ast = py.detach(|| {
            super::parser::parse(yaml).map_err(|e| {
                YamlParseError::new_err(format_i18n_error(
                    "yaml-parse-error",
                    &[("detail", &e.to_string())],
                ))
            })
        })?;
        let mut anchors = HashMap::new();
        collect_anchors(&ast, &mut anchors);
        let mut visited = HashSet::new();
        node_to_pyobject_with_anchors(&ast, py, &anchors, &mut visited)
    }

    /// PyYAML 兼容接口：解析多个 YAML 文档并返回原生 Python 对象列表。
    #[pyfunction]
    #[pyo3(signature = (yaml: "str") -> "list[dict[str, Any] | list[Any]]")]
    fn safe_loads(py: Python, yaml: &str) -> PyResult<Vec<Py<PyAny>>> {
        let asts = py.detach(|| {
            super::parser::parse_all(yaml).map_err(|e| {
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
                node_to_pyobject_with_anchors(ast, py, &anchors, &mut visited)
            })
            .collect()
    }

    /// PyYAML 兼容接口：将 Python 对象序列化为 YAML 字符串。
    #[pyfunction]
    #[pyo3(signature = (data: "dict[str, Any] | list[Any]") -> "str")]
    fn safe_dump(py: Python, data: Py<PyAny>) -> PyResult<String> {
        let node = pyobject_to_node(py, &data)?;
        Ok(super::serializer::to_yaml(&node))
    }

    /// PyYAML 兼容接口：将 Python 对象序列化为 YAML 字符串（`safe_dump` 的别名）。
    #[pyfunction]
    #[pyo3(signature = (data: "dict[str, Any] | list[Any]") -> "str")]
    fn safe_dumps(py: Python, data: Py<PyAny>) -> PyResult<String> {
        safe_dump(py, data)
    }

    /// 将 Python 字典/列表转换为 YAML 字符串。
    #[pyfunction]
    #[pyo3(signature = (data: "dict[str, Any] | list[Any]") -> "str")]
    fn from_dict(py: Python, data: Py<PyAny>) -> PyResult<String> {
        let node = pyobject_to_node(py, &data)?;
        Ok(super::serializer::to_yaml(&node))
    }

    /// 将 JSON 字符串转换为 YAML 字符串。
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
        Ok(super::serializer::to_yaml(&node))
    }

    /// 将 Python 对象序列化为 YAML 并写入文件。
    #[pyfunction]
    #[pyo3(signature = (data: "Any", path: "str") -> "None")]
    fn dump_file(py: Python, data: Py<PyAny>, path: &str) -> PyResult<()> {
        let node = pyobject_to_node(py, &data)?;
        let yaml = super::serializer::to_yaml(&node);
        std::fs::write(path, yaml).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                "file-write-error",
                &[("detail", &e.to_string()), ("path", path)],
            ))
        })?;
        Ok(())
    }

    /// 从 Markdown 文件中读取 YAML frontmatter。
    #[pyfunction]
    #[pyo3(signature = (path: "str") -> "tuple[dict[str, Any] | None, str]")]
    fn read_markdown(py: Python, path: &str) -> PyResult<(Option<Py<PyAny>>, String)> {
        let content = py.detach(|| {
            std::fs::read_to_string(path).map_err(|e| {
                pyo3::exceptions::PyIOError::new_err(format_i18n_error(
                    "file-read-error",
                    &[("detail", &e.to_string()), ("path", path)],
                ))
            })
        })?;
        read_markdown_str(py, &content)
    }

    /// 从 Markdown 字符串中读取 YAML frontmatter。
    #[pyfunction]
    #[pyo3(signature = (content: "str") -> "tuple[dict[str, Any] | None, str]")]
    fn read_markdown_str(_py: Python, content: &str) -> PyResult<(Option<Py<PyAny>>, String)> {
        let content = content.trim_start();

        if let Some(rest) = content.strip_prefix("---") {
            if let Some(end_idx) = rest.find("---") {
                let frontmatter = rest[..end_idx].trim();
                let markdown_content = rest[end_idx + 3..].trim();

                if !frontmatter.is_empty() {
                    return Python::attach(|py| {
                        let ast = super::parser::parse(frontmatter).map_err(|e| {
                            YamlParseError::new_err(format_i18n_error(
                                "yaml-parse-error",
                                &[("detail", &e.to_string())],
                            ))
                        })?;
                        Ok((
                            Some(node_to_pyobject(&ast, py)?),
                            markdown_content.to_string(),
                        ))
                    });
                }
            }
        }

        Ok((None, content.to_string()))
    }

    /// 设置错误消息语言。
    #[pyfunction]
    #[pyo3(signature = (lang: "str") -> "None")]
    fn set_language(lang: &str) -> PyResult<()> {
        i18n::set_language(lang).map_err(|_| {
            pyo3::exceptions::PyValueError::new_err(format_i18n_error(
                "unsupported-language",
                &[
                    ("lang", lang),
                    ("supported", &format!("{:?}", i18n::SUPPORTED_LANGUAGES)),
                ],
            ))
        })
    }

    /// 获取当前错误消息语言。
    #[pyfunction]
    #[pyo3(signature = () -> "str")]
    fn get_language() -> &'static str {
        i18n::get_language_static()
    }

    /// 列出所有支持的语言。
    #[pyfunction]
    #[pyo3(signature = () -> "list[str]")]
    fn list_languages() -> Vec<&'static str> {
        i18n::list_languages()
    }

    /// 自动检测用户偏好语言（从环境变量）。
    #[pyfunction]
    #[pyo3(signature = () -> "str")]
    fn detect_language() -> String {
        i18n::detect_language()
    }

    /// BCP 47 语言协商：从用户提供的语言列表中匹配最合适的支持语言。
    #[pyfunction]
    #[pyo3(signature = (user_locales: "list[str]", default: "str" = "en") -> "str")]
    fn negotiate_language(user_locales: &Bound<'_, PyAny>, default: &str) -> PyResult<String> {
        let locales: Vec<String> = user_locales.extract()?;
        let refs: Vec<&str> = locales.iter().map(|s| s.as_str()).collect();
        Ok(i18n::negotiate_language(&refs, default).to_string())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_and_serialize() {
        let yaml = "key: value";
        let ast = super::parser::parse(yaml).unwrap();
        let output = super::serializer::to_yaml(&ast);
        assert_eq!(output, "key: value\n");
    }
}
