//! Conversion between `CustomNode` and Python objects, including alias
//! resolution and YAML type inference.

use crate::YamlTypeError;
use crate::ast::{CustomNode, ScalarStyle};
use crate::parser::yaml::{YamlSchema, YamlType, resolve_yaml_type};

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList};
use std::collections::{HashMap, HashSet};

/// 格式化 i18n 错误消息（pub 以便 sibling modules 使用）。
pub fn format_i18n_error(key: &str, args: &[(&str, &str)]) -> String {
    crate::i18n::format_message(key, args)
}

/// Parse a schema string into YamlSchema.
pub fn parse_schema(raw: &str) -> PyResult<YamlSchema> {
    let s = raw.to_lowercase();
    match s.as_str() {
        "core" | "yaml.org,2002" | "yamlorg2002" => Ok(YamlSchema::Core),
        "json" | "yaml.org,2002:json" => Ok(YamlSchema::Json),
        "failsafe" | "yaml.org,2002:failsafe" => Ok(YamlSchema::Failsafe),
        "yaml1.1" | "1.1" | "yaml.org,2002:yaml1.1" => Ok(YamlSchema::Yaml1_1),
        _ => Err(YamlTypeError::new_err(format!(
            "Unsupported schema '{}'. Supported: core, json, failsafe, yaml1.1",
            raw
        ))),
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
    schema: YamlSchema,
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
pub fn node_to_pyobject(node: &CustomNode, py: Python, schema: YamlSchema) -> PyResult<Py<PyAny>> {
    node_to_pyobject_simple(node, py, schema)
}

fn scalar_to_pyobject(
    py: Python,
    value: &str,
    style: &ScalarStyle,
    schema: YamlSchema,
) -> PyResult<Py<PyAny>> {
    if matches!(
        style,
        ScalarStyle::Plain | ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted
    ) {
        // Note: quoted scalars are also resolved for round-trip compatibility.
        // The serializer quotes negative numbers / special values, so the
        // loader must de-quote them back to the correct type.
        match resolve_yaml_type(value, schema) {
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

fn batch_mapping<'a, F>(
    py: Python,
    schema: YamlSchema,
    pairs: &'a indexmap::IndexMap<CustomNode, CustomNode>,
    mut convert: F,
) -> PyResult<Py<PyAny>>
where
    F: FnMut(&'a CustomNode, Python, YamlSchema) -> PyResult<Py<PyAny>>,
{
    let mut items: Vec<(Py<PyAny>, Py<PyAny>)> = Vec::with_capacity(pairs.len());
    for (key, value) in pairs {
        let k = match key {
            CustomNode::Scalar { value, .. } => {
                value.as_ref().into_pyobject(py)?.into_any().unbind()
            }
            _ => format!("{:?}", key).into_pyobject(py)?.into_any().unbind(),
        };
        let v = convert(value, py, schema)?;
        items.push((k, v));
    }
    let list = PyList::new(py, items)?;
    let dict = PyDict::from_sequence(list.as_any())?;
    Ok(dict.into_any().unbind())
}

fn batch_sequence<'a, F>(
    py: Python,
    schema: YamlSchema,
    items: &'a [CustomNode],
    mut convert: F,
) -> PyResult<Py<PyAny>>
where
    F: FnMut(&'a CustomNode, Python, YamlSchema) -> PyResult<Py<PyAny>>,
{
    let mut vals: Vec<Py<PyAny>> = Vec::with_capacity(items.len());
    for item in items {
        vals.push(convert(item, py, schema)?);
    }
    let list = PyList::new(py, vals)?;
    Ok(list.into_any().unbind())
}

fn node_to_pyobject_inner<'a>(
    node: &'a CustomNode,
    py: Python,
    anchors: &HashMap<&'a str, &'a CustomNode>,
    visited: &mut HashSet<usize>,
    schema: YamlSchema,
) -> PyResult<Py<PyAny>> {
    match node {
        CustomNode::Scalar { value, style, .. } => scalar_to_pyobject(py, value, style, schema),
        CustomNode::Mapping { pairs, .. } => batch_mapping(py, schema, pairs, |n, py, schema| {
            node_to_pyobject_with_anchors(n, py, anchors, visited, schema)
        }),
        CustomNode::Sequence { items, .. } => batch_sequence(py, schema, items, |n, py, schema| {
            node_to_pyobject_with_anchors(n, py, anchors, visited, schema)
        }),
        CustomNode::Null { .. } => Ok(py.None()),
        CustomNode::Alias { .. } => Ok(py.None()),
    }
}

pub(crate) fn node_to_pyobject_simple(
    node: &CustomNode,
    py: Python,
    schema: YamlSchema,
) -> PyResult<Py<PyAny>> {
    match node {
        CustomNode::Scalar { value, style, .. } => scalar_to_pyobject(py, value, style, schema),
        CustomNode::Mapping { pairs, .. } => batch_mapping(py, schema, pairs, |n, py, schema| {
            node_to_pyobject_simple(n, py, schema)
        }),
        CustomNode::Sequence { items, .. } => batch_sequence(py, schema, items, |n, py, schema| {
            node_to_pyobject_simple(n, py, schema)
        }),
        CustomNode::Null { .. } => Ok(py.None()),
        CustomNode::Alias { .. } => Ok(py.None()),
    }
}
