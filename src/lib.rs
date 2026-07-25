//! # pyyaml-rs
//!
//! A high-performance Python YAML library with perfect round-trip support.

pub mod ast;
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

/// Python wrapper for the parsed YAML document.
#[pyclass]
struct YamlDocument {
    ast: CustomNode,
}

#[pymethods]
impl YamlDocument {
    fn to_yaml(&self) -> String {
        serializer::to_yaml(&self.ast)
    }

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

    fn to_dict(&self, py: Python) -> Py<PyAny> {
        node_to_pyobject(&self.ast, py)
    }

    #[pyo3(signature = (key, default=None))]
    fn get(&self, py: Python, key: &str, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        match &self.ast {
            CustomNode::Mapping { pairs, .. } => {
                let key_node = CustomNode::Scalar {
                    value: key.to_string(),
                    style: ast::ScalarStyle::Plain,
                    comment: None,
                    anchor: None,
                    tag: None,
                    chomping: ast::Chomping::Clip,
                };
                if let Some(value) = pairs.get(&key_node) {
                    Ok(node_to_pyobject(value, py))
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
                let key_node = CustomNode::Scalar {
                    value: key.to_string(),
                    style: ast::ScalarStyle::Plain,
                    comment: None,
                    anchor: None,
                    tag: None,
                    chomping: ast::Chomping::Clip,
                };
                pairs.contains_key(&key_node)
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
        Python::attach(|py| match &self.ast {
            CustomNode::Mapping { pairs, .. } => {
                let keys: Vec<Py<PyAny>> = pairs.keys().map(|k| node_to_pyobject(k, py)).collect();
                Ok(keys.into_pyobject(py)?.into_any().unbind())
            }
            CustomNode::Sequence { items, .. } => {
                let values: Vec<Py<PyAny>> = items.iter().map(|v| node_to_pyobject(v, py)).collect();
                Ok(values.into_pyobject(py)?.into_any().unbind())
            }
            _ => Ok(Vec::<Py<PyAny>>::new().into_pyobject(py)?.into_any().unbind()),
        })
    }

    fn __getitem__<'py>(&self, py: Python<'py>, key: Py<PyAny>) -> PyResult<Py<PyAny>> {
        match &self.ast {
            CustomNode::Mapping { pairs, .. } => {
                if let Ok(key_str) = key.bind(py).extract::<String>() {
                    let key_node = CustomNode::Scalar {
                        value: key_str,
                        style: ast::ScalarStyle::Plain,
                        comment: None,
                        anchor: None,
                        tag: None,
                        chomping: ast::Chomping::Clip,
                    };
                    if let Some(value) = pairs.get(&key_node) {
                        Ok(node_to_pyobject(value, py))
                    } else {
                        Err(pyo3::exceptions::PyKeyError::new_err("Key not found"))
                    }
                } else {
                    Err(pyo3::exceptions::PyTypeError::new_err("Key must be a string"))
                }
            }
            CustomNode::Sequence { items, .. } => {
                if let Ok(idx) = key.bind(py).extract::<usize>() {
                    if idx < items.len() {
                        Ok(node_to_pyobject(&items[idx], py))
                    } else {
                        Err(pyo3::exceptions::PyIndexError::new_err("Index out of range"))
                    }
                } else {
                    Err(pyo3::exceptions::PyTypeError::new_err("Index must be an integer"))
                }
            }
            _ => Err(pyo3::exceptions::PyTypeError::new_err("YamlDocument is not subscriptable")),
        }
    }
}

/// Parse a YAML string or bytes into a YamlDocument.
#[pyfunction]
#[pyo3(signature = (yaml, resolve_merges=true))]
fn parse(py: Python, yaml: &Bound<'_, pyo3::types::PyAny>, resolve_merges: bool) -> PyResult<YamlDocument> {
    let yaml_str: String = if let Ok(s) = yaml.extract::<String>() {
        s
    } else if let Ok(bytes) = yaml.extract::<Vec<u8>>() {
        String::from_utf8(bytes)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid UTF-8: {}", e)))?
    } else {
        return Err(pyo3::exceptions::PyTypeError::new_err("Expected str or bytes"));
    };

    let ast = py.detach(|| {
        parser::parse_with_options(&yaml_str, resolve_merges).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("YAML parse error: {}", e))
        })
    })?;
    Ok(YamlDocument { ast })
}

/// Parse a YAML file.
#[pyfunction]
fn parse_file(py: Python, path: &str) -> PyResult<YamlDocument> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    let ast = py.detach(|| {
        parser::parse_with_options(&content, true).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("YAML parse error: {}", e))
        })
    })?;
    Ok(YamlDocument { ast })
}

