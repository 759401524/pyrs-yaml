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

/// Try to convert a tagged value via a registered CustomType.
/// Returns `Some(PyObject)` if a handler matched and `can_parse` returned true;
/// returns `None` to fall through to default schema resolution.
fn try_custom_type(
    py: Python<'_>,
    tag: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<Option<Py<PyAny>>> {
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
///
/// `in_progress` 是**递归路径**上的锚点集合，不是全程累积集（issue #163）：
/// 只有当一个锚点正处在「展开中」时才说明出现了引用环，此时该分支用
/// `None` 占位以终止递归。锚点一旦展开完毕就从集合中移除，因此同一锚点
/// 在兄弟位置被引用任意多次都会各自得到独立的完整副本
/// （`a: &x 1` / `b: *x` / `c: *x` → `1, 1, 1`），而真正的自引用
/// （`a: &x [*x]`）仍然安全终止。
pub(crate) fn node_to_pyobject_with_anchors<'a>(
    node: &'a CustomNode,
    py: Python,
    anchors: &HashMap<&'a str, &'a CustomNode>,
    in_progress: &mut HashSet<usize>,
    schema: &Schema,
) -> PyResult<Py<PyAny>> {
    match node {
        // An alias target that is not currently being expanded has no AST
        // node to walk - the parser rejected the unknown anchor name, or it
        // belongs to a different document. Nothing to replay.
        CustomNode::Alias { .. } => Ok(py.None()),
        _ => node_to_pyobject_inner(node, py, anchors, in_progress, schema),
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
    in_progress: &mut HashSet<usize>,
    schema: &Schema,
) -> PyResult<Py<PyAny>> {
    match node {
        CustomNode::Scalar {
            value, style, meta, ..
        } => {
            // Resolve the scalar value first, then try custom type conversion
            // so that `from_yaml` receives the resolved Python object.
            let py_obj = scalar_to_pyobject(py, value, style, schema)?;
            if let Some(t) = meta.tag.as_ref()
                && let Some(result) = try_custom_type(py, &t.to_string(), py_obj.bind(py))?
            {
                return Ok(result);
            }
            Ok(py_obj)
        }
        CustomNode::Mapping { pairs, .. } => {
            let dict = PyDict::new(py);
            for (key, value) in pairs {
                let val = resolve_alias_target(value, py, anchors, in_progress, schema)?;
                // A key is a node like any other, and YAML resolves it by the same rule
                // the value side uses: `schema.resolve` for plain scalars, text for
                // quoted ones. Taking the key's text instead made one document mean two
                // things depending on which side of the `:` a scalar sat (`~: 1` ->
                // `{'~': 1}` while `a: ~` -> `{'a': None}`, `1: a` -> `{'1': 'a'}`), so a
                // config keyed by an integer, bool or null could not be reached by lookup
                // and disagreed with both reference libraries.
                //
                // Two arms, because the hot case deserves the direct route. An untagged
                // scalar can only become str / int / float / bool / None, every one of
                // them hashable, so it is resolved inline: no alias wrapper and no error
                // bookkeeping, which measured ~10 ns per key on a 120-pair mapping and is
                // what a flagged `to_dict` regression came from. A tagged or aliased key
                // can reach a custom `from_yaml` and return anything, so it keeps the
                // shared path, and an unhashable result there falls back to the source
                // text rather than dropping the pair.
                match key {
                    CustomNode::Scalar {
                        value, style, meta, ..
                    } if meta.tag.is_none() => {
                        dict.set_item(scalar_to_pyobject(py, value, style, schema)?, &val)?;
                    }
                    CustomNode::Scalar { .. }
                    | CustomNode::Null { .. }
                    | CustomNode::Alias { .. } => {
                        let k = resolve_alias_target(key, py, anchors, in_progress, schema)?;
                        if dict.set_item(k, &val).is_err() {
                            set_text_key(&dict, key, &val)?;
                        }
                    }
                    // A complex key (a nested mapping or sequence) has no hashable
                    // Python counterpart in this engine yet, and both reference libraries
                    // refuse it (`ConstructorError`), so it keeps the previous stand-in
                    // rather than being dropped. Named in ROADMAP as the remaining half of
                    // key fidelity.
                    _ => set_text_key(&dict, key, &val)?,
                }
            }
            Ok(dict.into_any().unbind())
        }
        CustomNode::Sequence { items, .. } => {
            let list = PyList::empty(py);
            for item in items {
                let val = resolve_alias_target(item, py, anchors, in_progress, schema)?;
                list.append(val).ok();
            }
            Ok(list.into_any().unbind())
        }
        CustomNode::Null { .. } => Ok(py.None()),
        CustomNode::Alias { .. } => Ok(py.None()),
    }
}

/// Put one pair in the dict under a stand-in key: the scalar's own text, or the debug
/// form of a node that has no scalar text at all (a nested key).
///
/// This is what keeps a mapping entry from being dropped: `set_item` fails on an
/// unhashable object, and losing the pair silently is worse than a key the user can
/// still see and re-read.
fn set_text_key(dict: &Bound<'_, PyDict>, key: &CustomNode, value: &Py<PyAny>) -> PyResult<()> {
    match key {
        CustomNode::Scalar { value: text, .. } => dict.set_item(text.as_ref(), value),
        _ => dict.set_item(format!("{key:?}"), value),
    }
}

/// Expand an `Alias` node against the anchor table, guarding against
/// reference cycles with a path-scoped `in_progress` set.
///
/// The guard is pushed only for the duration of the expansion, so sibling
/// references to the same anchor each get their own freshly built object.
/// Without the pop, the second `*x` in `a: &x 1 / b: *x / c: *x` would hit
/// the already-visited address and silently degrade to `None` (issue #163).
fn resolve_alias_target<'a>(
    node: &'a CustomNode,
    py: Python,
    anchors: &HashMap<&'a str, &'a CustomNode>,
    in_progress: &mut HashSet<usize>,
    schema: &Schema,
) -> PyResult<Py<PyAny>> {
    let CustomNode::Alias { name } = node else {
        return node_to_pyobject_inner(node, py, anchors, in_progress, schema);
    };
    let Some(target) = anchors.get(name.as_str()) else {
        return Ok(py.None());
    };
    let addr = std::ptr::addr_of!(*target) as usize;
    if !in_progress.insert(addr) {
        // Cycle: this anchor is already being expanded further up the
        // recursion path. Emit `None` so the walk terminates.
        return Ok(py.None());
    }
    let result = node_to_pyobject_inner(target, py, anchors, in_progress, schema);
    in_progress.remove(&addr);
    result
}

/// 将 `CustomNode` 转换为 Python 对象，不解析别名（别名节点返回 `None`）。
///
/// Thin wrapper over `node_to_pyobject_with_anchors` with an empty anchor
/// table: with no anchors registered, alias nodes fall through to `None` and
/// no cycle tracking is needed.
pub(crate) fn node_to_pyobject_simple(
    node: &CustomNode,
    py: Python,
    schema: &Schema,
) -> PyResult<Py<PyAny>> {
    let anchors: HashMap<&str, &CustomNode> = HashMap::new();
    let mut in_progress: HashSet<usize> = HashSet::new();
    node_to_pyobject_with_anchors(node, py, &anchors, &mut in_progress, schema)
}

/// Convert an AST to a Python object, resolving anchor references only when
/// the source text actually contains an `&`. Shared by every "source → native
/// value" entry point (`safe_load`/`safe_loads`/`YAML.safe_load*`/`to_dict`)
/// so the has-anchor fast path stays in one place.
pub(crate) fn node_to_pyobject_resolving_anchors(
    node: &CustomNode,
    py: Python,
    schema: &Schema,
    source_has_anchors: bool,
) -> PyResult<Py<PyAny>> {
    if source_has_anchors {
        let mut anchors = HashMap::new();
        collect_anchors(node, &mut anchors);
        let mut in_progress = HashSet::new();
        node_to_pyobject_with_anchors(node, py, &anchors, &mut in_progress, schema)
    } else {
        node_to_pyobject_simple(node, py, schema)
    }
}
