//! # pyyaml-rs
//!
//! A high-performance Python YAML library with perfect round-trip support.

pub mod ast;
pub mod i18n;
pub mod parser;
pub mod serializer;

#[cfg(test)]
mod test_saphyr;
#[cfg(test)]
mod test_yaml_suite_saphyr;

use ast::CustomNode;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::collections::HashSet;

/// i18n 辅助函数：格式化错误消息
fn msg(key: &str, args: &[(&str, &str)]) -> String {
    i18n::format_message(key, args)
}

// 自定义 Python 异常类型
pyo3::create_exception!(pyyaml_rs, YamlParseError, pyo3::exceptions::PyValueError);
pyo3::create_exception!(pyyaml_rs, YamlSerializeError, pyo3::exceptions::PyValueError);
pyo3::create_exception!(pyyaml_rs, YamlTypeError, pyo3::exceptions::PyTypeError);

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
        serializer::to_yaml(&self.ast)
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
    #[pyo3(signature = (indent_size=2, explicit_start=false, explicit_end=false, sort_keys=false))]
    fn to_yaml_with_options(&self, indent_size: usize, explicit_start: bool, explicit_end: bool, sort_keys: bool) -> String {
        let options = serializer::SerializeOptions {
            indent_size,
            explicit_start,
            explicit_end,
            sort_keys,
        };
        serializer::to_yaml_with_options(&self.ast, &options)
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
    #[pyo3(signature = (key, default=None))]
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
                let keys: Vec<Py<PyAny>> = pairs.keys().map(|k| node_to_pyobject(k, py)).collect::<PyResult<Vec<_>>>()?;
                Ok(keys.into_pyobject(py)?.into_any().unbind())
            }
            CustomNode::Sequence { items, .. } => {
                let values: Vec<Py<PyAny>> = items.iter().map(|v| node_to_pyobject(v, py)).collect::<PyResult<Vec<_>>>()?;
                Ok(values.into_pyobject(py)?.into_any().unbind())
            }
            _ => Ok(Vec::<Py<PyAny>>::new().into_pyobject(py)?.into_any().unbind()),
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
                        Err(pyo3::exceptions::PyKeyError::new_err(msg("key-not-found", &[("key", key_str.as_str())])))
                    }
                } else {
                    Err(YamlTypeError::new_err(msg("key-not-string", &[])))
                }
            }
            CustomNode::Sequence { items, .. } => {
                if let Ok(idx) = key.bind(py).extract::<usize>() {
                    if idx < items.len() {
                        Ok(node_to_pyobject(&items[idx], py)?)
                    } else {
                        Err(pyo3::exceptions::PyIndexError::new_err(msg("index-out-of-range", &[("index", &idx.to_string()), ("len", &items.len().to_string())])))
                    }
                } else {
                    Err(YamlTypeError::new_err(msg("index-not-integer", &[])))
                }
            }
            _ => Err(YamlTypeError::new_err(msg("not-subscriptable", &[]))),
        }
    }
}

/// 解析 YAML 字符串或字节流，返回 `YamlDocument` 对象。
///
/// # Arguments
/// * `yaml` - YAML 内容，可以是 `str` 或 `bytes`（必须为有效 UTF-8）。
/// * `resolve_merges` - 是否解析合并键（`<<`），默认为 `true`。
///
/// # Returns
/// 解析后的 `YamlDocument` 对象，可进行序列化和类型查询。
///
/// # Errors
/// 输入不是 `str`/`bytes` 时抛出 `TypeError`，
/// UTF-8 编码无效时抛出 `ValueError`，
/// YAML 语法错误时抛出 `ValueError`（包含行号和列号）。
///
/// # Examples
/// ```python
/// import pyyaml_rs
/// doc = pyyaml_rs.parse("key: value")
/// print(doc["key"])  # "value"
/// ```
/// Parse a YAML string or bytes into a YamlDocument.
#[pyfunction]
#[pyo3(signature = (yaml, resolve_merges=true))]
fn parse(py: Python, yaml: &Bound<'_, pyo3::types::PyAny>, resolve_merges: bool) -> PyResult<YamlDocument> {
    let yaml_str: String = if let Ok(s) = yaml.extract::<String>() {
        s
    } else if let Ok(bytes) = yaml.extract::<Vec<u8>>() {
        String::from_utf8(bytes)
            .map_err(|e| YamlParseError::new_err(msg("invalid-utf8", &[("detail", &e.to_string())])))?
    } else {
        return Err(YamlTypeError::new_err(msg("expected-str-or-bytes", &[])));
    };

    let ast = py.detach(|| {
        parser::parse_with_options(&yaml_str, resolve_merges).map_err(|e| {
            YamlParseError::new_err(msg("yaml-parse-error", &[("detail", &e.to_string())]))
        })
    })?;
    Ok(YamlDocument { ast })
}

