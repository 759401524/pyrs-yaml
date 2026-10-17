//! Conversion between `CustomNode` and Python objects, including alias
//! resolution and YAML type inference.

use crate::YamlTypeError;
use crate::ast::{CustomNode, ScalarStyle};
use crate::parser::yaml::registry;
use crate::parser::yaml::{Schema, YamlType};
use crate::py::type_registry;

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList, PyTuple};
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

/// 递归遍历 AST，按文档顺序把每个别名绑到它前面最近的那个锚点。
///
/// An alias names an anchor, and a name may be defined more than once in one document. YAML
/// resolves a reference to the **nearest preceding** definition, so
/// `a: &x 1` / `b: *x` / `c: &x 2` / `d: *x` reads 1 then 2. One table keyed by name cannot say
/// that: filling it in finishes before the walk starts, so every alias in the document sees the
/// *last* definition — which is what suite case `3GZX` measured against PyYAML. Recording the
/// binding while walking in document order answers that, and leaves the conversion walk free to
/// build a pair's value before its key because the answer is already written down.
///
/// An alias with no preceding definition of its name is absent from the table, which is the same
/// "no target" answer the projection already gave for a foreign or missing anchor; it now covers a
/// forward reference too, which YAML does not allow.
fn collect_anchor_targets<'a>(
    node: &'a CustomNode,
    defined: &mut HashMap<&'a str, &'a CustomNode>,
    targets: &mut HashMap<usize, &'a CustomNode>,
) {
    if let Some(name) = node.anchor() {
        defined.insert(name, node);
    }
    if let CustomNode::Alias { name } = node {
        if let Some(target) = defined.get(name.as_str()) {
            targets.insert(node_addr(node), *target);
        }
        return;
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs {
                collect_anchor_targets(key, defined, targets);
                collect_anchor_targets(value, defined, targets);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                collect_anchor_targets(item, defined, targets);
            }
        }
        _ => {}
    }
}

