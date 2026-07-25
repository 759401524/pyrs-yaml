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

/// Python wrapper for the parsed YAML document
#[pyclass]
struct YamlDocument {
    ast: CustomNode,
}

#[pymethods]
impl YamlDocument {
    /// Convert the AST back to YAML string
    fn to_yaml(&self) -> String {
        serializer::to_yaml(&self.ast)
    }

    /// Convert the AST to a Python dict/list
    fn to_dict(&self, _py: Python) -> PyObject {
        node_to_pyobject(&self.ast)
    }

    /// Get a value by key (for mapping root)
    fn get(&self, key: &str) -> PyResult<Option<PyObject>> {
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
                    Ok(Some(node_to_pyobject(value)))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    /// Get the root node type as string
    fn root_type(&self) -> String {
        match &self.ast {
            CustomNode::Scalar { .. } => "scalar".to_string(),
            CustomNode::Mapping { .. } => "mapping".to_string(),
            CustomNode::Sequence { .. } => "sequence".to_string(),
            CustomNode::Null { .. } => "null".to_string(),
            CustomNode::Alias { .. } => "alias".to_string(),
        }
    }
}

/// Parse a YAML string into a YamlDocument
#[pyfunction]
fn parse(py: Python, yaml: &str) -> PyResult<YamlDocument> {
    // Release GIL during parsing
    let ast = py.allow_threads(|| {
        parser::parse(yaml).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("YAML parse error: {}", e))
        })
    })?;

    Ok(YamlDocument { ast })
}

/// Parse a YAML file
#[pyfunction]
fn parse_file(py: Python, path: &str) -> PyResult<YamlDocument> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    parse(py, &content)
}

/// Convert a CustomNode to a Python object
fn node_to_pyobject(node: &CustomNode) -> PyObject {
    Python::with_gil(|py| match node {
        CustomNode::Scalar { value, style, .. } => {
            // Use YAML 1.2 type resolution for plain scalars
            match style {
                ast::ScalarStyle::Plain => {
                    use parser::yaml::resolve_yaml_type;
                    use parser::yaml::YamlType;
                    match resolve_yaml_type(value) {
                        YamlType::Null => py.None().into_py(py),
                        YamlType::Bool(b) => b.into_py(py),
                        YamlType::Int(n) => n.into_py(py),
                        YamlType::Float(f) => f.into_py(py),
                        YamlType::Str(s) => s.into_py(py),
                    }
                }
                _ => value.clone().into_py(py),
            }
        }
        CustomNode::Mapping { pairs, .. } => {
            let dict = pyo3::types::PyDict::new_bound(py);
            for (key, value) in pairs {
                let key_str = match key {
                    CustomNode::Scalar { value, .. } => value.clone(),
                    _ => format!("{:?}", key),
                };
                dict.set_item(key_str, node_to_pyobject(value)).ok();
            }
            dict.into_py(py)
        }
        CustomNode::Sequence { items, .. } => {
            let list = pyo3::types::PyList::empty_bound(py);
            for item in items {
                list.append(node_to_pyobject(item)).ok();
            }
            list.into_py(py)
        }
        CustomNode::Null { .. } => py.None().into_py(py),
        CustomNode::Alias { name } => {
            // For aliases, return a dict marker
            let dict = pyo3::types::PyDict::new_bound(py);
            dict.set_item("__alias__", name.clone()).ok();
            dict.into_py(py)
        }
    })
}

/// A Python module implemented in Rust.
#[pymodule]
fn pyamlium_custom(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_file, m)?)?;
    m.add_function(wrap_pyfunction!(safe_load, m)?)?;
    m.add_function(wrap_pyfunction!(safe_loads, m)?)?;
    m.add_function(wrap_pyfunction!(safe_dump, m)?)?;
    m.add_function(wrap_pyfunction!(safe_dumps, m)?)?;
    m.add_function(wrap_pyfunction!(from_dict, m)?)?;
    m.add_function(wrap_pyfunction!(from_json, m)?)?;
    m.add_function(wrap_pyfunction!(read_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(read_markdown_str, m)?)?;
    m.add_class::<YamlDocument>()?;
    Ok(())
}