/// Convert a CustomNode to a Python object.
fn node_to_pyobject(node: &CustomNode, py: Python) -> Py<PyAny> {
    match node {
        CustomNode::Scalar { value, style, .. } => {
            match style {
                ast::ScalarStyle::Plain => {
                    use parser::yaml::{resolve_yaml_type, YamlType};
                    match resolve_yaml_type(value) {
                        YamlType::Null => py.None(),
                        YamlType::Bool(b) => {
                            // Convert bool to PyObject directly
                            pyo3::types::PyBool::new(py, b).to_owned().into_any().unbind()
                        }
                        YamlType::Int(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
                        YamlType::Float(f) => f.into_pyobject(py).unwrap().into_any().unbind(),
                        YamlType::Str(s) => s.into_pyobject(py).unwrap().into_any().unbind(),
                    }
                }
                _ => value.clone().into_pyobject(py).unwrap().into_any().unbind(),
            }
        }
        CustomNode::Mapping { pairs, .. } => {
            let dict = PyDict::new(py);
            for (key, value) in pairs {
                let key_str = match key {
                    CustomNode::Scalar { value, .. } => value.clone(),
                    _ => format!("{:?}", key),
                };
                dict.set_item(key_str, node_to_pyobject(value, py)).ok();
            }
            dict.into_any().unbind()
        }
        CustomNode::Sequence { items, .. } => {
            let list = pyo3::types::PyList::empty(py);
            for item in items {
                list.append(node_to_pyobject(item, py)).ok();
            }
            list.into_any().unbind()
        }
        CustomNode::Null { .. } => py.None(),
        CustomNode::Alias { .. } => {
            // Aliases cannot be resolved in to_dict() without anchor context
            // Return None as a safe default
            py.None()
        }
    }
}

/// Parse multiple YAML documents from a string.
#[pyfunction]
fn parse_all_docs(py: Python, yaml: &str) -> PyResult<Vec<YamlDocument>> {
    let asts = py.detach(|| {
        parser::parse_all(yaml).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("YAML parse error: {}", e))
        })
    })?;
    Ok(asts.into_iter().map(|ast| YamlDocument { ast }).collect())
}

/// Dump (serialize) a Python object to YAML and write to file.
#[pyfunction]
fn dump_file(py: Python, data: Py<PyAny>, path: &str) -> PyResult<()> {
    let node = pyobject_to_node(py, &data)?;
    let yaml = serializer::to_yaml(&node);
    std::fs::write(path, yaml)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    Ok(())
}

/// A Python module implemented in Rust.
#[pymodule]
fn pyyaml_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
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

/// PyYAML compatible: safe_load(stream) -> dict/list
#[pyfunction]
fn safe_load(py: Python, yaml: &str) -> PyResult<Py<PyAny>> {
    let ast = py.detach(|| {
        parser::parse(yaml).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("YAML parse error: {}", e))
        })
    })?;
    Ok(node_to_pyobject(&ast, py))
}

/// PyYAML compatible: safe_loads(stream) -> list of dict/list
#[pyfunction]
fn safe_loads(py: Python, yaml: &str) -> PyResult<Vec<Py<PyAny>>> {
    let asts = py.detach(|| {
        parser::parse_all(yaml).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("YAML parse error: {}", e))
        })
    })?;
    Ok(asts.iter().map(|ast| node_to_pyobject(ast, py)).collect())
}

/// PyYAML compatible: safe_dump(data) -> str
#[pyfunction]
fn safe_dump(py: Python, data: Py<PyAny>) -> PyResult<String> {
    let node = pyobject_to_node(py, &data)?;
    Ok(serializer::to_yaml(&node))
}

/// PyYAML compatible: safe_dumps(data) -> str
#[pyfunction]
fn safe_dumps(py: Python, data: Py<PyAny>) -> PyResult<String> {
    safe_dump(py, data)
}

/// Convert a Python dict to YAML string.
#[pyfunction]
fn from_dict(py: Python, data: Py<PyAny>) -> PyResult<String> {
    let node = pyobject_to_node(py, &data)?;
    Ok(serializer::to_yaml(&node))
}

/// Convert a JSON string to YAML string.
#[pyfunction]
fn from_json(_py: Python, json_str: &str) -> PyResult<String> {
    let json_value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("JSON parse error: {}", e)))?;
    let node = json_value_to_node(&json_value)?;
    Ok(serializer::to_yaml(&node))
}

