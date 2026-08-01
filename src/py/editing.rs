//! Pure-Rust path navigation and validate-then-mutate helpers for in-place
//! editing of the `CustomNode` AST.
//!
//! These helpers are deliberately free of PyO3 types except where noted — the
//! `_*_path` methods on `YamlDocument` (src/py/mod.rs) hold the GIL, convert
//! Python path segments / values, then call into these pure functions inside
//! `py.detach`.

use crate::ast::{CustomNode, ScalarStyle};
use indexmap::IndexMap;
use pyo3::prelude::*;
use std::borrow::Cow;

/// Compare two nodes by content only, ignoring metadata (comment/anchor/tag/style).
///
/// Used for mapping-key lookup so that a key with an attached inline comment
/// still matches a bare `plain_scalar(key)` segment. Scalar keys compare by
/// value; complex keys compare structurally (recursively, metadata-insensitive).
pub fn key_eq(a: &CustomNode, b: &CustomNode) -> bool {
    match (a, b) {
        (CustomNode::Scalar { value: av, .. }, CustomNode::Scalar { value: bv, .. }) => av == bv,
        (CustomNode::Null { .. }, CustomNode::Null { .. }) => true,
        (CustomNode::Mapping { pairs: ap, .. }, CustomNode::Mapping { pairs: bp, .. }) => {
            ap.len() == bp.len()
                && ap
                    .iter()
                    .all(|(k, v)| bp.iter().any(|(bk, bv)| key_eq(k, bk) && key_eq(v, bv)))
        }
        (CustomNode::Sequence { items: ai, .. }, CustomNode::Sequence { items: bi, .. }) => {
            ai.len() == bi.len() && ai.iter().zip(bi).all(|(x, y)| key_eq(x, y))
        }
        _ => false,
    }
}

/// Locate a mapping value by key content (metadata-insensitive).
pub fn mapping_get_mut<'a>(
    pairs: &'a mut IndexMap<CustomNode, CustomNode>,
    key: &CustomNode,
) -> Option<&'a mut CustomNode> {
    let idx = pairs.iter().position(|(k, _)| key_eq(k, key))?;
    pairs.get_index_mut(idx).map(|(_, v)| v)
}

/// Stored key index for a content-matching mapping key (metadata-insensitive).
pub fn mapping_key_index(
    pairs: &IndexMap<CustomNode, CustomNode>,
    key: &CustomNode,
) -> Option<usize> {
    pairs.iter().position(|(k, _)| key_eq(k, key))
}

/// A path segment: mapping key (String) or sequence index (usize).
#[derive(Debug, Clone)]
pub enum Segment<'a> {
    Key(Cow<'a, str>),
    Index(usize),
}

impl<'a> Segment<'a> {
    /// Extract from a bound Python object: str → Key, int (non-negative) → Index, else error.
    pub fn from_py(_py: Python, obj: &Bound<'_, PyAny>) -> PyResult<Segment<'a>> {
        if let Ok(s) = obj.extract::<String>() {
            Ok(Segment::Key(Cow::Owned(s)))
        } else if let Ok(i) = obj.extract::<i64>() {
            if i < 0 {
                Err(pyo3::exceptions::PyValueError::new_err(
                    "segment must be a non-negative integer",
                ))
            } else {
                Ok(Segment::Index(i as usize))
            }
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(
                "segment must be str or non-negative int",
            ))
        }
    }
}

/// Navigation failure modes, mapped to i18n keys at the call site.
#[derive(Debug)]
pub enum NavigateError {
    /// Segment not found / index out of bounds.
    Missing(String),
    /// A scalar/alias leaf was reached with remaining segments.
    CannotDescend(String),
    /// The operation requires a mapping/sequence parent but the node is not one.
    NotContainer,
}