/// 解析 YAML 文件，返回 `YamlDocument` 对象。
///
/// # Arguments
/// * `path` - YAML 文件的路径。
///
/// # Returns
/// 解析后的 `YamlDocument` 对象。
///
/// # Errors
/// 文件不存在或不可读时抛出 `IOError`，
/// YAML 语法错误时抛出 `ValueError`。
///
/// Parse a YAML file.
#[pyfunction]
#[pyo3(signature = (path))]
fn parse_file(py: Python, path: &str) -> PyResult<YamlDocument> {
    let ast = py.detach(|| {
        let content = std::fs::read_to_string(path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        parser::parse_with_options(&content, true).map_err(|e| {
            YamlParseError::new_err(msg("yaml-parse-error", &[("detail", &e.to_string())]))
        })
    })?;
    Ok(YamlDocument { ast })
}

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
        CustomNode::Scalar { value, style, .. } => {
            match style {
                ast::ScalarStyle::Plain => {
                    use parser::yaml::{resolve_yaml_type, YamlType};
                    match resolve_yaml_type(value) {
                        YamlType::Null => Ok(py.None()),
                        YamlType::Bool(b) => {
                            Ok(pyo3::types::PyBool::new(py, b).to_owned().into_any().unbind())
                        }
                        YamlType::Int(n) => Ok(n.into_pyobject(py)?.into_any().unbind()),
                        YamlType::Float(f) => Ok(f.into_pyobject(py)?.into_any().unbind()),
                        YamlType::Str(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
                    }
                }
                _ => Ok(value.clone().into_pyobject(py)?.into_any().unbind()),
            }
        }
        CustomNode::Mapping { pairs, .. } => {
            let dict = PyDict::new(py);
            for (key, value) in pairs {
                let key_str = match key {
                    CustomNode::Scalar { value, .. } => value.clone(),
                    _ => format!("{:?}", key),
                };
                // SAFETY: set_item only fails if key is unhashable or value type mismatch,
                // which cannot happen here since we control both key (String) and value (Py<PyAny>) types
                dict.set_item(key_str, node_to_pyobject_with_anchors(value, py, anchors, visited)?).ok();
            }
            Ok(dict.into_any().unbind())
        }
        CustomNode::Sequence { items, .. } => {
            let list = pyo3::types::PyList::empty(py);
            for item in items {
                // SAFETY: append only fails on type mismatch,
                // which cannot happen here since we control the value (Py<PyAny>) type
                list.append(node_to_pyobject_with_anchors(item, py, anchors, visited)?).ok();
            }
            Ok(list.into_any().unbind())
        }
        CustomNode::Null { .. } => Ok(py.None()),
        CustomNode::Alias { .. } => {
            // Should be handled by node_to_pyobject_with_anchors, but fallback
            Ok(py.None())
        }
    }
}

