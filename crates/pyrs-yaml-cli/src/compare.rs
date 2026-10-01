//! Semantic tree walks behind `pyq diff` / `pyq merge`.
//!
//! Comparison is deliberately *semantic*: resolved scalar values (per the
//! selected schema), structure and tags — comments, quoting styles and
//! byte layout never appear in the output. Merge is right-biased deep
//! overlay (the `yq eval-all *+` shape): mappings recurse, sequences
//! append (or replace with `ArrayMode::Replace`), anything else the
//! right node wins with its style and comments intact.

use pyrs_json::key_text;
use pyrs_yaml_core::ast::{CustomNode, ScalarStyle};
use pyrs_yaml_core::parser::yaml::{Schema, YamlType};

/// Sequence policy for [`merge`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ArrayMode {
    /// Right items extend the left sequence (yq `*+` semantics).
    Append,
    /// Right sequence replaces the left wholesale.
    Replace,
}

/// One-line kind/value summary used in diff messages. Scalar text is
/// flattened (escaped newlines) and capped so a stray multi-line block
/// cannot flood a terminal.
fn render(node: &CustomNode) -> String {
    match node {
        CustomNode::Scalar { value, .. } => {
            let flat: String = value
                .chars()
                .flat_map(|c| c.escape_debug())
                .take(60)
                .collect();
            flat
        }
        CustomNode::Null { .. } => "null".to_string(),
        CustomNode::Alias { name } => format!("*{name}"),
        CustomNode::Mapping { .. } => "mapping".to_string(),
        CustomNode::Sequence { .. } => "sequence".to_string(),
    }
}

/// Structural equivalence for the recursion base case: same leaf kind,
/// same tag, and scalars resolving to the same typed value under
/// `schema`. Style/chomping/comments are formatting, not semantics.
fn sem_eq(a: &CustomNode, b: &CustomNode, schema: &Schema) -> bool {
    match (a, b) {
        (
            CustomNode::Scalar {
                value: v1,
                style: s1,
                meta: m1,
                ..
            },
            CustomNode::Scalar {
                value: v2,
                style: s2,
                meta: m2,
                ..
            },
        ) => {
            if m1.tag != m2.tag {
                return false;
            }
            // Only plain scalars are schema-resolved (YAML 1.2); quoted or
            // block styles are strings no matter what the text looks like.
            let t1 = if *s1 == ScalarStyle::Plain {
                schema.resolve(v1)
            } else {
                YamlType::Str(std::borrow::Cow::Borrowed(v1))
            };
            let t2 = if *s2 == ScalarStyle::Plain {
                schema.resolve(v2)
            } else {
                YamlType::Str(std::borrow::Cow::Borrowed(v2))
            };
            t1 == t2
        }
        (CustomNode::Null { meta: m1 }, CustomNode::Null { meta: m2 }) => m1.tag == m2.tag,
        (CustomNode::Alias { name: n1 }, CustomNode::Alias { name: n2 }) => n1 == n2,
        _ => false,
    }
}

/// Key text for path labelling / lookup; non-scalar keys (rare, legal in
/// YAML) fall back to a placeholder so the walk never panics.
fn key_label(key: &CustomNode) -> String {
    key_text(key).unwrap_or_else(|_| "<complex-key>".to_string())
}

fn find<'a>(
    pairs: &'a indexmap::IndexMap<CustomNode, CustomNode>,
    want: &str,
) -> Option<&'a CustomNode> {
    pairs
        .iter()
        .find(|(k, _)| key_label(k) == want)
        .map(|(_, v)| v)
}

/// Child path label; the document root (`.`) swallows its leading dot so
/// top-level keys read `.b`, not `..b`.
fn child(path: &str, label: &str) -> String {
    if path == "." {
        format!(".{label}")
    } else {
        format!("{path}.{label}")
    }
}

fn walk(path: &str, a: &CustomNode, b: &CustomNode, schema: &Schema, out: &mut Vec<String>) {
    match (a, b) {
        (CustomNode::Mapping { pairs: p1, .. }, CustomNode::Mapping { pairs: p2, .. }) => {
            for (k, v1) in p1.iter() {
                let label = key_label(k);
                let p = child(path, &label);
                match find(p2, &label) {
                    Some(v2) => walk(&p, v1, v2, schema, out),
                    None => out.push(format!("- {p}: {}", render(v1))),
                }
            }
            for (k, v2) in p2.iter() {
                let label = key_label(k);
                if find(p1, &label).is_none() {
                    out.push(format!("+ {}: {}", child(path, &label), render(v2)));
                }
            }
        }
        (CustomNode::Sequence { items: i1, .. }, CustomNode::Sequence { items: i2, .. }) => {
            for idx in 0..i1.len().max(i2.len()) {
                let p = format!("{path}[{idx}]");
                match (i1.get(idx), i2.get(idx)) {
                    (Some(x), Some(y)) => walk(&p, x, y, schema, out),
                    (Some(x), None) => out.push(format!("- {p}: {}", render(x))),
                    (None, Some(y)) => out.push(format!("+ {p}: {}", render(y))),
                    (None, None) => unreachable!("bounded by max(len)"),
                }
            }
        }
        _ => {
            if !sem_eq(a, b, schema) {
                out.push(format!("~ {path}: {} -> {}", render(a), render(b)));
            }
        }
    }
}

/// Differences between two documents; empty means semantically equal.
pub fn diff(a: &CustomNode, b: &CustomNode, schema: Schema) -> Vec<String> {
    let mut out = Vec::new();
    walk(".", a, b, &schema, &mut out);
    out
}

/// Deep right-biased overlay of `b` onto `a` (see module docs).
pub fn merge(a: &CustomNode, b: &CustomNode, arrays: ArrayMode) -> CustomNode {
    match (a, b) {
        (CustomNode::Mapping { .. }, CustomNode::Mapping { .. }) => {
            let mut out = a.clone();
            let CustomNode::Mapping { pairs, .. } = &mut out else {
                unreachable!("just matched")
            };
            let CustomNode::Mapping { pairs: bp, .. } = b else {
                unreachable!("just matched")
            };
            for (k, v2) in bp.iter() {
                let label = key_label(k);
                match pairs.iter().position(|(pk, _)| key_label(pk) == label) {
                    Some(idx) => {
                        let (_, old_val) = pairs.get_index_mut(idx).unwrap();
                        *old_val = match (&*old_val, v2) {
                            (CustomNode::Mapping { .. }, CustomNode::Mapping { .. })
                            | (CustomNode::Sequence { .. }, CustomNode::Sequence { .. }) => {
                                merge(&old_val.clone(), v2, arrays)
                            }
                            _ => v2.clone(),
                        };
                    }
                    None => {
                        pairs.insert(k.clone(), v2.clone());
                    }
                }
            }
            out
        }
        (CustomNode::Sequence { .. }, CustomNode::Sequence { .. }) => {
            if arrays == ArrayMode::Replace {
                return b.clone();
            }
            let mut out = a.clone();
            let CustomNode::Sequence { items, .. } = &mut out else {
                unreachable!("just matched")
            };
            let CustomNode::Sequence { items: bi, .. } = b else {
                unreachable!("just matched")
            };
            items.extend(bi.iter().cloned());
            out
        }
        _ => b.clone(),
    }
}
