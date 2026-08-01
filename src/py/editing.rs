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

/// A path segment: mapping key (String) or sequence index (i64).
///
/// Indexes follow Python semantics: negative values count from the end of the
/// sequence (`-1` = last element). They are normalized against the length at
/// navigation time via [`normalize_index`].
#[derive(Debug, Clone)]
pub enum Segment<'a> {
    Key(Cow<'a, str>),
    Index(i64),
}

impl<'a> Segment<'a> {
    /// Extract from a bound Python object: str → Key, int (possibly negative) → Index, else error.
    pub fn from_py(_py: Python, obj: &Bound<'_, PyAny>) -> PyResult<Segment<'a>> {
        if let Ok(s) = obj.extract::<String>() {
            Ok(Segment::Key(Cow::Owned(s)))
        } else if let Ok(i) = obj.extract::<i64>() {
            Ok(Segment::Index(i))
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(
                "segment must be str or int",
            ))
        }
    }
}

/// Normalize a possibly-negative sequence index against a length, following
/// Python semantics (`-1` = last element). Returns `None` when out of range
/// (including when the sequence is empty).
pub fn normalize_index(index: i64, len: usize) -> Option<usize> {
    let normalized = if index < 0 {
        i64::try_from(len).ok()?.checked_add(index)?
    } else {
        index
    };
    usize::try_from(normalized).ok().filter(|&i| i < len)
}

