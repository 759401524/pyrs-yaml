use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use pyo3::prelude::{Py, PyAny, Python};

type HandlerList = Vec<(u32, Py<PyAny>)>;
static TAG_REGISTRY: LazyLock<Mutex<HashMap<String, HandlerList>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn register(name: &str, handler: Py<PyAny>, priority: u32) {
    let Ok(mut guard) = TAG_REGISTRY.lock() else {
        return;
    };
    guard
        .entry(name.to_string())
        .or_default()
        .push((priority, handler));
}

pub(crate) fn get_handlers(name: &str, py: Python<'_>) -> Option<Vec<(u32, Py<PyAny>)>> {
    let guard = TAG_REGISTRY.lock().ok()?;
    let handlers = guard.get(name)?;
    let mut sorted: Vec<(u32, Py<PyAny>)> = handlers
        .iter()
        .map(|(p, h)| (*p, h.clone_ref(py)))
        .collect();
    sorted.sort_by_key(|(p, _)| *p);
    Some(sorted)
}

pub(crate) fn clear_all() {
    let Ok(mut guard) = TAG_REGISTRY.lock() else {
        return;
    };
    guard.clear();
}

/// Cheap check whether any tag handlers are registered.
pub(crate) fn is_empty() -> bool {
    let Ok(guard) = TAG_REGISTRY.lock() else {
        return true;
    };
    guard.is_empty()
}

pub(crate) fn remove(name: &str) {
    let Ok(mut guard) = TAG_REGISTRY.lock() else {
        return;
    };
    guard.remove(name);
}
