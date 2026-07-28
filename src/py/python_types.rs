//! Convert Python objects (dict/list/scalars/ndarray) to `CustomNode` AST.

use super::super::YamlTypeError;
use super::convert::format_i18n_error;

#[cfg(feature = "numpy")]
use super::ndarray::ndarray_to_node;
use crate::ast::CustomNode;

use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// 将 Python 对象递归转换为 `CustomNode` AST 节点，支持 dict/list/str/int/float/bool/None/ndarray。
pub fn pyobject_to_node(py: Python, obj: &Py<PyAny>) -> PyResult<CustomNode> {
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

    if let Ok(list) = obj.cast::<PyList>() {
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

    #[cfg(feature = "numpy")]
    if let Some(node) = ndarray_to_node(py, obj) {
        return Ok(node);
    }

    Err(YamlTypeError::new_err(format_i18n_error(
        "unsupported-type",
        &[],
    )))
}

/// 将 `serde_json::Value` 转换为 `CustomNode` AST 节点。
pub fn json_value_to_node(value: &serde_json::Value) -> PyResult<CustomNode> {
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