/// Parse a JSONPath-like string (`$.a.b`, `$.arr[0]`, `$.arr[-1]`, `$`) into
/// path segments for editing/querying. Mapping keys and sequence indexes are
/// interleaved freely; negative indexes are preserved for later normalization.
///
/// Wildcard (`*`) and deep-scan (`..`) paths are rejected because they target
/// more than one node and edits resolve to exactly one.
pub fn parse_path_segments(path: &str) -> Result<Vec<Segment<'_>>, String> {
    let rest = path.strip_prefix('$').unwrap_or(path);
    let rest = rest.strip_prefix('.').unwrap_or(rest);
    if rest.starts_with('.') {
        return Err("wildcard-or-deep-scan".to_string());
    }
    let mut segments = Vec::new();
    let mut chars = rest.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            '.' => {
                chars.next();
            }
            '[' => {
                chars.next();
                let mut num = String::new();
                if matches!(chars.peek(), Some('-')) {
                    num.push('-');
                    chars.next();
                }
                while matches!(chars.peek(), Some(d) if d.is_ascii_digit()) {
                    num.push(chars.next().expect("peeked digit"));
                }
                if chars.next() != Some(']') || num.is_empty() || num == "-" {
                    return Err(format!("invalid-index:{num}"));
                }
                let idx: i64 = num.parse().map_err(|_| format!("invalid-index:{num}"))?;
                segments.push(Segment::Index(idx));
            }
            '*' => return Err("wildcard-or-deep-scan".to_string()),
            _ => {
                let mut key = String::new();
                while let Some(&ch) = chars.peek() {
                    if matches!(ch, '.' | '[' | '*' | '$') {
                        break;
                    }
                    key.push(ch);
                    chars.next();
                }
                if key.is_empty() {
                    return Err("invalid-path".to_string());
                }
                segments.push(Segment::Key(Cow::Owned(key)));
            }
        }
    }
    Ok(segments)
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
                .get(
                    normalize_index(*i, items.len())
                        .ok_or_else(|| NavigateError::Missing(i.to_string()))?,
                )
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
                let idx = normalize_index(*i, items.len())
                    .ok_or_else(|| NavigateError::Missing(i.to_string()))?;
                items
                    .get_mut(idx)
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
    // Setting a path on an empty document auto-creates a mapping root so the
    // first key has somewhere to live (`set("a.b", 1)` on an empty doc).
    if matches!(node, CustomNode::Null { .. }) {
        *node = CustomNode::Mapping {
            pairs: IndexMap::new(),
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
        };
    }
    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let last = &last[0];
    let parent = navigate_mut(node, parent_segments).map_err(|e| match e {
        NavigateError::Missing(s) => format!("missing-path:{s}"),
        NavigateError::CannotDescend(t) => format!("cannot-descend-into-scalar:{t}"),
        NavigateError::NotContainer => "create-needs-mapping".to_string(),
    })?;
    let parent_is_alias = matches!(parent, CustomNode::Alias { .. });

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
            let idx = normalize_index(*i, items.len())
                .ok_or_else(|| format!("index-out-of-range-edit:{i}"))?;
            let target = items
                .get_mut(idx)
                .ok_or_else(|| format!("index-out-of-range-edit:{i}"))?;
            let mut v = new_value;
            if preserve_metadata {
                v = with_metadata_from(&v, target);
            }
            items[idx] = v;
            Ok(())
        }
        _ if parent_is_alias => Err("cannot-edit-alias".to_string()),
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
/// `index == len` appends; negative `index` counts from the end. The final
/// segment must resolve to a sequence.
pub fn insert_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    index: i64,
    value: CustomNode,
) -> Result<(), String> {
    let seq = navigate_mut(node, segments).map_err(nav_err)?;
    match seq {
        CustomNode::Sequence { items, .. } => {
            let insert_at = if index < 0 {
                i64::try_from(items.len())
                    .map_err(|_| "index-out-of-range-edit".to_string())?
                    .checked_add(index)
                    .ok_or_else(|| "index-out-of-range-edit".to_string())?
            } else {
                index
            };
            if insert_at < 0 || insert_at > items.len() as i64 {
                return Err("index-out-of-range-edit".to_string());
            }
            items.insert(insert_at as usize, value);
            Ok(())
        }
        _ if matches!(seq, CustomNode::Alias { .. }) => Err("cannot-edit-alias".to_string()),
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
        _ if matches!(seq, CustomNode::Alias { .. }) => Err("cannot-edit-alias".to_string()),
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
    let parent_is_alias = matches!(parent, CustomNode::Alias { .. });

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
            let idx = normalize_index(*i, items.len())
                .ok_or_else(|| "index-out-of-range-edit".to_string())?;
            items.remove(idx);
            Ok(())
        }
        _ if parent_is_alias => Err("cannot-edit-alias".to_string()),
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
    let parent_is_alias = matches!(parent, CustomNode::Alias { .. });
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
        _ if parent_is_alias => Err("cannot-edit-alias".to_string()),
        _ => Err("cannot-rename-complex-key".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse, yaml::YamlSchema};

    fn doc(yaml: &str) -> CustomNode {
        parse(yaml, YamlSchema::Core).unwrap()
    }

    #[test]
    fn normalize_index_python_semantics() {
        assert_eq!(normalize_index(0, 3), Some(0));
        assert_eq!(normalize_index(2, 3), Some(2));
        assert_eq!(normalize_index(3, 3), None);
        assert_eq!(normalize_index(-1, 3), Some(2));
        assert_eq!(normalize_index(-3, 3), Some(0));
        assert_eq!(normalize_index(-4, 3), None);
        assert_eq!(normalize_index(-1, 0), None);
        assert_eq!(normalize_index(0, 0), None);
    }

    #[test]
    fn parse_path_segments_basic() {
        let segs = parse_path_segments("$.a.b").unwrap();
        assert_eq!(segs.len(), 2);
        assert!(matches!(&segs[0], Segment::Key(k) if k == "a"));
        assert!(matches!(&segs[1], Segment::Key(k) if k == "b"));

        let segs = parse_path_segments("$.arr[0]").unwrap();
        assert!(matches!(&segs[0], Segment::Key(k) if k == "arr"));
        assert!(matches!(&segs[1], Segment::Index(0)));

        let segs = parse_path_segments("$.arr[-1]").unwrap();
        assert!(matches!(&segs[1], Segment::Index(-1)));

        assert!(parse_path_segments("$").unwrap().is_empty());
    }

    #[test]
    fn parse_path_segments_rejects_wildcard_and_deep_scan() {
        assert!(parse_path_segments("$.a[*]").is_err());
        assert!(parse_path_segments("$..a").is_err());
        assert!(parse_path_segments("$[0").is_err());
        assert!(parse_path_segments("$[]").is_err());
    }

    #[test]
    fn set_path_negative_index() {
        let mut node = doc("- a\n- b\n- c");
        set_path(
            &mut node,
            &[Segment::Index(-1)],
            CustomNode::plain_scalar("z"),
            true,
        )
        .unwrap();
        assert_eq!(crate::serializer::to_yaml(&node), "- a\n- b\n- z\n");
    }

    #[test]
    fn set_path_out_of_range_negative() {
        let mut node = doc("- a\n- b");
        assert!(set_path(
            &mut node,
            &[Segment::Index(-3)],
            CustomNode::plain_scalar("z"),
            true,
        )
        .is_err());
    }

    #[test]
    fn set_path_creates_mapping_root_on_empty_document() {
        let mut node = doc("");
        set_path(
            &mut node,
            &[Segment::Key(Cow::Borrowed("a"))],
            CustomNode::plain_scalar("1"),
            true,
        )
        .unwrap();
        assert_eq!(crate::serializer::to_yaml(&node), "a: 1\n");
    }

    #[test]
    fn set_path_through_alias_is_rejected() {
        let mut node = doc("base: &b\n  x: 1\ncopy: *b");
        let err = set_path(
            &mut node,
            &[
                Segment::Key(Cow::Borrowed("copy")),
                Segment::Key(Cow::Borrowed("y")),
            ],
            CustomNode::plain_scalar("2"),
            true,
        )
        .unwrap_err();
        assert_eq!(err, "cannot-edit-alias");
    }

    #[test]
    fn delete_path_negative_index() {
        let mut node = doc("- a\n- b\n- c");
        delete_path(&mut node, &[Segment::Index(-2)]).unwrap();
        assert_eq!(crate::serializer::to_yaml(&node), "- a\n- c\n");
    }

    #[test]
    fn insert_path_negative_index() {
        let mut node = doc("- a\n- b\n- c");
        insert_path(&mut node, &[], -1, CustomNode::plain_scalar("z")).unwrap();
        assert_eq!(crate::serializer::to_yaml(&node), "- a\n- b\n- z\n- c\n");
    }

    #[test]
    fn serializer_compact_sequence_item() {
        let node = doc("servers:\n  - host: a");
        assert_eq!(crate::serializer::to_yaml(&node), "servers:\n  - host: a\n");
    }

    #[test]
    fn serializer_compact_multi_key_item() {
        let node = doc("- host: a\n  port: 8080");
        assert_eq!(
            crate::serializer::to_yaml(&node),
            "- host: a\n  port: 8080\n"
        );
    }

    #[test]
    fn serializer_compact_skips_metadata_items() {
        let node = doc("- name: &x anchor\n  value: 1");
        let out = crate::serializer::to_yaml(&node);
        assert!(out.contains("&x"));
    }

    #[test]
    fn serializer_compact_skips_block_nested_mappings() {
        let node = doc("- top:\n    inner: 1");
        assert_eq!(
            crate::serializer::to_yaml(&node),
            "- \n  top:\n    inner: 1\n"
        );
    }
}