/// PyYAML compatible: safe_load(stream) -> dict/list
/// Parse a YAML string and return Python dict/list
#[pyfunction]
fn safe_load(py: Python, yaml: &str) -> PyResult<PyObject> {
    let ast = py.allow_threads(|| {
        parser::parse(yaml).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("YAML parse error: {}", e))
        })
    })?;

    Ok(node_to_pyobject(&ast))
}

/// PyYAML compatible: safe_loads(stream) -> list of dict/list
/// Parse multiple YAML documents
#[pyfunction]
fn safe_loads(py: Python, yaml: &str) -> PyResult<Vec<PyObject>> {
    // Split by document separators and parse each
    let docs: Vec<&str> = yaml.split("---").collect();
    let mut results = Vec::new();

    for doc in docs {
        let doc = doc.trim();
        if doc.is_empty() {
            continue;
        }
        let ast = py.allow_threads(|| {
            parser::parse(doc).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("YAML parse error: {}", e))
            })
        })?;
        results.push(node_to_pyobject(&ast));
    }

    Ok(results)
}

/// PyYAML compatible: safe_dump(data) -> str
/// Serialize a Python dict/list to YAML string
#[pyfunction]
fn safe_dump(py: Python, data: PyObject) -> PyResult<String> {
    let node = pyobject_to_node(py, &data)?;
    Ok(serializer::to_yaml(&node))
}

/// PyYAML compatible: safe_dumps(data) -> str
/// Alias for safe_dump
#[pyfunction]
fn safe_dumps(py: Python, data: PyObject) -> PyResult<String> {
    safe_dump(py, data)
}

/// Convert a Python dict to YAML string (yamlium compatible)
#[pyfunction]
fn from_dict(py: Python, data: PyObject) -> PyResult<String> {
    let node = pyobject_to_node(py, &data)?;
    Ok(serializer::to_yaml(&node))
}

/// Convert a JSON string to YAML string (yamlium compatible)
#[pyfunction]
fn from_json(_py: Python, json_str: &str) -> PyResult<String> {
    let json_value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("JSON parse error: {}", e)))?;

    let node = json_value_to_node(&json_value)?;
    Ok(serializer::to_yaml(&node))
}

/// Convert a serde_json Value to CustomNode
fn json_value_to_node(value: &serde_json::Value) -> PyResult<CustomNode> {
    match value {
        serde_json::Value::Null => Ok(CustomNode::Null {
            comment: None,
            anchor: None,
            tag: None,
        }),
        serde_json::Value::Bool(b) => {
            let s = if *b {
                "true".to_string()
            } else {
                "false".to_string()
            };
            Ok(CustomNode::Scalar {
                value: s,
                style: ast::ScalarStyle::Plain,
                comment: None,
                anchor: None,
                tag: None,
                chomping: ast::Chomping::Clip,
            })
        }
        serde_json::Value::Number(n) => {
            let s = if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(f) = n.as_f64() {
                f.to_string()
            } else {
                n.to_string()
            };
            Ok(CustomNode::Scalar {
                value: s,
                style: ast::ScalarStyle::Plain,
                comment: None,
                anchor: None,
                tag: None,
                chomping: ast::Chomping::Clip,
            })
        }
        serde_json::Value::String(s) => Ok(CustomNode::Scalar {
            value: s.clone(),
            style: ast::ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: ast::Chomping::Clip,
        }),
        serde_json::Value::Array(arr) => {
            let mut items = Vec::new();
            for item in arr {
                items.push(json_value_to_node(item)?);
            }
            Ok(CustomNode::Sequence {
                items,
                comment: None,
                anchor: None,
                tag: None,
            })
        }
        serde_json::Value::Object(map) => {
            let mut pairs = IndexMap::new();
            for (key, value) in map {
                let key_node = CustomNode::Scalar {
                    value: key.clone(),
                    style: ast::ScalarStyle::Plain,
                    comment: None,
                    anchor: None,
                    tag: None,
                    chomping: ast::Chomping::Clip,
                };
                let value_node = json_value_to_node(value)?;
                pairs.insert(key_node, value_node);
            }
            Ok(CustomNode::Mapping {
                pairs,
                comment: None,
                anchor: None,
                tag: None,
            })
        }
    }
}