/// 解析包含多个 YAML 文档的字符串（以 `---` 分隔）。
///
/// # Arguments
/// * `yaml` - 包含一个或多个 YAML 文档的字符串。
///
/// # Returns
/// `YamlDocument` 对象列表，每个文档对应一个元素。
///
/// # Errors
/// YAML 语法错误时抛出 `ValueError`。
///
/// Parse multiple YAML documents from a string.
#[pyfunction]
#[pyo3(signature = (yaml))]
fn parse_all_docs(py: Python, yaml: &str) -> PyResult<Vec<YamlDocument>> {
    let asts = py.detach(|| {
        parser::parse_all(yaml).map_err(|e| {
            YamlParseError::new_err(msg("yaml-parse-error", &[("detail", &e.to_string())]))
        })
    })?;
    Ok(asts.into_iter().map(|ast| YamlDocument { ast }).collect())
}

/// 将 Python 对象序列化为 YAML 并写入文件。
///
/// # Arguments
/// * `data` - 要序列化的 Python 对象（dict/list/str/int/float/bool/None）。
/// * `path` - 输出文件路径。
///
/// # Errors
/// 不支持的 Python 类型时抛出 `ValueError`，
/// 文件写入失败时抛出 `IOError`。
///
/// Dump (serialize) a Python object to YAML and write to file.
#[pyfunction]
#[pyo3(signature = (data, path))]
fn dump_file(py: Python, data: Py<PyAny>, path: &str) -> PyResult<()> {
    let node = pyobject_to_node(py, &data)?;
    let yaml = serializer::to_yaml(&node);
    std::fs::write(path, yaml)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    Ok(())
}

/// 设置错误消息语言。
///
/// # Arguments
/// * `lang` - 语言代码，支持 "en" 和 "zh-CN"
///
/// # Errors
/// 返回 `PyValueError` 如果语言不受支持
#[pyfunction]
#[pyo3(signature = (lang))]
fn set_language(lang: &str) -> PyResult<()> {
    i18n::set_language(lang).map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(format!(
            "Unsupported language: {}. Supported: {:?}",
            lang,
            i18n::SUPPORTED_LANGUAGES
        ))
    })
}

/// 获取当前错误消息语言。
#[pyfunction]
#[pyo3(signature = ())]
fn get_language() -> &'static str {
    i18n::get_language()
}

/// A Python module implemented in Rust.
#[pymodule]
fn pyyaml_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // 注册自定义异常类型
    m.add("YamlParseError", m.py().get_type::<YamlParseError>())?;
    m.add("YamlSerializeError", m.py().get_type::<YamlSerializeError>())?;
    m.add("YamlTypeError", m.py().get_type::<YamlTypeError>())?;

    // 注册 i18n 函数
    m.add_function(wrap_pyfunction!(set_language, m)?)?;
    m.add_function(wrap_pyfunction!(get_language, m)?)?;

    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_file, m)?)?;
    m.add_function(wrap_pyfunction!(parse_all_docs, m)?)?;
    m.add_function(wrap_pyfunction!(safe_load, m)?)?;
    m.add_function(wrap_pyfunction!(safe_loads, m)?)?;
    m.add_function(wrap_pyfunction!(safe_dump, m)?)?;
    m.add_function(wrap_pyfunction!(safe_dumps, m)?)?;
    m.add_function(wrap_pyfunction!(from_dict, m)?)?;
    m.add_function(wrap_pyfunction!(from_json, m)?)?;
    m.add_function(wrap_pyfunction!(dump_file, m)?)?;
    m.add_function(wrap_pyfunction!(read_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(read_markdown_str, m)?)?;
    m.add_class::<YamlDocument>()?;
    Ok(())
}