/// The identity an alias table is keyed by: the AST node itself, not its text.
///
/// Two `*x` nodes with the same name must be able to resolve apart, which is the whole point of
/// binding by position, so the key has to be the node. `resolve_alias_target` already names a
/// target node by address to guard reference cycles, and the tree is borrowed for the whole
/// projection, so the addresses stay valid for as long as the table is read.
fn node_addr(node: &CustomNode) -> usize {
    std::ptr::addr_of!(*node) as usize
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
    anchors: &HashMap<usize, &'a CustomNode>,
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
    anchors: &HashMap<usize, &'a CustomNode>,
    in_progress: &mut HashSet<usize>,
    schema: &Schema,
) -> PyResult<Py<PyAny>> {
    match node {
        CustomNode::Scalar {
            value, style, meta, ..
        } => {
            // An explicit tag states the type, so it is read first: the document's schema
            // resolves `1.20` to a float and `aGk=` to a string, and both answers contradict the
            // tag its author wrote. The non-specific `!` belongs to that rule rather than escaping
            // it - it states that no schema reading applies, which for a scalar is its own text.
            // A plugin registered on one of these tags no longer wins over the
            // tag itself - the standard reading is what both reference libraries apply.
            if let Some(t) = meta.tag()
                && let Some(tagged) = crate::py::tags::tagged_scalar(py, &t.to_string(), value)?
            {
                return Ok(tagged);
            }
            // Resolve the scalar value first, then try custom type conversion
            // so that `from_yaml` receives the resolved Python object.
            let py_obj = scalar_to_pyobject(py, value, style, schema)?;
            if let Some(t) = meta.tag()
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
                    } if meta.tag().is_none() => {
                        dict.set_item(scalar_to_pyobject(py, value, style, schema)?, &val)?;
                    }
                    CustomNode::Scalar { .. } | CustomNode::Null { .. } => {
                        let k = resolve_alias_target(key, py, anchors, in_progress, schema)?;
                        if dict.set_item(k, &val).is_err() {
                            set_text_key(&dict, key, &val)?;
                        }
                    }
                    // A sequence used as a key has a hashable Python form and ruamel.yaml uses
                    // it: the top level becomes a tuple, so `? [1, 2]` reads back as
                    // `{(1, 2): 'v'}`. Measured on 2026-10-14 against ruamel's safe loader, which
                    // gives `(1, 2)` for `? [1, 2]` and `(1, 'x')` for `? [1, "x"]`, while
                    // `? [[1], [2]]` and `? [{a: 1}]` stay refused by Python with its own
                    // `unhashable type: 'list'` / `'dict'` - the same two errors ruamel produces,
                    // because only the outer container is converted. PyYAML refuses all of them
                    // (`ConstructorError`), so this is a documented divergence toward the more
                    // informative library rather than a silent third behaviour.
                    CustomNode::Sequence { items, .. } => {
                        let mut parts = Vec::with_capacity(items.len());
                        for item in items {
                            parts.push(resolve_alias_target(
                                item,
                                py,
                                anchors,
                                in_progress,
                                schema,
                            )?);
                        }
                        dict.set_item(PyTuple::new(py, parts)?, &val)?;
                    }
                    // An alias in key position is decided by what it points at, not by the fact that
                    // it is an alias. Measured against PyYAML on this tree: `a: &b b` + `*b: a`
                    // constructs `{'a': 'b', 'b': 'a'}` — the suite spells the same answer as the
                    // expected JSON of E76Z and 26DV — while `a: &x [1, 2]` + `? *x` raises
                    // `ConstructorError` there too, because a list has no hashable form. So the
                    // resolved object is tried, and the refusal below is what an unhashable target
                    // (or an alias with no anchor, which must not become a `None` key) gets.
                    CustomNode::Alias { .. } if anchors.contains_key(&node_addr(key)) => {
                        let k = resolve_alias_target(key, py, anchors, in_progress, schema)?;
                        if dict.set_item(k, &val).is_err() {
                            set_text_key(&dict, key, &val)?;
                        }
                    }
                    // A nested mapping as a key has no hashable Python form and both reference
                    // libraries refuse it - PyYAML and ruamel both raise `ConstructorError:
                    // found unhashable key` for `? {a: 1}` and for `? ?`. It used to be replaced
                    // by the node's Rust debug text, which is worse than either failure: the key
                    // stops being the structure the reader parsed, and the stand-in is a plain
                    // string, so it re-emits stably and the corruption looks like a fixed point.
                    _ => return Err(unhashable_key_error(key)),
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

/// Put one pair in the dict under the key's own text.
///
/// Reached when `set_item` refused the resolved key. If the node is a scalar its source
/// text is a real, if narrower, rendering of what the document wrote, and the pair is kept.
/// Anything else has no scalar text to fall back to, and fabricating a key out of a debug
/// dump would silently change the document, so it gets `unhashable_key_error` instead.
fn set_text_key(dict: &Bound<'_, PyDict>, key: &CustomNode, value: &Py<PyAny>) -> PyResult<()> {
    match key {
        CustomNode::Scalar { value: text, .. } => dict.set_item(text.as_ref(), value),
        other => Err(unhashable_key_error(other)),
    }
}

/// The refusal for a key that has no hashable Python form.
///
/// The message names the shape and the one route that works. It deliberately does not offer the
/// other projections as escapes, because they refuse here too — measured on this tree: `to_json()`
/// raises `json-object-key` and `to_toml()` raises `toml-keys-must-be-scalars`. For a mapping or
/// sequence key the AST does hold it and the writer emits the `?` form (tested), and an alias key
/// whose anchor is missing has nothing to resolve to, which is why the alias wording covers both
/// shapes rather than claiming a target that may not exist.
fn unhashable_key_error(key: &CustomNode) -> pyo3::PyErr {
    let shape = match key {
        CustomNode::Mapping { .. } => "a mapping",
        CustomNode::Sequence { .. } => "a sequence",
        CustomNode::Alias { .. } => {
            "an alias whose resolved form is not a valid key \
                                     (a mapping, a sequence, or no anchor at all)"
        }
        _ => "a node",
    };
    YamlTypeError::new_err(format!(
        "found unhashable key: the YAML key is {shape}, which has no hashable Python form. \
         PyYAML and ruamel.yaml raise at this same point. Read the document with parse() and work \
         on the AST; to_json() and to_toml() refuse this key as well."
    ))
}

/// Expand an `Alias` node against the bindings the walk ordered, guarding against
/// reference cycles with a path-scoped `in_progress` set.
///
/// The guard is pushed only for the duration of the expansion, so sibling
/// references to the same anchor each get their own freshly built object.
/// Without the pop, the second `*x` in `a: &x 1 / b: *x / c: *x` would hit
/// the already-visited address and silently degrade to `None` (issue #163).
fn resolve_alias_target<'a>(
    node: &'a CustomNode,
    py: Python,
    anchors: &HashMap<usize, &'a CustomNode>,
    in_progress: &mut HashSet<usize>,
    schema: &Schema,
) -> PyResult<Py<PyAny>> {
    let CustomNode::Alias { .. } = node else {
        return node_to_pyobject_inner(node, py, anchors, in_progress, schema);
    };
    let Some(target) = anchors.get(&node_addr(node)).copied() else {
        return Ok(py.None());
    };
    let addr = node_addr(target);
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
    let anchors: HashMap<usize, &CustomNode> = HashMap::new();
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
        let mut defined = HashMap::new();
        let mut targets = HashMap::new();
        collect_anchor_targets(node, &mut defined, &mut targets);
        let mut in_progress = HashSet::new();
        node_to_pyobject_with_anchors(node, py, &targets, &mut in_progress, schema)
    } else {
        node_to_pyobject_simple(node, py, schema)
    }
}
