//! Convert Python objects (dict/list/scalars/ndarray) to `CustomNode` AST.

use super::super::YamlTypeError;
use super::convert::format_i18n_error;
use super::type_registry;

#[cfg(feature = "numpy")]
use super::ndarray::{is_dimensioned_array, ndarray_to_node};
use crate::ast::{CustomNode, NodeMap};

use crate::parser::yaml::schema::needs_quotes;
use crate::parser::yaml::schema::resolve_core_type;
use crate::parser::yaml::types::YamlType;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytesMethods, PyDict, PyInt, PyList, PyString, PyStringMethods, PyTuple};
// Only the NumPy veto reads a float's exact type: it is the cheap question that keeps a plain
// Python float from paying for the import.
#[cfg(feature = "numpy")]
use pyo3::types::PyFloat;
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

/// Whether `text` is exactly what `str()` produces for a CPython `int`: an optional leading `-`,
/// then digits and nothing else.
///
/// The writers use it to tell a genuine integer spelling from an `int` subclass that overrides
/// `__str__` (`IntEnum` prints `<Klass.MEMBER>` on some versions), so such a subclass keeps the
/// route it always had instead of gaining a scalar text the reader could never resolve.
pub(crate) fn is_decimal_digits(text: &str) -> bool {
    let body = text.strip_prefix('-').unwrap_or(text);
    !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit())
}

/// Whether a mapping key has a form this project's reader can hand back.
///
/// The dict projection converts only the *outer* sequence of a YAML key to a tuple — `? [1, 2]`
/// reads as `{(1, 2): …}`, while `? [[1], [2]]` raises Python's own `unhashable type: 'list'`,
/// measured against ruamel's safe loader and recorded here as the documented divergence. A writer
/// that accepted a tuple containing another sequence would therefore emit text the loader refuses,
/// breaking the round trip in the direction that matters; the empty tuple is fine, because `? []`
/// re-reads as `()`.
pub(crate) fn key_is_readable_shape(obj: &Bound<'_, PyAny>) -> bool {
    if let Ok(tuple) = obj.cast::<PyTuple>() {
        return tuple.iter().all(|item| !has_structure(&item));
    }
    if let Ok(list) = obj.cast::<PyList>() {
        return list.iter().all(|item| !has_structure(&item));
    }
    true
}

/// A dict, list or tuple — the shapes whose Python projection a key cannot be built from.
fn has_structure(obj: &Bound<'_, PyAny>) -> bool {
    obj.cast::<PyDict>().is_ok() || obj.cast::<PyList>().is_ok() || obj.cast::<PyTuple>().is_ok()
}