/// Navigate to the final node (immutable).
pub fn navigate<'a>(
    node: &'a CustomNode,
    segments: &[Segment<'_>],
) -> Result<&'a CustomNode, NavigateError> {
    let mut cur = node;
    for seg in segments {
        cur = match (cur, seg) {
            (CustomNode::Mapping { pairs, .. }, Segment::Key(k)) => pairs
                .iter()
                .find(|(knode, _)| match knode {
                    CustomNode::Scalar { value, .. } => value == k.as_ref(),
                    _ => false,
                })
                .map(|(_, v)| v)
                .ok_or_else(|| NavigateError::Missing(k.to_string()))?,
            (CustomNode::Sequence { items, .. }, Segment::Index(i)) => items
                .get(*i)
                .ok_or_else(|| NavigateError::Missing(i.to_string()))?,
            (_, Segment::Key(k)) => return Err(NavigateError::CannotDescend(k.to_string())),
            (_, Segment::Index(i)) => return Err(NavigateError::CannotDescend(i.to_string())),
        };
    }
    Ok(cur)
}

/// Navigate to the final node (mutable).
pub fn navigate_mut<'a>(
    node: &'a mut CustomNode,
    segments: &[Segment<'_>],
) -> Result<&'a mut CustomNode, NavigateError> {
    let mut cur = node;
    for seg in segments {
        cur = match cur {
            CustomNode::Mapping { pairs, .. } => {
                let k = match seg {
                    Segment::Key(k) => k,
                    Segment::Index(i) => return Err(NavigateError::CannotDescend(i.to_string())),
                };
                let key_node = CustomNode::plain_scalar(k.as_ref());
                mapping_get_mut(pairs, &key_node)
                    .ok_or_else(|| NavigateError::Missing(k.to_string()))?
            }
            CustomNode::Sequence { items, .. } => {
                let i = match seg {
                    Segment::Index(i) => i,
                    Segment::Key(k) => return Err(NavigateError::CannotDescend(k.to_string())),
                };
                items
                    .get_mut(*i)
                    .ok_or_else(|| NavigateError::Missing(i.to_string()))?
            }
            _ => {
                return Err(match seg {
                    Segment::Key(k) => NavigateError::CannotDescend(k.to_string()),
                    Segment::Index(i) => NavigateError::CannotDescend(i.to_string()),
                })
            }
        };
    }
    Ok(cur)
}

/// Copy comment/anchor/tag/flow_style/chomping from `src` (the OLD target) onto
/// `target` (the NEW value), keeping the new node's scalar value / contents.
pub fn with_metadata_from(target: &CustomNode, src: &CustomNode) -> CustomNode {
    match (target, src) {
        (
            CustomNode::Scalar { value, style, .. },
            CustomNode::Scalar {
                style: src_style,
                comment,
                anchor,
                tag,
                chomping,
                ..
            },
        ) => {
            let new_style = if *style == ScalarStyle::Plain
                && *src_style != ScalarStyle::Plain
                && !needs_quoting(value)
            {
                src_style.clone()
            } else {
                style.clone()
            };
            CustomNode::Scalar {
                value: value.clone(),
                style: new_style,
                comment: comment.clone(),
                anchor: anchor.clone(),
                tag: tag.clone(),
                chomping: chomping.clone(),
            }
        }
        (
            CustomNode::Mapping {
                pairs, flow_style, ..
            },
            CustomNode::Mapping {
                comment,
                anchor,
                tag,
                flow_style: src_flow_style,
                ..
            },
        ) => CustomNode::Mapping {
            pairs: pairs.clone(),
            comment: comment.clone(),
            anchor: anchor.clone(),
            tag: tag.clone(),
            flow_style: *flow_style || *src_flow_style,
        },
        (
            CustomNode::Sequence {
                items, flow_style, ..
            },
            CustomNode::Sequence {
                comment,
                anchor,
                tag,
                flow_style: src_flow_style,
                ..
            },
        ) => CustomNode::Sequence {
            items: items.clone(),
            comment: comment.clone(),
            anchor: anchor.clone(),
            tag: tag.clone(),
            flow_style: *flow_style || *src_flow_style,
        },
        (
            CustomNode::Null { .. },
            CustomNode::Null {
                comment,
                anchor,
                tag,
            },
        ) => CustomNode::Null {
            comment: comment.clone(),
            anchor: anchor.clone(),
            tag: tag.clone(),
        },
        _ => target.clone(),
    }
}