/// PyYAML 兼容接口：解析 YAML 字符串并返回原生 Python 对象。
///
/// # Arguments
/// * `yaml` - YAML 内容字符串。
///
/// # Returns
/// 解析后的 Python 对象（dict/list/str/int/float/bool/None）。
///
/// # Errors
/// YAML 语法错误时抛出 `ValueError`。
///
/// PyYAML compatible: safe_load(stream) -> dict/list
#[pyfunction]
#[pyo3(signature = (yaml))]
fn safe_load(py: Python, yaml: &str) -> PyResult<Py<PyAny>> {
    let ast = py.detach(|| {
        parser::parse(yaml).map_err(|e| {
            YamlParseError::new_err(msg("yaml-parse-error", &[("detail", &e.to_string())]))
        })
    })?;
    let mut anchors = HashMap::new();
    collect_anchors(&ast, &mut anchors);
    let mut visited = HashSet::new();
    node_to_pyobject_with_anchors(&ast, py, &anchors, &mut visited)
}

/// PyYAML 兼容接口：解析多个 YAML 文档并返回原生 Python 对象列表。
///
/// # Arguments
/// * `yaml` - 包含一个或多个 YAML 文档的字符串。
///
/// # Returns
/// Python 对象列表，每个文档对应一个元素。
///
/// # Errors
/// YAML 语法错误时抛出 `ValueError`。
///
/// PyYAML compatible: safe_loads(stream) -> list of dict/list
#[pyfunction]
#[pyo3(signature = (yaml))]
fn safe_loads(py: Python, yaml: &str) -> PyResult<Vec<Py<PyAny>>> {
    let asts = py.detach(|| {
        parser::parse_all(yaml).map_err(|e| {
            YamlParseError::new_err(msg("yaml-parse-error", &[("detail", &e.to_string())]))
        })
    })?;
    asts.iter().map(|ast| {
        let mut anchors = HashMap::new();
        collect_anchors(ast, &mut anchors);
        let mut visited = HashSet::new();
        node_to_pyobject_with_anchors(ast, py, &anchors, &mut visited)
    }).collect()
}

/// PyYAML 兼容接口：将 Python 对象序列化为 YAML 字符串。
///
/// # Arguments
/// * `data` - 要序列化的 Python 对象（dict/list/str/int/float/bool/None）。
///
/// # Returns
/// 格式化的 YAML 字符串。
///
/// # Errors
/// 不支持的 Python 类型时抛出 `ValueError`。
///
/// PyYAML compatible: safe_dump(data) -> str
#[pyfunction]
#[pyo3(signature = (data))]
fn safe_dump(py: Python, data: Py<PyAny>) -> PyResult<String> {
    let node = pyobject_to_node(py, &data)?;
    Ok(serializer::to_yaml(&node))
}

/// PyYAML 兼容接口：将 Python 对象序列化为 YAML 字符串（`safe_dump` 的别名）。
///
/// # Arguments
/// * `data` - 要序列化的 Python 对象。
///
/// # Returns
/// 格式化的 YAML 字符串。
///
/// # Errors
/// 不支持的 Python 类型时抛出 `ValueError`。
///
/// PyYAML compatible: safe_dumps(data) -> str
#[pyfunction]
#[pyo3(signature = (data))]
fn safe_dumps(py: Python, data: Py<PyAny>) -> PyResult<String> {
    safe_dump(py, data)
}

/// 将 Python 字典/列表转换为 YAML 字符串。
///
/// # Arguments
/// * `data` - Python 对象（dict/list/str/int/float/bool/None）。
///
/// # Returns
/// 格式化的 YAML 字符串。
///
/// # Errors
/// 不支持的 Python 类型时抛出 `ValueError`。
///
/// Convert a Python dict to YAML string.
#[pyfunction]
#[pyo3(signature = (data))]
fn from_dict(py: Python, data: Py<PyAny>) -> PyResult<String> {
    let node = pyobject_to_node(py, &data)?;
    Ok(serializer::to_yaml(&node))
}