fn json_value_to_node(value: &serde_json::Value) -> PyResult<CustomNode> {
    match value {
        serde_json::Value::Null => Ok(CustomNode::Null { comment: None, anchor: None, tag: None }),
        serde_json::Value::Bool(b) => {
            let s = if *b { "true".to_string() } else { "false".to_string() };
            Ok(CustomNode::Scalar { value: s, style: ast::ScalarStyle::Plain, comment: None, anchor: None, tag: None, chomping: ast::Chomping::Clip })
        }
        serde_json::Value::Number(n) => {
            let s = if let Some(i) = n.as_i64() { i.to_string() } else if let Some(f) = n.as_f64() { f.to_string() } else { n.to_string() };
            Ok(CustomNode::Scalar { value: s, style: ast::ScalarStyle::Plain, comment: None, anchor: None, tag: None, chomping: ast::Chomping::Clip })
        }
        serde_json::Value::String(s) => Ok(CustomNode::Scalar { value: s.clone(), style: ast::ScalarStyle::Plain, comment: None, anchor: None, tag: None, chomping: ast::Chomping::Clip }),
        serde_json::Value::Array(arr) => {
            let mut items = Vec::new();
            for item in arr { items.push(json_value_to_node(item)?); }
            Ok(CustomNode::Sequence { items, comment: None, anchor: None, tag: None })
        }
        serde_json::Value::Object(map) => {
            let mut pairs = IndexMap::new();
            for (key, value) in map {
                let key_node = CustomNode::Scalar { value: key.clone(), style: ast::ScalarStyle::Plain, comment: None, anchor: None, tag: None, chomping: ast::Chomping::Clip };
                let value_node = json_value_to_node(value)?;
                pairs.insert(key_node, value_node);
            }
            Ok(CustomNode::Mapping { pairs, comment: None, anchor: None, tag: None })
        }
    }
}

/// Read YAML frontmatter from a markdown file.
#[pyfunction]
fn read_markdown(py: Python, path: &str) -> PyResult<(Option<Py<PyAny>>, String)> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    read_markdown_str(py, &content)
}

/// Read YAML frontmatter from a markdown string.
#[pyfunction]
fn read_markdown_str(_py: Python, content: &str) -> PyResult<(Option<Py<PyAny>>, String)> {
    let content = content.trim_start();

    if let Some(rest) = content.strip_prefix("---") {
        if let Some(end_idx) = rest.find("---") {
            let frontmatter = rest[..end_idx].trim();
            let markdown_content = rest[end_idx + 3..].trim();

            if !frontmatter.is_empty() {
                return Python::attach(|py| {
                    let ast = parser::parse(frontmatter).map_err(|e| {
                        pyo3::exceptions::PyValueError::new_err(format!("YAML parse error: {}", e))
                    })?;
                    Ok((Some(node_to_pyobject(&ast, py)), markdown_content.to_string()))
                });
            }
        }
    }

    Ok((None, content.to_string()))
}

fn pyobject_to_node(py: Python, obj: &Py<PyAny>) -> PyResult<CustomNode> {
    let obj = obj.bind(py);

    if obj.is_none() {
        return Ok(CustomNode::Null { comment: None, anchor: None, tag: None });
    }

    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut pairs = IndexMap::new();
        for (key, value) in dict.iter() {
            let key_node = if let Ok(key_str) = key.extract::<String>() {
                CustomNode::Scalar { value: key_str, style: ast::ScalarStyle::Plain, comment: None, anchor: None, tag: None, chomping: ast::Chomping::Clip }
            } else {
                pyobject_to_node(py, &key.into_any().unbind())?
            };
            let value_node = pyobject_to_node(py, &value.into_any().unbind())?;
            pairs.insert(key_node, value_node);
        }
        return Ok(CustomNode::Mapping { pairs, comment: None, anchor: None, tag: None });
    }

    if let Ok(list) = obj.cast::<pyo3::types::PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(pyobject_to_node(py, &item.into_any().unbind())?);
        }
        return Ok(CustomNode::Sequence { items, comment: None, anchor: None, tag: None });
    }

    if let Ok(b) = obj.extract::<bool>() {
        let value = if b { "true".to_string() } else { "false".to_string() };
        return Ok(CustomNode::Scalar { value, style: ast::ScalarStyle::Plain, comment: None, anchor: None, tag: None, chomping: ast::Chomping::Clip });
    }

    if let Ok(n) = obj.extract::<i64>() {
        return Ok(CustomNode::Scalar { value: n.to_string(), style: ast::ScalarStyle::Plain, comment: None, anchor: None, tag: None, chomping: ast::Chomping::Clip });
    }

    if let Ok(f) = obj.extract::<f64>() {
        return Ok(CustomNode::Scalar { value: f.to_string(), style: ast::ScalarStyle::Plain, comment: None, anchor: None, tag: None, chomping: ast::Chomping::Clip });
    }

    if let Ok(s) = obj.extract::<String>() {
        return Ok(CustomNode::Scalar { value: s, style: ast::ScalarStyle::Plain, comment: None, anchor: None, tag: None, chomping: ast::Chomping::Clip });
    }

    Err(pyo3::exceptions::PyValueError::new_err("Unsupported Python type for YAML conversion"))
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
