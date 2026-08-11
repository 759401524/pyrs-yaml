//! CustomType registry — Community Plugins (Spiral 3).
//!
//! Registers `CustomType` Python instances keyed by tag name. Unlike the tag
//! registry (which returns string replacements for scalar values), a
//! `CustomType` provides full `from_yaml`/`to_yaml`/`can_parse`/`validate`
//! methods so a node can be converted to/from an arbitrary Python object
//! (e.g. `!timestamp` → `datetime`).
//!
//! The registry is ordered: later-registered types take priority over
//! earlier-registered ones. This ensures user-registered types override
//! built-in plugins when both match the same `python_type`.

use std::sync::{LazyLock, Mutex};

use indexmap::IndexMap;
use pyo3::Bound;
use pyo3::prelude::{Py, PyAny, PyResult, Python};
use pyo3::types::PyAnyMethods;
use pyo3::types::PyDict;
use pyo3::types::PyDictMethods;
use pyo3::types::PyFrozenSet;
use pyo3::types::PyFrozenSetMethods;
use pyo3::types::PyList;
use pyo3::types::PyListMethods;
use pyo3::types::PySet;
use pyo3::types::PySetMethods;
use pyo3::types::PyTuple;
use pyo3::types::PyTupleMethods;

static TYPE_REGISTRY: LazyLock<Mutex<IndexMap<String, Py<PyAny>>>> =
    LazyLock::new(|| Mutex::new(IndexMap::new()));

pub(crate) fn register(name: &str, handler: Py<PyAny>) {
    let Ok(mut guard) = TYPE_REGISTRY.lock() else {
        return;
    };
    guard.insert(name.to_string(), handler);
}

pub(crate) fn get(name: &str, py: Python<'_>) -> Option<Py<PyAny>> {
    let guard = TYPE_REGISTRY.lock().ok()?;
    guard.get(name).map(|h| h.clone_ref(py))
}

/// Try to serialize a Python object using a registered CustomType.
/// Returns `(tag_name, yaml_string)` if a match is found.
///
/// Types are checked in reverse registration order: later-registered types
/// take priority over earlier ones. This ensures user-registered types
/// override built-in plugins when both match the same `python_type`.
///
/// The lock is released before calling any Python method to avoid deadlock
/// if a `to_yaml` or `isinstance` re-enters the registry.
pub(crate) fn try_to_yaml(py: Python<'_>, obj: &Py<PyAny>) -> Option<PyResult<(String, String)>> {
    let snapshot: Vec<(String, Py<PyAny>)> = {
        let Ok(guard) = TYPE_REGISTRY.lock() else {
            return None;
        };
        if guard.is_empty() {
            return None;
        }
        guard
            .iter()
            .rev()
            .map(|(k, v)| (k.clone(), v.clone_ref(py)))
            .collect()
    };
    let obj_ref = obj.bind(py);
    for (tag_name, handler) in &snapshot {
        let handler_ref = handler.bind(py);
        if let Ok(py_type) = handler_ref.getattr("python_type")
            && !py_type.is_none()
        {
            // Check isinstance(obj, python_type) via builtins isinstance
            let is_match = (|| -> PyResult<bool> {
                let builtins = py.import("builtins")?;
                let isinstance_fn = builtins.getattr("isinstance")?;
                let result = isinstance_fn.call1((obj_ref, py_type))?;
                result.extract()
            })();
            if let Ok(true) = is_match {
                // Call to_yaml on the handler
                let result = handler_ref.call_method1("to_yaml", (obj_ref,));
                return match result {
                    Ok(yaml_str) => {
                        let s = yaml_str.extract::<String>().ok()?;
                        Some(Ok((tag_name.clone(), s)))
                    }
                    Err(e) => Some(Err(e)),
                };
            }
        }
    }
    None
}

pub(crate) fn clear_all() {
    let Ok(mut guard) = TYPE_REGISTRY.lock() else {
        return;
    };
    guard.clear();
}

/// Validate a Python object against all registered CustomTypes.
///
/// Recursively walks dicts and lists. For each value that matches a
/// registered type's `python_type`, calls the handler's `validate` method.
/// Returns `Ok(())` if all valid; otherwise the first validation error.
///
/// The lock is released before recursively walking Python objects so a
/// `validate` method that re-enters the registry cannot deadlock.
pub(crate) fn validate_custom_types(py: Python<'_>, obj: &Py<PyAny>) -> PyResult<()> {
    let snapshot: Vec<(String, Py<PyAny>)> = {
        let Ok(guard) = TYPE_REGISTRY.lock() else {
            return Ok(());
        };
        if guard.is_empty() {
            return Ok(());
        }
        guard
            .iter()
            .rev()
            .map(|(k, v)| (k.clone(), v.clone_ref(py)))
            .collect()
    };
    let obj_ref = obj.bind(py);
    validate_node(py, obj_ref, &snapshot)
}

fn validate_node(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    types: &[(String, Py<PyAny>)],
) -> PyResult<()> {
    // Check dict
    if let Ok(dict) = obj.cast::<PyDict>() {
        for (k, v) in dict.iter() {
            validate_node(py, &v, types)?;
            let _ = k;
        }
        return Ok(());
    }
    // Check list
    if let Ok(list) = obj.cast::<PyList>() {
        for item in list.iter() {
            validate_node(py, &item, types)?;
        }
        return Ok(());
    }
    // Check tuple
    if let Ok(tuple) = obj.cast::<PyTuple>() {
        for item in tuple.iter() {
            validate_node(py, &item, types)?;
        }
        return Ok(());
    }
    // Check set / frozenset
    if let Ok(set) = obj.cast::<PySet>() {
        for item in set.iter() {
            validate_node(py, &item, types)?;
        }
        return Ok(());
    }
    if let Ok(fset) = obj.cast::<PyFrozenSet>() {
        for item in fset.iter() {
            validate_node(py, &item, types)?;
        }
        return Ok(());
    }
    // Check registered types (reverse order: user types first)
    for (tag_name, handler) in types {
        let handler_ref = handler.bind(py);
        if let Ok(py_type) = handler_ref.getattr("python_type")
            && !py_type.is_none()
        {
            let is_match = (|| -> PyResult<bool> {
                let builtins = py.import("builtins")?;
                let isinstance_fn = builtins.getattr("isinstance")?;
                let result = isinstance_fn.call1((obj, py_type))?;
                result.extract()
            })();
            if let Ok(true) = is_match {
                // Call validate on the handler
                let valid = handler_ref
                    .call_method1("validate", (obj,))?
                    .extract::<bool>()?;
                if !valid {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "value failed validation for custom type '{}'",
                        tag_name.trim_start_matches('!')
                    )));
                }
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Cheap check whether any custom types are registered.
pub(crate) fn is_empty() -> bool {
    let Ok(guard) = TYPE_REGISTRY.lock() else {
        return true;
    };
    guard.is_empty()
}

pub(crate) fn remove(name: &str) {
    let Ok(mut guard) = TYPE_REGISTRY.lock() else {
        return;
    };
    guard.swap_remove(name);
}