/// 将 JSON 字符串转换为 YAML 字符串。
///
/// # Arguments
/// * `json_str` - 合法的 JSON 字符串。
///
/// # Returns
/// 等价的 YAML 格式字符串。
///
/// # Errors
/// JSON 语法错误时抛出 `ValueError`。
///
/// Convert a JSON string to YAML string.
#[pyfunction]
#[pyo3(signature = (json_str))]
fn from_json(_py: Python, json_str: &str) -> PyResult<String> {
    let json_value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| YamlParseError::new_err(msg("json-parse-error", &[("detail", &e.to_string())])))?;
    let node = json_value_to_node(&json_value)?;
    Ok(serializer::to_yaml(&node))
}

/// 将 `serde_json::Value` 转换为 `CustomNode` AST 节点。
fn json_value_to_node(value: &serde_json::Value) -> PyResult<CustomNode> {
    match value {
        serde_json::Value::Null => Ok(CustomNode::plain_null()),
        serde_json::Value::Bool(b) => {
            Ok(CustomNode::plain_scalar(if *b { "true" } else { "false" }))
        }
        serde_json::Value::Number(n) => {
            let s = if let Some(i) = n.as_i64() { i.to_string() } else if let Some(f) = n.as_f64() { f.to_string() } else { n.to_string() };
            Ok(CustomNode::plain_scalar(s))
        }
        serde_json::Value::String(s) => Ok(CustomNode::plain_scalar(s.clone())),
        serde_json::Value::Array(arr) => {
            let items: Vec<CustomNode> = arr.iter().map(json_value_to_node).collect::<Result<_, _>>()?;
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

/// 从 Markdown 文件中读取 YAML frontmatter（`---` 包裹的部分）。
///
/// # Arguments
/// * `path` - Markdown 文件路径。
///
/// # Returns
/// `(Option[dict], str)` 元组：第一个元素是解析后的 frontmatter 字典（无 frontmatter 时为 `None`），
/// 第二个元素是去除 frontmatter 后的 Markdown 正文。
///
/// # Errors
/// 文件读取失败时抛出 `IOError`，YAML frontmatter 语法错误时抛出 `ValueError`。
///
/// Read YAML frontmatter from a markdown file.
#[pyfunction]
#[pyo3(signature = (path))]
fn read_markdown(py: Python, path: &str) -> PyResult<(Option<Py<PyAny>>, String)> {
    let content = py.detach(|| {
        std::fs::read_to_string(path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    })?;
    read_markdown_str(py, &content)
}

/// 从 Markdown 字符串中读取 YAML frontmatter（`---` 包裹的部分）。
///
/// # Arguments
/// * `content` - Markdown 内容字符串。
///
/// # Returns
/// `(Option[dict], str)` 元组：第一个元素是解析后的 frontmatter 字典（无 frontmatter 时为 `None`），
/// 第二个元素是去除 frontmatter 后的 Markdown 正文。
///
/// # Errors
/// YAML frontmatter 语法错误时抛出 `ValueError`。
///
/// Read YAML frontmatter from a markdown string.
#[pyfunction]
#[pyo3(signature = (content))]
fn read_markdown_str(_py: Python, content: &str) -> PyResult<(Option<Py<PyAny>>, String)> {
    let content = content.trim_start();

    if let Some(rest) = content.strip_prefix("---") {
        if let Some(end_idx) = rest.find("---") {
            let frontmatter = rest[..end_idx].trim();
            let markdown_content = rest[end_idx + 3..].trim();

            if !frontmatter.is_empty() {
                return Python::attach(|py| {
                    let ast = parser::parse(frontmatter).map_err(|e| {
                        YamlParseError::new_err(msg("yaml-parse-error", &[("detail", &e.to_string())]))
                    })?;
                    Ok((Some(node_to_pyobject(&ast, py)?), markdown_content.to_string()))
                });
            }
        }
    }

    Ok((None, content.to_string()))
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
        let items: Vec<CustomNode> = list.iter()
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

    Err(YamlTypeError::new_err(msg("unsupported-type", &[])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_serialize() {
        let yaml = "key: value";
        let ast = parser::parse(yaml).unwrap();
        let output = serializer::to_yaml(&ast);
        assert_eq!(output, "key: value\n");
    }
}