/// 将 Python 对象递归转换为 `CustomNode` AST 节点，支持 dict/list/tuple/str/int/float/bool/None/ndarray。
pub(crate) fn pyobject_to_node(py: Python, obj: &Py<PyAny>) -> PyResult<CustomNode> {
    let obj = obj.bind(py);

    if obj.is_none() {
        return Ok(CustomNode::plain_null());
    }

    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut pairs = NodeMap::default();
        for (key, value) in dict.iter() {
            let key_node = if let Ok(key_str) = key.cast::<PyString>() {
                let text = py_string_to_arc(key_str)?;
                if needs_quotes(&text) {
                    CustomNode::double_quoted_scalar(text)
                } else {
                    CustomNode::plain_scalar(text)
                }
            } else {
                if !key_is_readable_shape(&key) {
                    return Err(YamlTypeError::new_err(format_i18n_error(
                        "unsupported-type",
                        &[],
                    )));
                }
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

    // A tuple is a sequence here for one reason: the reader hands one back. `? [a, b]` projects to
    // the Python key `("a", "b")` (an unhashable list cannot be a dict key), so a converter that
    // refused tuples made `safe_dump(safe_load(text))` raise on a document the loader accepted.
    if let Ok(tuple) = obj.cast::<PyTuple>() {
        let items: Vec<CustomNode> = tuple
            .iter()
            .map(|item| pyobject_to_node(py, &item.into_any().unbind()))
            .collect::<Result<_, _>>()?;
        return Ok(CustomNode::plain_sequence(items));
    }

    // Whether the number protocol may answer for this object at all. A dimensional NumPy array has
    // `__index__` and `__float__`, and on the numpy versions up to 2.2 they succeed for a
    // one-element array: `numpy.array([42])` became the scalar `42` here and
    // `numpy.array([1e30], dtype="float32")` became the f64 widening of the element, so the emitted
    // shape depended on which numpy the caller had installed. An array is therefore read by the
    // route below or not at all; a 0-D array keeps the protocol's scalar answer, which is the
    // spelling `docs/en/guides/numpy.md` states. Guarded by two pointer compares so a native int or
    // float never pays for the numpy import.
    #[cfg(feature = "numpy")]
    let shaped_array = obj.cast::<PyInt>().is_err()
        && obj.cast::<PyFloat>().is_err()
        && is_dimensioned_array(py, obj);
    #[cfg(not(feature = "numpy"))]
    let shaped_array = false;

    // A registered `CustomType` is asked before Python's number protocol, for every shape except an
    // `int` or `int` subclass: a class that only *answers* as a number (`Decimal`, a money type
    // carrying `__index__`) belongs to its handler, and an extraction that succeeds is not a claim
    // of ownership. Integers keep their own spelling because that is what a reader resolves, and
    // the one cast is what keeps the common value in a real document off the registry path, which
    // snapshots the handlers and calls `isinstance` per entry.
    if obj.cast::<PyInt>().is_err()
        && !type_registry::is_empty()
        && let Some(result) = type_registry::try_to_yaml(py, &obj.clone().unbind())
    {
        let (tag_name, yaml_str) = result?;
        let tag = crate::py::tags::tag_of(&tag_name);
        return Ok(CustomNode::Scalar {
            value: std::sync::Arc::from(yaml_str),
            style: crate::ast::ScalarStyle::Plain,
            chomping: crate::ast::Chomping::Clip,
            block_indent: None,
            meta: crate::ast::NodeMeta::with_tag(tag),
        });
    }

    if let Ok(b) = obj.extract::<bool>() {
        return Ok(CustomNode::plain_scalar(if b { "true" } else { "false" }));
    }

    if !shaped_array && let Ok(n) = obj.extract::<i64>() {
        return Ok(CustomNode::plain_scalar(n.to_string()));
    }

    // A Python int wider than `i64` has to be caught before the `f64` extraction below: that one
    // *succeeds* on every `PyLong`, so a 30-digit integer used to leave the hub as
    // `123456789012345680000000000000.0` - a float spelling of a different number than the caller
    // passed. YAML puts no width limit on integers; the reader's i64 cap is a separate, documented
    // boundary. An `int` subclass whose text is not a number falls through to the old behaviour.
    if obj.cast::<PyInt>().is_ok() {
        let digits = py_string_to_arc(&obj.str()?)?;
        if is_decimal_digits(&digits) {
            return Ok(CustomNode::plain_scalar(digits));
        }
    }

    if !shaped_array && let Ok(f) = obj.extract::<f64>() {
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
    if let Some(node) = ndarray_to_node(py, obj) {
        return Ok(node);
    }

    // Check registered CustomTypes for serialization
    if let Some(result) = {
        let obj_ref = obj.clone().unbind();
        type_registry::try_to_yaml(py, &obj_ref)
    } {
        let (tag_name, yaml_str) = result?;
        let tag = crate::py::tags::tag_of(&tag_name);
        return Ok(CustomNode::Scalar {
            value: std::sync::Arc::from(yaml_str),
            style: crate::ast::ScalarStyle::Plain,
            chomping: crate::ast::Chomping::Clip,
            block_indent: None,
            meta: crate::ast::NodeMeta::with_tag(tag),
        });
    }

    Err(YamlTypeError::new_err(format_i18n_error(
        "unsupported-type",
        &[],
    )))
}