fn needs_quoting(value: &str) -> bool {
    value.is_empty()
        || value
            .chars()
            .any(|c| c.is_whitespace() || ":{}[],&#*!|>".contains(c))
}

/// Extract the scalar value of a node (used by with_metadata_from).
#[allow(dead_code)]
fn target_scalar_value(node: &CustomNode) -> String {
    match node {
        CustomNode::Scalar { value, .. } => value.clone(),
        CustomNode::Null { .. } => "null".to_string(),
        _ => node_to_plain(node),
    }
}

#[allow(dead_code)]
fn node_to_plain(node: &CustomNode) -> String {
    crate::serializer::to_yaml(node)
        .trim_end()
        .trim_start_matches("- ")
        .to_string()
}

/// Set the value at `segments`. Final segment missing in a mapping CREATES the
/// pair; intermediate missing errors; parent must be a mapping for create.
///
/// Returns an i18n-key+detail string on error, e.g. `"missing-path:a"` or
/// `"index-out-of-range-edit:3"`.
pub fn set_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    new_value: CustomNode,
    preserve_metadata: bool,
) -> Result<(), String> {
    if segments.is_empty() {
        *node = new_value;
        return Ok(());
    }
    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let last = &last[0];
    let parent = navigate_mut(node, parent_segments).map_err(|e| match e {
        NavigateError::Missing(s) => format!("missing-path:{s}"),
        NavigateError::CannotDescend(t) => format!("cannot-descend-into-scalar:{t}"),
        NavigateError::NotContainer => "create-needs-mapping".to_string(),
    })?;

    match (parent, last) {
        (CustomNode::Mapping { pairs, .. }, Segment::Key(k)) => {
            let key_node = CustomNode::plain_scalar(k.as_ref());
            match mapping_key_index(pairs, &key_node) {
                Some(idx) => {
                    // Insert under the STORED key node so metadata (e.g. inline
                    // comment) on the key itself survives replacement.
                    let stored_key = pairs.get_index(idx).map(|(k, _)| k.clone());
                    let stored_key = match stored_key {
                        Some(k) => k,
                        None => key_node.clone(),
                    };
                    let target = pairs.get_index_mut(idx).map(|(_, v)| v);
                    if let Some(target) = target {
                        let mut v = new_value;
                        if preserve_metadata {
                            v = with_metadata_from(&v, target);
                        }
                        pairs.insert(stored_key, v);
                    } else {
                        pairs.insert(stored_key, new_value);
                    }
                }
                None => {
                    pairs.insert(key_node, new_value);
                }
            }
            Ok(())
        }
        (CustomNode::Sequence { items, .. }, Segment::Index(i)) => {
            let idx = *i;
            let target = items
                .get_mut(idx)
                .ok_or_else(|| format!("index-out-of-range-edit:{idx}"))?;
            let mut v = new_value;
            if preserve_metadata {
                v = with_metadata_from(&v, target);
            }
            items[idx] = v;
            Ok(())
        }
        _ => Err("create-needs-mapping".to_string()),
    }
}

/// Map a navigation failure to its i18n-key-prefixed message.
fn nav_err(e: NavigateError) -> String {
    match e {
        NavigateError::Missing(s) => format!("missing-path:{s}"),
        NavigateError::CannotDescend(t) => format!("cannot-descend-into-scalar:{t}"),
        NavigateError::NotContainer => "create-needs-mapping".to_string(),
    }
}