/// Read YAML frontmatter from a markdown file (yamlium compatible)
/// Returns (frontmatter_dict, content_string)
#[pyfunction]
fn read_markdown(py: Python, path: &str) -> PyResult<(Option<PyObject>, String)> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    read_markdown_str(py, &content)
}

/// Read YAML frontmatter from a markdown string
#[pyfunction]
fn read_markdown_str(_py: Python, content: &str) -> PyResult<(Option<PyObject>, String)> {
    let content = content.trim_start();

    // Check for --- frontmatter separator
    if let Some(rest) = content.strip_prefix("---") {
        if let Some(end_idx) = rest.find("---") {
            let frontmatter = rest[..end_idx].trim();
            let markdown_content = rest[end_idx + 3..].trim();

            // Parse the frontmatter as YAML
            if !frontmatter.is_empty() {
                let ast = parser::parse(frontmatter).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!("YAML parse error: {}", e))
                })?;
                return Ok((Some(node_to_pyobject(&ast)), markdown_content.to_string()));
            }
        }
    }

    // No frontmatter found
    Ok((None, content.to_string()))
}

/// Convert a Python object to a CustomNode
fn pyobject_to_node(py: Python, obj: &PyObject) -> PyResult<CustomNode> {
    let obj = obj.bind(py);

    if obj.is_none() {
        return Ok(CustomNode::Null {
            comment: None,
            anchor: None,
            tag: None,
        });
    }

    // Try as dict
    if let Ok(dict) = obj.downcast::<pyo3::types::PyDict>() {
        let mut pairs = IndexMap::new();
        for (key, value) in dict.iter() {
            let key_node = if let Ok(key_str) = key.extract::<String>() {
                CustomNode::Scalar {
                    value: key_str,
                    style: ast::ScalarStyle::Plain,
                    comment: None,
                    anchor: None,
                    tag: None,
                    chomping: ast::Chomping::Clip,
                }
            } else {
                pyobject_to_node(py, &key.into_py(py))?
            };
            let value_node = pyobject_to_node(py, &value.into_py(py))?;
            pairs.insert(key_node, value_node);
        }
        return Ok(CustomNode::Mapping {
            pairs,
            comment: None,
            anchor: None,
            tag: None,
        });
    }

    // Try as list
    if let Ok(list) = obj.downcast::<pyo3::types::PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(pyobject_to_node(py, &item.into_py(py))?);
        }
        return Ok(CustomNode::Sequence {
            items,
            comment: None,
            anchor: None,
            tag: None,
        });
    }

    // Try as bool (must be before int/float check)
    if let Ok(b) = obj.extract::<bool>() {
        let value = if b {
            "true".to_string()
        } else {
            "false".to_string()
        };
        return Ok(CustomNode::Scalar {
            value,
            style: ast::ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: ast::Chomping::Clip,
        });
    }

    // Try as int
    if let Ok(n) = obj.extract::<i64>() {
        return Ok(CustomNode::Scalar {
            value: n.to_string(),
            style: ast::ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: ast::Chomping::Clip,
        });
    }

    // Try as float
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(CustomNode::Scalar {
            value: f.to_string(),
            style: ast::ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: ast::Chomping::Clip,
        });
    }

    // Try as string
    if let Ok(s) = obj.extract::<String>() {
        return Ok(CustomNode::Scalar {
            value: s,
            style: ast::ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: ast::Chomping::Clip,
        });
    }

    Err(pyo3::exceptions::PyValueError::new_err(
        "Unsupported Python type for YAML conversion",
    ))
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
