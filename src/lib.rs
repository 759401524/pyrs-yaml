pub mod ast;
pub mod parser;
pub mod serializer;

use ast::CustomNode;
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
    let ast = py.allow_threads(|| parser::parse(yaml).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("YAML parse error: {}", e))
    }))?;

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
    m.add_class::<YamlDocument>()?;
    Ok(())
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