/// Insert `value` at `index` in the sequence reached by `segments`.
/// `index == len` appends. The final segment must resolve to a sequence.
pub fn insert_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    index: usize,
    value: CustomNode,
) -> Result<(), String> {
    let seq = navigate_mut(node, segments).map_err(nav_err)?;
    match seq {
        CustomNode::Sequence { items, .. } => {
            if index > items.len() {
                return Err("index-out-of-range-edit".to_string());
            }
            items.insert(index, value);
            Ok(())
        }
        _ => Err("not-a-sequence".to_string()),
    }
}

/// Append `value` to the sequence reached by `segments`.
pub fn append_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    value: CustomNode,
) -> Result<(), String> {
    let seq = navigate_mut(node, segments).map_err(nav_err)?;
    match seq {
        CustomNode::Sequence { items, .. } => {
            items.push(value);
            Ok(())
        }
        _ => Err("not-a-sequence".to_string()),
    }
}

/// Delete the node reached by `segments`. The final segment is removed from
/// its parent. Deleting the root (empty segments) is an error.
pub fn delete_path(node: &mut CustomNode, segments: &[Segment<'_>]) -> Result<(), String> {
    if segments.is_empty() {
        return Err("edit-error".to_string());
    }
    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let last = &last[0];
    let parent = navigate_mut(node, parent_segments).map_err(nav_err)?;

    match (parent, last) {
        (CustomNode::Mapping { pairs, .. }, Segment::Key(k)) => {
            let key_node = CustomNode::plain_scalar(k.as_ref());
            let idx =
                mapping_key_index(pairs, &key_node).ok_or_else(|| "missing-path".to_string())?;
            // shift_remove preserves mapping order (swap_remove would reorder).
            pairs.shift_remove_index(idx);
            Ok(())
        }
        (CustomNode::Sequence { items, .. }, Segment::Index(i)) => {
            if *i >= items.len() {
                return Err("index-out-of-range-edit".to_string());
            }
            items.remove(*i);
            Ok(())
        }
        _ => Err("missing-path".to_string()),
    }
}

/// Rename a mapping key. The value node (with its comment/anchor/tag) is
/// untouched — metadata preservation for free. Complex (non-scalar) keys and
/// the root are rejected.
pub fn rename_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    new_key: &str,
) -> Result<(), String> {
    if segments.is_empty() {
        return Err("cannot-rename-root".to_string());
    }
    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let last = &last[0];
    let parent = navigate_mut(node, parent_segments).map_err(nav_err)?;
    match (parent, last) {
        (CustomNode::Mapping { pairs, .. }, Segment::Key(old_key)) => {
            let old_key_node = CustomNode::plain_scalar(old_key.as_ref());
            let idx = mapping_key_index(pairs, &old_key_node)
                .ok_or_else(|| "missing-path".to_string())?;
            let key_node = pairs.get_index(idx).map(|(k, _)| k);
            let is_scalar = matches!(
                key_node,
                Some(CustomNode::Scalar { .. }) | Some(CustomNode::Null { .. })
            );
            if !is_scalar {
                return Err("cannot-rename-complex-key".to_string());
            }
            // Reject renaming onto an existing key: indexmap's shift_insert
            // would overwrite the existing entry's value, silently losing it.
            // The check runs BEFORE shift_remove so a failed rename leaves the
            // document untouched (atomicity). Renaming a key to itself is a
            // no-op and stays allowed.
            let new_key_node = CustomNode::plain_scalar(new_key.to_string());
            if !key_eq(&new_key_node, &old_key_node)
                && mapping_key_index(pairs, &new_key_node).is_some()
            {
                return Err("rename-key-exists".to_string());
            }
            // Keys are immutable in IndexMap (mutation would break hashing);
            // shift_remove preserves order, then re-insert at the SAME index
            // under the new key with the original value node (comment/
            // anchor/tag untouched).
            let value_node = pairs
                .shift_remove_index(idx)
                .map(|(_, v)| v)
                .ok_or_else(|| "missing-path".to_string())?;
            pairs.shift_insert(idx, new_key_node, value_node);
            Ok(())
        }
        _ => Err("cannot-rename-complex-key".to_string()),
    }
}
