use std::collections::HashMap;
use std::sync::Mutex;

use pyo3::prelude::{Py, PyAny, Python};

#[allow(clippy::type_complexity)]
static TAG_REGISTRY: Mutex<Option<HashMap<String, Vec<(u32, Py<PyAny>)>>>> = Mutex::new(None);

pub fn register(name: &str, handler: Py<PyAny>, priority: u32) {
    let mut guard = TAG_REGISTRY.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.entry(name.to_string())
        .or_default()
        .push((priority, handler));
}

pub fn get_handlers(name: &str, py: Python<'_>) -> Option<Vec<(u32, Py<PyAny>)>> {
    let guard = TAG_REGISTRY.lock().unwrap();
    let map = guard.as_ref()?;
    let handlers = map.get(name)?;
    let mut sorted: Vec<(u32, Py<PyAny>)> = handlers
        .iter()
        .map(|(p, h)| (*p, h.clone_ref(py)))
        .collect();
    sorted.sort_by_key(|(p, _)| *p);
    Some(sorted)
}

pub fn clear_all() {
    let mut guard = TAG_REGISTRY.lock().unwrap();
    *guard = None;
}

pub fn remove(name: &str) {
    let mut guard = TAG_REGISTRY.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        map.remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_nonexistent() {
        pyo3::Python::try_attach(|py| {
            assert!(get_handlers("!nonexistent", py).is_none());
        });
    }

    #[test]
    fn test_remove_nonexistent() {
        remove("!custom");
    }
}
