//! CustomType registry — Community Plugins (Spiral 3).
//!
//! Registers `CustomType` Python instances keyed by tag name. Unlike the tag
//! registry (which returns string replacements for scalar values), a
//! `CustomType` provides full `from_yaml`/`to_yaml`/`can_parse`/`validate`
//! methods so a node can be converted to/from an arbitrary Python object
//! (e.g. `!timestamp` → `datetime`).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use pyo3::prelude::{Py, PyAny, PyResult, Python};
use pyo3::types::PyAnyMethods;

static TYPE_REGISTRY: LazyLock<Mutex<HashMap<String, Py<PyAny>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn register(name: &str, handler: Py<PyAny>) {
    let Ok(mut guard) = TYPE_REGISTRY.lock() else {
        return;
    };
    guard.insert(name.to_string(), handler);
}

pub fn get(name: &str, py: Python<'_>) -> Option<Py<PyAny>> {
    let guard = TYPE_REGISTRY.lock().ok()?;
    guard.get(name).map(|h| h.clone_ref(py))
}

/// Try to serialize a Python object using a registered CustomType.
/// Returns `(tag_name, yaml_string)` if a match is found.
pub fn try_to_yaml(py: Python<'_>, obj: &Py<PyAny>) -> Option<PyResult<(String, String)>> {
    let guard = TYPE_REGISTRY.lock().ok()?;
    if guard.is_empty() {
        return None;
    }
    let obj_ref = obj.bind(py);
    for (tag_name, handler) in guard.iter() {
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

pub fn clear_all() {
    let Ok(mut guard) = TYPE_REGISTRY.lock() else {
        return;
    };
    guard.clear();
}

/// Cheap check whether any custom types are registered.
pub fn is_empty() -> bool {
    let Ok(guard) = TYPE_REGISTRY.lock() else {
        return true;
    };
    guard.is_empty()
}

pub fn remove(name: &str) {
    let Ok(mut guard) = TYPE_REGISTRY.lock() else {
        return;
    };
    guard.remove(name);
}
