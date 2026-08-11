//! Conversion between `CustomNode` and Python objects, including alias
//! resolution and YAML type inference.

use crate::YamlTypeError;
use crate::ast::{CustomNode, ScalarStyle};
use crate::parser::yaml::registry;
use crate::parser::yaml::{Schema, YamlType};
use crate::py::type_registry;

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList};
use std::collections::{HashMap, HashSet};

/// 格式化 i18n 错误消息（pub 以便 sibling modules 使用）。
pub fn format_i18n_error(key: &str, args: &[(&str, &str)]) -> String {
    crate::i18n::format_message(key, args)
}

/// Parse a schema string into Schema. Returns built-in schemas or looks up
/// the global registry for custom schemas.
pub fn parse_schema(raw: &str) -> PyResult<Schema> {
    let s = raw.to_lowercase();
    match s.as_str() {
        "core" | "yaml.org,2002" | "yamlorg2002" => Ok(Schema::Core),
        "json" | "yaml.org,2002:json" => Ok(Schema::Json),
        "failsafe" | "yaml.org,2002:failsafe" => Ok(Schema::Failsafe),
        "yaml1.1" | "1.1" | "yaml.org,2002:yaml1.1" => Ok(Schema::Yaml1_1),
        _ => {
            // Try the global registry
            if let Some(schema) = registry::get(&s) {
                Ok(schema)
            } else {
                Err(YamlTypeError::new_err(format!(
                    "Unsupported schema '{}'. Supported: core, json, failsafe, yaml1.1 {}",
                    raw,
                    registry::names()
                        .iter()
                        .filter(|n| !matches!(n.as_str(), "core" | "json" | "failsafe" | "yaml1.1"))
                        .map(|n| format!("'{}'", n))
                        .collect::<Vec<_>>()
                        .join(", "),
                )))
            }
        }
    }
}

/// 递归遍历 AST，收集所有锚点到节点的映射，用于别名解析。
pub fn collect_anchors<'a>(node: &'a CustomNode, anchors: &mut HashMap<&'a str, &'a CustomNode>) {
    if let Some(name) = node.anchor() {
        anchors.insert(name, node);
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
pub fn node_to_pyobject_with_anchors<'a>(
    node: &'a CustomNode,
    py: Python,
    anchors: &HashMap<&'a str, &'a CustomNode>,
    visited: &mut HashSet<usize>,
    schema: &Schema,
) -> PyResult<Py<PyAny>> {
    match node {
        CustomNode::Alias { name } => {
            if let Some(target) = anchors.get(name.as_str()) {
                let addr = std::ptr::addr_of!(*target) as usize;
                if visited.contains(&addr) {
                    return Ok(py.None());
                }
                visited.insert(addr);
                node_to_pyobject_with_anchors(target, py, anchors, visited, schema)
            } else {
                Ok(py.None())
            }
        }
        _ => node_to_pyobject_inner(node, py, anchors, visited, schema),
    }
}

/// 将 `CustomNode` 转换为 Python 对象，不解析别名（别名节点返回 `None`）。
pub fn node_to_pyobject(node: &CustomNode, py: Python, schema: &Schema) -> PyResult<Py<PyAny>> {
    node_to_pyobject_simple(node, py, schema)
}

fn scalar_to_pyobject(
    py: Python,
    value: &str,
    style: &ScalarStyle,
    schema: &Schema,
) -> PyResult<Py<PyAny>> {
    if matches!(
        style,
        ScalarStyle::Plain | ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted
    ) {
        match schema.resolve(value) {
            YamlType::Null => Ok(py.None()),
            YamlType::Bool(b) => Ok(PyBool::new(py, b).to_owned().into_any().unbind()),
            YamlType::Int(n) => Ok(n.into_pyobject(py)?.into_any().unbind()),
            YamlType::Float(f) => Ok(f.into_pyobject(py)?.into_any().unbind()),
            YamlType::Str(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        }
    } else {
        Ok(value.into_pyobject(py)?.into_any().unbind())
    }
}

fn node_to_pyobject_inner<'a>(
    node: &'a CustomNode,
    py: Python,
    anchors: &HashMap<&'a str, &'a CustomNode>,
    visited: &mut HashSet<usize>,
    schema: &Schema,
) -> PyResult<Py<PyAny>> {
    match node {
        CustomNode::Scalar {
            value, tag, style, ..
        } => {
            // Check if a registered CustomType handles this tag.
            if let Some(t) = tag {
                let tag_str = t.to_string();
                if let Some(handler) = type_registry::get(&tag_str, py) {
                    return handler.call_method1(py, "from_yaml", (value.as_ref(),));
                }
            }
            scalar_to_pyobject(py, value, style, schema)
        }
        CustomNode::Mapping { pairs, .. } => {
            let dict = PyDict::new(py);
            for (key, value) in pairs {
                let val = node_to_pyobject_with_anchors(value, py, anchors, visited, schema)?;
                match key {
                    CustomNode::Scalar { value, .. } => dict.set_item(value.as_ref(), val),
                    _ => dict.set_item(format!("{:?}", key), val),
                }
                .ok();
            }
            Ok(dict.into_any().unbind())
        }
        CustomNode::Sequence { items, .. } => {
            let list = PyList::empty(py);
            for item in items {
                let val = node_to_pyobject_with_anchors(item, py, anchors, visited, schema)?;
                list.append(val).ok();
            }
            Ok(list.into_any().unbind())
        }
        CustomNode::Null { .. } => Ok(py.None()),
        CustomNode::Alias { .. } => Ok(py.None()),
    }
}

pub(crate) fn node_to_pyobject_simple(
    node: &CustomNode,
    py: Python,
    schema: &Schema,
) -> PyResult<Py<PyAny>> {
    match node {
        CustomNode::Scalar {
            value, tag, style, ..
        } => {
            if let Some(t) = tag {
                let tag_str = t.to_string();
                if let Some(handler) = type_registry::get(&tag_str, py) {
                    return handler.call_method1(py, "from_yaml", (value.as_ref(),));
                }
            }
            scalar_to_pyobject(py, value, style, schema)
        }
        CustomNode::Mapping { pairs, .. } => {
            let dict = PyDict::new(py);
            for (key, value) in pairs {
                let val = node_to_pyobject_simple(value, py, schema)?;
                match key {
                    CustomNode::Scalar { value, .. } => dict.set_item(value.as_ref(), val),
                    _ => dict.set_item(format!("{:?}", key), val),
                }
                .ok();
            }
            Ok(dict.into_any().unbind())
        }
        CustomNode::Sequence { items, .. } => {
            let list = PyList::empty(py);
            for item in items {
                let val = node_to_pyobject_simple(item, py, schema)?;
                list.append(val).ok();
            }
            Ok(list.into_any().unbind())
        }
        CustomNode::Null { .. } => Ok(py.None()),
        CustomNode::Alias { .. } => Ok(py.None()),
    }
}
