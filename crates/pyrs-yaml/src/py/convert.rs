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
pub(crate) fn format_i18n_error(key: &str, args: &[(&str, &str)]) -> String {
    crate::i18n::format_message(key, args)
}

/// Try to convert a tagged scalar via a registered CustomType.
/// Returns `Some(PyObject)` if a handler matched and `can_parse` returned true;
/// returns `None` to fall through to default schema resolution.
fn try_custom_type(py: Python<'_>, tag: &str, value: &str) -> PyResult<Option<Py<PyAny>>> {
    if let Some(handler) = type_registry::get(tag, py) {
        let can_parse = handler
            .call_method1(py, "can_parse", (value,))
            .and_then(|r| r.extract::<bool>(py))?;
        if can_parse {
            return handler.call_method1(py, "from_yaml", (value,)).map(Some);
        }
    }
    Ok(None)
}

/// Parse a schema string into Schema. Returns built-in schemas or looks up
/// the global registry for custom schemas.
pub(crate) fn parse_schema(raw: &str) -> PyResult<Schema> {
    if let Ok(schema) = raw.parse::<Schema>() {
        Ok(schema)
    } else if let Some(schema) = registry::get(&raw.to_lowercase()) {
        Ok(schema)
    } else {
        let custom = registry::names()
            .iter()
            .filter(|n| !matches!(n.as_str(), "core" | "json" | "failsafe" | "yaml1.1"))
            .map(|n| format!("'{}'", n))
            .collect::<Vec<_>>()
            .join(", ");
        Err(YamlTypeError::new_err(format!(
            "Unsupported schema '{}'. Supported: core, json, failsafe, yaml1.1{}",
            raw,
            if custom.is_empty() {
                String::new()
            } else {
                format!(", {custom}")
            }
        )))
    }
}

/// 递归遍历 AST，收集所有锚点到节点的映射，用于别名解析。
pub(crate) fn collect_anchors<'a>(
    node: &'a CustomNode,
    anchors: &mut HashMap<&'a str, &'a CustomNode>,
) {
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
pub(crate) fn node_to_pyobject_with_anchors<'a>(
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

fn scalar_to_pyobject(
    py: Python,
    value: &str,
    style: &ScalarStyle,
    schema: &Schema,
) -> PyResult<Py<PyAny>> {
    // YAML 1.2: only plain scalars undergo implicit schema resolution. Single-
    // and double-quoted scalars always load as strings, regardless of content.
    if matches!(style, ScalarStyle::Plain) {
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
            value, style, meta, ..
        } => {
            if let Some(t) = meta.tag.as_ref()
                && let Some(result) = try_custom_type(py, &t.to_string(), value.as_ref())?
            {
                return Ok(result);
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

/// 将 `CustomNode` 转换为 Python 对象，不解析别名（别名节点返回 `None`）。
///
/// Thin wrapper over `node_to_pyobject_with_anchors` with empty anchors/visited
/// sets: with no anchors registered, alias nodes fall through to `None` and no
/// cycle tracking is needed.
pub(crate) fn node_to_pyobject_simple(
    node: &CustomNode,
    py: Python,
    schema: &Schema,
) -> PyResult<Py<PyAny>> {
    let anchors: HashMap<&str, &CustomNode> = HashMap::new();
    let mut visited: HashSet<usize> = HashSet::new();
    node_to_pyobject_with_anchors(node, py, &anchors, &mut visited, schema)
}
