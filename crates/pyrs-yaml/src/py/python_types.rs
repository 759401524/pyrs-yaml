//! Convert Python objects (dict/list/scalars/ndarray) to `CustomNode` AST.

use super::super::YamlTypeError;
use super::convert::format_i18n_error;
use super::type_registry;

#[cfg(feature = "numpy")]
use super::ndarray::ndarray_to_node;
use crate::ast::CustomNode;

use crate::parser::yaml::schema::needs_quotes;
use crate::parser::yaml::schema::resolve_core_type;
use crate::parser::yaml::types::YamlType;
use indexmap::IndexMap;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytesMethods, PyDict, PyList, PyString, PyStringMethods};
use std::sync::Arc;

/// 将 Python 字符串零拷贝借用后转 `Arc<str>`：`encode_utf8` 产出 UTF-8 `PyBytes`
/// （单次分配），`Arc::from(&str)` 再拷贝一次 —— 共 2 次分配，与
/// `extract::<String>`（内部即 `to_cow().into_owned()`）路径持平。
/// 注：abi3-py38 下 `PyStringMethods::to_str` 被 cfg 禁用，借用必须走 `encode_utf8`。
pub(crate) fn py_string_to_arc(s: &Bound<'_, PyString>) -> PyResult<Arc<str>> {
    let bytes = s.encode_utf8()?;
    let text = std::str::from_utf8(bytes.as_bytes())
        .map_err(|e| PyValueError::new_err(format!("invalid UTF-8 in Python string: {e}")))?;
    Ok(Arc::from(text))
}

/// Format an `f64` for YAML output so it round-trips back as a float.
///
/// Rust `Display` emits `42.0` as `"42"` and `1e300` as a bare digit string —
/// both resolve back as int/string on load. Appending `.0` when the text does
/// not already resolve to a float keeps the type stable (`inf`/`NaN` already
/// resolve as floats and are left untouched).
pub(crate) fn float_to_yaml_string(f: f64) -> String {
    let mut s = f.to_string();
    if !matches!(resolve_core_type(&s), YamlType::Float(_)) {
        s.push_str(".0");
    }
    s
}

/// 将 Python 对象递归转换为 `CustomNode` AST 节点，支持 dict/list/str/int/float/bool/None/ndarray。
pub(crate) fn pyobject_to_node(py: Python, obj: &Py<PyAny>) -> PyResult<CustomNode> {
    let obj = obj.bind(py);

    if obj.is_none() {
        return Ok(CustomNode::plain_null());
    }

    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut pairs = IndexMap::new();
        for (key, value) in dict.iter() {
            let key_node = if let Ok(key_str) = key.cast::<PyString>() {
                let text = py_string_to_arc(key_str)?;
                if needs_quotes(&text) {
                    CustomNode::double_quoted_scalar(text)
                } else {
                    CustomNode::plain_scalar(text)
                }
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

    // Check CustomType for types that would be extracted as f64
    // (e.g. Decimal) before the float fallback.
    if !type_registry::is_empty()
        && let Some(result) = type_registry::try_to_yaml(py, &obj.clone().unbind())
    {
        let (tag_name, yaml_str) = result?;
        let tag = crate::ast::Tag {
            handle: "!".to_string(),
            suffix: tag_name.trim_start_matches('!').to_string(),
        };
        return Ok(CustomNode::Scalar {
            value: std::sync::Arc::from(yaml_str),
            style: crate::ast::ScalarStyle::Plain,
            chomping: crate::ast::Chomping::Clip,
            meta: crate::ast::NodeMeta {
                tag: Some(tag),
                ..Default::default()
            },
        });
    }

    if let Ok(f) = obj.extract::<f64>() {
        return Ok(CustomNode::plain_scalar(float_to_yaml_string(f)));
    }

    if let Ok(s) = obj.cast::<PyString>() {
        let text = py_string_to_arc(s)?;
        return Ok(if needs_quotes(&text) {
            CustomNode::double_quoted_scalar(text)
        } else {
            CustomNode::plain_scalar(text)
        });
    }

    #[cfg(feature = "numpy")]
    if py.import("numpy").is_ok() {
        // numpy is only probed at runtime: free-threaded (Py_GIL_DISABLED)
        // builds compile with the default "numpy" feature but ship without
        // the module installed, and numpy's capsule access would panic.
        if let Some(node) = ndarray_to_node(py, obj) {
            return Ok(node);
        }
    }

    // Check registered CustomTypes for serialization
    if let Some(result) = {
        let obj_ref = obj.clone().unbind();
        type_registry::try_to_yaml(py, &obj_ref)
    } {
        let (tag_name, yaml_str) = result?;
        let tag = crate::ast::Tag {
            handle: "!".to_string(),
            suffix: tag_name.trim_start_matches('!').to_string(),
        };
        return Ok(CustomNode::Scalar {
            value: std::sync::Arc::from(yaml_str),
            style: crate::ast::ScalarStyle::Plain,
            chomping: crate::ast::Chomping::Clip,
            meta: crate::ast::NodeMeta {
                tag: Some(tag),
                ..Default::default()
            },
        });
    }

    Err(YamlTypeError::new_err(format_i18n_error(
        "unsupported-type",
        &[],
    )))
}

/// 将 `serde_json::Value` 转换为 `CustomNode` AST 节点。
pub(crate) fn json_value_to_node(value: &serde_json::Value) -> PyResult<CustomNode> {
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
        serde_json::Value::String(s) => {
            let value = s.clone();
            Ok(if needs_quotes(&value) {
                CustomNode::double_quoted_scalar(value)
            } else {
                CustomNode::plain_scalar(value)
            })
        }
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
                let key_node = if needs_quotes(key) {
                    CustomNode::double_quoted_scalar(key.clone())
                } else {
                    CustomNode::plain_scalar(key.clone())
                };
                let value_node = json_value_to_node(value)?;
                pairs.insert(key_node, value_node);
            }
            Ok(CustomNode::plain_mapping(pairs))
        }
    }
}
