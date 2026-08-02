//! Pure-Rust path navigation and validate-then-mutate helpers for in-place
//! editing of the `CustomNode` AST.
//!
//! These helpers are deliberately free of PyO3 types except where noted — the
//! `_*_path` methods on `YamlDocument` (src/py/mod.rs) hold the GIL, convert
//! Python path segments / values, then call into these pure functions inside
//! `py.detach`.

use crate::ast::{CustomNode, ScalarStyle};
use crate::parser::yaml::compute_line_offsets;
use indexmap::IndexMap;
use pyo3::prelude::*;
use std::borrow::Cow;
use std::ops::Range;

/// The splice unit an edit primitive produces: how a single mutation maps
/// onto the ORIGINAL source text (byte coordinates, line-aligned).
#[derive(Debug, Clone, PartialEq)]
pub enum DirtyKind {
    /// Regenerate the pair/item/root text at this byte range (line-aligned,
    /// includes the trailing newline), then splice the serialized text in.
    Region { range: Range<usize>, indent: usize },
    /// Insert `text` at byte offset (original source space).
    Insert { at: usize, text: String },
    /// Remove bytes; range extends upward over the node's preceding
    /// standalone comment lines.
    Delete { range: Range<usize> },
}

/// Describes one edit in splice coordinates. `eligible` is false when the
/// touched node or any ancestor is flow-style, or any involved node's
/// `source_range` is `None` (merged keys, aliases, programmatic AST) — the
/// splice layer rejects ineligible units and falls back to full serialize.
#[derive(Debug, Clone, PartialEq)]
pub struct DirtyUnit {
    pub kind: DirtyKind,
    pub eligible: bool,
}

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
                source_range: None,
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
            source_range: None,
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
            source_range: None,
        },
        (
            CustomNode::Null { .. },
            CustomNode::Null {
                comment,
                anchor,
                tag,
                ..
            },
        ) => CustomNode::Null {
            comment: comment.clone(),
            anchor: anchor.clone(),
            tag: tag.clone(),
            source_range: None,
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
    source: &str,
) -> Result<DirtyUnit, String> {
    let line_offsets = compute_line_offsets(source);

    if segments.is_empty() {
        let eligible = eligible_path(&path_nodes(node, segments).map_err(nav_err)?);
        *node = new_value;
        return Ok(DirtyUnit {
            kind: DirtyKind::Region {
                range: 0..source.len(),
                indent: 0,
            },
            eligible,
        });
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
            source_range: None,
        };
    }
    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let last = &last[0];
    let depth = segments.len().saturating_sub(1);

    // D4: eligibility and ranges are computed on the PRE-mutation tree.
    let (eligible, compact_override) =
        precompute(node, segments, parent_segments, &line_offsets, source);

    let parent = navigate_mut(node, parent_segments).map_err(nav_err)?;
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
                    // Capture the old pair span BEFORE mutation.
                    let (old_key_start, old_value_end, old_indent) = pairs
                        .get_index(idx)
                        .map(|(k, v)| {
                            let s = k.source_range().map(|r| r.start).unwrap_or(0);
                            let e = v.source_range().map(|r| r.end).unwrap_or(s);
                            (s, e, line_indent(&line_offsets, source, s))
                        })
                        .unwrap_or((0, 0, 0));
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
                    Ok(DirtyUnit {
                        kind: region_unit(
                            old_key_start,
                            old_value_end,
                            old_indent,
                            &line_offsets,
                            source,
                            compact_override,
                        ),
                        eligible,
                    })
                }
                None => {
                    // Setting a new key creates a pair: insert it at the end of
                    // the mapping, on a fresh line after the last pair.
                    let (at, indent) = match pairs.iter().last() {
                        Some((lk, lv)) => {
                            let key_start = lk.source_range().map(|r| r.start).unwrap_or(0);
                            let val_end = lv.source_range().map(|r| r.end).unwrap_or(key_start);
                            (
                                line_end(&line_offsets, val_end, source.len()),
                                line_indent(&line_offsets, source, key_start),
                            )
                        }
                        None => (0, 0),
                    };
                    let text = crate::serializer::pair_to_string(
                        &key_node, &new_value, indent, true, depth,
                    )?;
                    pairs.insert(key_node, new_value);
                    Ok(DirtyUnit {
                        kind: DirtyKind::Insert { at, text },
                        eligible,
                    })
                }
            }
        }
        (CustomNode::Sequence { items, .. }, Segment::Index(i)) => {
            let idx = normalize_index(*i, items.len())
                .ok_or_else(|| format!("index-out-of-range-edit:{i}"))?;
            let item_range = items[idx].source_range().cloned();
            let target = items
                .get_mut(idx)
                .ok_or_else(|| format!("index-out-of-range-edit:{i}"))?;
            let mut v = new_value;
            if preserve_metadata {
                v = with_metadata_from(&v, target);
            }
            items[idx] = v;
            let raw = item_range.unwrap_or(0..0);
            let raw_start = raw.start;
            let range = line_aligned(raw, &line_offsets, source);
            let indent = line_indent(&line_offsets, source, raw_start);
            Ok(DirtyUnit {
                kind: DirtyKind::Region { range, indent },
                eligible,
            })
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

// ---- line / range helpers (original-source byte coordinates) ----

/// Index into `line_offsets` of the line containing `byte_offset`.
fn line_index_of(line_offsets: &[usize], byte_offset: usize) -> usize {
    match line_offsets.binary_search(&byte_offset) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    }
}

/// Byte offset of the start of the line containing `byte_offset`.
fn line_start(line_offsets: &[usize], byte_offset: usize) -> usize {
    line_offsets[line_index_of(line_offsets, byte_offset)]
}

/// Byte offset just past the end of the line containing `byte_offset`,
/// including its trailing newline (`text_len` for a newline-less last line).
fn line_end(line_offsets: &[usize], byte_offset: usize, text_len: usize) -> usize {
    let idx = line_index_of(line_offsets, byte_offset);
    line_offsets.get(idx + 1).copied().unwrap_or(text_len)
}

/// Number of leading spaces on the line containing `byte_offset`.
fn line_indent(line_offsets: &[usize], text: &str, byte_offset: usize) -> usize {
    let start = line_start(line_offsets, byte_offset);
    let bytes = text.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    i - start
}

/// Expand `range` to whole lines: start back to its line start, end forward
/// to its line end (including the trailing newline, or EOF).
fn line_aligned(range: Range<usize>, line_offsets: &[usize], text: &str) -> Range<usize> {
    let start = line_start(line_offsets, range.start);
    let end = line_end(line_offsets, range.end, text.len());
    start..end
}

/// Extend a delete range upward over immediately-preceding standalone
/// comment lines (first non-space char is `#`), matching comment.rs
/// own-line attachment and full-serialize drop semantics.
fn extend_delete_over_comments(
    mut range: Range<usize>,
    line_offsets: &[usize],
    text: &str,
) -> Range<usize> {
    let bytes = text.as_bytes();
    let mut target_start = line_start(line_offsets, range.start);
    loop {
        if target_start == 0 {
            break;
        }
        let prev_start = line_start(line_offsets, target_start - 1);
        let line = &bytes[prev_start..target_start];
        if line.iter().find(|&&b| b != b' ').copied() == Some(b'#') {
            range.start = prev_start;
            target_start = prev_start;
        } else {
            break;
        }
    }
    range
}

// ---- navigation path / eligibility ----

/// Navigate to the final node, collecting every node on the path from the
/// root (inclusive) to the target (inclusive). Uses the same key lookup
/// semantics as `navigate_mut` (`key_eq`, so complex keys match too).
fn path_nodes<'a>(
    node: &'a CustomNode,
    segments: &[Segment<'_>],
) -> Result<Vec<&'a CustomNode>, NavigateError> {
    let mut path = Vec::with_capacity(segments.len() + 1);
    let mut cur = node;
    path.push(cur);
    for seg in segments {
        cur = match (cur, seg) {
            (CustomNode::Mapping { pairs, .. }, Segment::Key(k)) => {
                let key_node = CustomNode::plain_scalar(k.as_ref());
                let idx = mapping_key_index(pairs, &key_node)
                    .ok_or_else(|| NavigateError::Missing(k.to_string()))?;
                pairs
                    .get_index(idx)
                    .map(|(_, v)| v)
                    .ok_or_else(|| NavigateError::Missing(k.to_string()))?
            }
            (CustomNode::Sequence { items, .. }, Segment::Index(i)) => items
                .get(
                    normalize_index(*i, items.len())
                        .ok_or_else(|| NavigateError::Missing(i.to_string()))?,
                )
                .ok_or_else(|| NavigateError::Missing(i.to_string()))?,
            (_, Segment::Key(k)) => return Err(NavigateError::CannotDescend(k.to_string())),
            (_, Segment::Index(i)) => return Err(NavigateError::CannotDescend(i.to_string())),
        };
        path.push(cur);
    }
    Ok(path)
}

fn node_is_flow(node: &CustomNode) -> bool {
    match node {
        CustomNode::Mapping { flow_style, .. } | CustomNode::Sequence { flow_style, .. } => {
            *flow_style
        }
        _ => false,
    }
}

/// P4 eligibility: false when any node on the path (root → target) is
/// flow-style or lacks a `source_range`, or when the target's direct
/// children lack one (merged keys / aliases hiding inside a container).
fn eligible_path(path: &[&CustomNode]) -> bool {
    for (i, node) in path.iter().enumerate() {
        if node_is_flow(node) || node.source_range().is_none() {
            return false;
        }
        if i == path.len() - 1 {
            match node {
                CustomNode::Mapping { pairs, .. }
                    if pairs
                        .iter()
                        .any(|(k, v)| k.source_range().is_none() || v.source_range().is_none()) =>
                {
                    return false;
                }
                CustomNode::Sequence { items, .. }
                    if items.iter().any(|it| it.source_range().is_none()) =>
                {
                    return false;
                }
                _ => {}
            }
        }
    }
    true
}

/// P2: the innermost node on the path whose parent is a `Sequence` and which
/// serializes in the compact `- key: value` form. When present, the edit
/// covers the WHOLE item and regeneration goes through `write_sequence_item`.
fn compact_item_ancestor<'a>(path: &'a [&'a CustomNode]) -> Option<&'a CustomNode> {
    for i in 1..path.len() {
        let node = path[i];
        let parent = path[i - 1];
        if matches!(parent, CustomNode::Sequence { .. }) && crate::serializer::is_compact_item(node)
        {
            return Some(node);
        }
    }
    None
}

/// Build the `Region` kind for a pair/item edit. When the edit falls inside a
/// compact sequence item (P2), the region covers the WHOLE item so the
/// regeneration (via `write_sequence_item`) re-emits every key.
fn region_unit(
    key_start: usize,
    value_end: usize,
    pair_indent: usize,
    line_offsets: &[usize],
    source: &str,
    compact_override: Option<(Range<usize>, usize)>,
) -> DirtyKind {
    if let Some((range, indent)) = compact_override {
        DirtyKind::Region { range, indent }
    } else {
        let range = line_aligned(key_start..value_end, line_offsets, source);
        DirtyKind::Region {
            range,
            indent: pair_indent,
        }
    }
}

/// P4 eligibility and the P2 compact-item override, computed from immutable
/// borrows only so `navigate_mut` can run afterwards. Runs BEFORE the
/// mutation (D4). The override is a value (`Range` + indent), never a
/// reference into the tree.
fn precompute(
    node: &CustomNode,
    segments: &[Segment<'_>],
    parent_segments: &[Segment<'_>],
    line_offsets: &[usize],
    source: &str,
) -> (bool, Option<(Range<usize>, usize)>) {
    let path = match path_nodes(node, segments) {
        Ok(p) => p,
        Err(_) => path_nodes(node, parent_segments).unwrap_or_default(),
    };
    let eligible = eligible_path(&path);
    let compact_override = compact_item_ancestor(&path)
        .and_then(|item| item.source_range().cloned())
        .map(|r| {
            (
                line_aligned(r.clone(), line_offsets, source),
                line_indent(line_offsets, source, r.start),
            )
        });
    (eligible, compact_override)
}
/// Insert `value` at `index` in the sequence reached by `segments`.
/// `index == len` appends; negative `index` counts from the end. The final
/// segment must resolve to a sequence.
pub fn insert_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    index: i64,
    value: CustomNode,
    source: &str,
) -> Result<DirtyUnit, String> {
    let line_offsets = compute_line_offsets(source);
    let depth = segments.len().saturating_sub(1);
    // D4: eligibility is computed on the PRE-mutation tree.
    let eligible = {
        let path = path_nodes(node, segments).map_err(nav_err)?;
        eligible_path(&path)
    };
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
            if items.is_empty() {
                // No existing item to anchor the insert to; the caller keeps
                // the placeholder and falls back to full serialization.
                items.insert(0, value);
                return Ok(DirtyUnit {
                    kind: DirtyKind::Insert {
                        at: 0,
                        text: String::new(),
                    },
                    eligible: false,
                });
            }
            let insert_at = insert_at as usize;
            // Compute the anchor and indent BEFORE mutating the items.
            let (at, indent, is_last) = if insert_at < items.len() {
                let next_item = items[insert_at].source_range().cloned().unwrap_or(0..0);
                (
                    line_start(&line_offsets, next_item.start),
                    line_indent(&line_offsets, source, next_item.start),
                    false,
                )
            } else {
                let last_item = items[items.len() - 1]
                    .source_range()
                    .cloned()
                    .unwrap_or(0..0);
                (
                    line_end(&line_offsets, last_item.end, source.len()),
                    line_indent(&line_offsets, source, last_item.start),
                    true,
                )
            };
            let text = crate::serializer::item_to_string(&value, indent, is_last, depth)?;
            items.insert(insert_at, value);
            Ok(DirtyUnit {
                kind: DirtyKind::Insert { at, text },
                eligible,
            })
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
    source: &str,
) -> Result<DirtyUnit, String> {
    let line_offsets = compute_line_offsets(source);
    let depth = segments.len().saturating_sub(1);
    // D4: eligibility is computed on the PRE-mutation tree.
    let eligible = {
        let path = path_nodes(node, segments).map_err(nav_err)?;
        eligible_path(&path)
    };
    let seq = navigate_mut(node, segments).map_err(nav_err)?;
    match seq {
        CustomNode::Sequence { items, .. } => {
            if items.is_empty() {
                // No existing item to anchor the append to; the caller keeps
                // the placeholder and falls back to full serialization.
                items.push(value);
                return Ok(DirtyUnit {
                    kind: DirtyKind::Insert {
                        at: 0,
                        text: String::new(),
                    },
                    eligible: false,
                });
            }
            let last_item = items[items.len() - 1]
                .source_range()
                .cloned()
                .unwrap_or(0..0);
            let at = line_end(&line_offsets, last_item.end, source.len());
            let indent = line_indent(&line_offsets, source, last_item.start);
            let text = crate::serializer::item_to_string(&value, indent, true, depth)?;
            items.push(value);
            Ok(DirtyUnit {
                kind: DirtyKind::Insert { at, text },
                eligible,
            })
        }
        _ if matches!(seq, CustomNode::Alias { .. }) => Err("cannot-edit-alias".to_string()),
        _ => Err("not-a-sequence".to_string()),
    }
}

/// Delete the node reached by `segments`. The final segment is removed from
/// its parent. Deleting the root (empty segments) is an error.
pub fn delete_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    source: &str,
) -> Result<DirtyUnit, String> {
    let line_offsets = compute_line_offsets(source);
    if segments.is_empty() {
        return Err("edit-error".to_string());
    }
    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let last = &last[0];
    // D4: eligibility is computed on the PRE-mutation tree.
    let eligible = {
        let path = path_nodes(node, segments).map_err(nav_err)?;
        eligible_path(&path)
    };
    let parent = navigate_mut(node, parent_segments).map_err(nav_err)?;
    let parent_is_alias = matches!(parent, CustomNode::Alias { .. });

    match (parent, last) {
        (CustomNode::Mapping { pairs, .. }, Segment::Key(k)) => {
            let key_node = CustomNode::plain_scalar(k.as_ref());
            let idx =
                mapping_key_index(pairs, &key_node).ok_or_else(|| "missing-path".to_string())?;
            // Capture the pair span BEFORE mutation; P3 then folds in any
            // standalone comment lines directly above it.
            let raw = pairs.get_index(idx).map(|(k, v)| {
                let s = k.source_range().map(|r| r.start).unwrap_or(0);
                let e = v.source_range().map(|r| r.end).unwrap_or(s);
                s..e
            });
            // shift_remove preserves mapping order (swap_remove would reorder).
            pairs.shift_remove_index(idx);
            let range = line_aligned(raw.unwrap_or(0..0), &line_offsets, source);
            let range = extend_delete_over_comments(range, &line_offsets, source);
            Ok(DirtyUnit {
                kind: DirtyKind::Delete { range },
                eligible,
            })
        }
        (CustomNode::Sequence { items, .. }, Segment::Index(i)) => {
            let idx = normalize_index(*i, items.len())
                .ok_or_else(|| "index-out-of-range-edit".to_string())?;
            let item_range = items[idx].source_range().cloned();
            items.remove(idx);
            let range = line_aligned(item_range.unwrap_or(0..0), &line_offsets, source);
            let range = extend_delete_over_comments(range, &line_offsets, source);
            Ok(DirtyUnit {
                kind: DirtyKind::Delete { range },
                eligible,
            })
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
    source: &str,
) -> Result<DirtyUnit, String> {
    let line_offsets = compute_line_offsets(source);
    if segments.is_empty() {
        return Err("cannot-rename-root".to_string());
    }
    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let last = &last[0];
    let (eligible, compact_override) =
        precompute(node, segments, parent_segments, &line_offsets, source);
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
            let old_range = pairs.get_index(idx).map(|(k, v)| {
                let s = k.source_range().map(|r| r.start).unwrap_or(0);
                let e = v.source_range().map(|r| r.end).unwrap_or(s);
                (s, e)
            });
            let old_indent =
                line_indent(&line_offsets, source, old_range.map(|r| r.0).unwrap_or(0));
            let value_node = pairs
                .shift_remove_index(idx)
                .map(|(_, v)| v)
                .ok_or_else(|| "missing-path".to_string())?;
            pairs.shift_insert(idx, new_key_node, value_node);
            let (key_start, value_end) = old_range.unwrap_or((0, 0));
            Ok(DirtyUnit {
                kind: region_unit(
                    key_start,
                    value_end,
                    old_indent,
                    &line_offsets,
                    source,
                    compact_override,
                ),
                eligible,
            })
        }
        _ if parent_is_alias => Err("cannot-edit-alias".to_string()),
        _ => Err("cannot-rename-complex-key".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse, parse_with_options, yaml::YamlSchema};

    fn doc(yaml: &str) -> CustomNode {
        parse(yaml, YamlSchema::Core).unwrap()
    }

    fn doc_with_ranges(yaml: &str) -> CustomNode {
        parse_with_options(yaml, true, YamlSchema::Core, 1000, false)
            .unwrap()
            .0
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
            "- a\n- b\n- c",
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
            "- a\n- b",
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
            "",
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
            "base: &b\n  x: 1\ncopy: *b",
        )
        .unwrap_err();
        assert_eq!(err, "cannot-edit-alias");
    }

    #[test]
    fn delete_path_negative_index() {
        let mut node = doc("- a\n- b\n- c");
        delete_path(&mut node, &[Segment::Index(-2)], "- a\n- b\n- c").unwrap();
        assert_eq!(crate::serializer::to_yaml(&node), "- a\n- c\n");
    }

    #[test]
    fn insert_path_negative_index() {
        let mut node = doc("- a\n- b\n- c");
        insert_path(
            &mut node,
            &[],
            -1,
            CustomNode::plain_scalar("z"),
            "- a\n- b\n- c",
        )
        .unwrap();
        assert_eq!(crate::serializer::to_yaml(&node), "- a\n- b\n- z\n- c\n");
    }

    #[test]
    fn set_scalar_pair_region_is_pair_lines() {
        let mut node = doc_with_ranges("a: 1\nb: 2\n");
        let unit = set_path(
            &mut node,
            &[Segment::Key(Cow::Borrowed("b"))],
            CustomNode::plain_scalar("9"),
            true,
            "a: 1\nb: 2\n",
        )
        .unwrap();
        match unit.kind {
            DirtyKind::Region { range, indent } => {
                assert_eq!(range, 5..10);
                assert_eq!(indent, 0);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn delete_item_region_is_item_lines() {
        let mut node = doc_with_ranges("- a\n- b\n- c\n");
        let unit = delete_path(&mut node, &[Segment::Index(1)], "- a\n- b\n- c\n").unwrap();
        match unit.kind {
            DirtyKind::Delete { range } => assert_eq!(range, 4..8),
            _ => panic!(),
        }
    }

    #[test]
    fn insert_computes_offset() {
        let mut node = doc_with_ranges("- a\n- b\n");
        let unit = insert_path(
            &mut node,
            &[],
            1,
            CustomNode::plain_scalar("z"),
            "- a\n- b\n",
        )
        .unwrap();
        match unit.kind {
            DirtyKind::Insert { at, .. } => assert_eq!(at, 4),
            _ => panic!(),
        }
    }

    #[test]
    fn compact_item_edit_regions_whole_item() {
        let mut node = doc_with_ranges("- host: a\n");
        let unit = set_path(
            &mut node,
            &[Segment::Index(0), Segment::Key(Cow::Borrowed("host"))],
            CustomNode::plain_scalar("b"),
            true,
            "- host: a\n",
        )
        .unwrap();
        match unit.kind {
            DirtyKind::Region { range, .. } => assert_eq!(range, 0..10),
            _ => panic!(),
        }
    }

    #[test]
    fn delete_includes_preceding_comment_lines() {
        let mut node = doc_with_ranges("# note\nb: 2\n");
        let unit = delete_path(
            &mut node,
            &[Segment::Key(Cow::Borrowed("b"))],
            "# note\nb: 2\n",
        )
        .unwrap();
        match unit.kind {
            DirtyKind::Delete { range } => assert_eq!(range, 0..12),
            _ => panic!(),
        }
    }

    #[test]
    fn multi_key_compact_item_regions_whole_item() {
        let mut node = doc_with_ranges("- host: a\n  port: 8080\n");
        let unit = set_path(
            &mut node,
            &[Segment::Index(0), Segment::Key(Cow::Borrowed("host"))],
            CustomNode::plain_scalar("b"),
            true,
            "- host: a\n  port: 8080\n",
        )
        .unwrap();
        match unit.kind {
            DirtyKind::Region { range, .. } => assert_eq!(range, 0..23),
            _ => panic!(),
        }
    }

    #[test]
    fn set_creating_pair_is_insert_at_container_end() {
        let mut node = doc_with_ranges("a: 1\n");
        let unit = set_path(
            &mut node,
            &[Segment::Key(Cow::Borrowed("b"))],
            CustomNode::plain_scalar("2"),
            true,
            "a: 1\n",
        )
        .unwrap();
        match unit.kind {
            DirtyKind::Insert { at, text } => {
                assert_eq!(at, 5);
                assert_eq!(text, "b: 2\n");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn append_uses_last_item_end() {
        let mut node = doc_with_ranges("- a\n- b\n");
        let unit =
            append_path(&mut node, &[], CustomNode::plain_scalar("c"), "- a\n- b\n").unwrap();
        match unit.kind {
            DirtyKind::Insert { at, text } => {
                assert_eq!(at, 8);
                assert_eq!(text, "- c\n");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn flow_style_ancestor_is_ineligible() {
        let mut node = doc_with_ranges("a: {b: 1}\n");
        let unit = set_path(
            &mut node,
            &[
                Segment::Key(Cow::Borrowed("a")),
                Segment::Key(Cow::Borrowed("b")),
            ],
            CustomNode::plain_scalar("2"),
            true,
            "a: {b: 1}\n",
        )
        .unwrap();
        assert!(!unit.eligible);
    }

    #[test]
    fn missing_range_is_ineligible_placeholder() {
        let mut node = CustomNode::plain_mapping(
            [(CustomNode::plain_scalar("a"), CustomNode::plain_scalar("1"))]
                .into_iter()
                .collect::<IndexMap<_, _>>(),
        );
        let unit = set_path(
            &mut node,
            &[Segment::Key(Cow::Borrowed("a"))],
            CustomNode::plain_scalar("2"),
            true,
            "a: 1\n",
        )
        .unwrap();
        assert!(!unit.eligible);
    }

    #[test]
    fn rename_in_compact_item_regions_whole_item() {
        let mut node = doc_with_ranges("- host: a\n");
        let unit = rename_path(
            &mut node,
            &[Segment::Index(0), Segment::Key(Cow::Borrowed("host"))],
            "name",
            "- host: a\n",
        )
        .unwrap();
        match unit.kind {
            DirtyKind::Region { range, .. } => assert_eq!(range, 0..10),
            _ => panic!(),
        }
    }

    #[test]
    fn delete_compact_inner_key_removes_key_line() {
        let mut node = doc_with_ranges("- host: a\n  port: 8080\n");
        let unit = delete_path(
            &mut node,
            &[Segment::Index(0), Segment::Key(Cow::Borrowed("port"))],
            "- host: a\n  port: 8080\n",
        )
        .unwrap();
        match unit.kind {
            DirtyKind::Delete { range } => assert_eq!(range, 10..23),
            _ => panic!(),
        }
    }

    #[test]
    fn compact_guard_shared_with_serializer() {
        let c = doc_with_ranges("- host: a\n");
        let CustomNode::Sequence { items, .. } = c else {
            panic!()
        };
        assert!(crate::serializer::is_compact_item(&items[0]));
        // A value-level anchor does not break the compact form: the compact
        // arm serializes values through `serialize_node_internal`, so `&x`
        // survives. Guard and serializer agree.
        let c2 = doc_with_ranges("- name: &x anchor\n  value: 1\n");
        let CustomNode::Sequence { items: i2, .. } = c2 else {
            panic!()
        };
        assert!(crate::serializer::is_compact_item(&i2[0]));
        let c3 = doc_with_ranges("- top:\n    inner: 1\n");
        let CustomNode::Sequence { items: i3, .. } = c3 else {
            panic!()
        };
        assert!(!crate::serializer::is_compact_item(&i3[0]));
        // Mapping-level metadata (anchor) disqualifies the compact form.
        let anchored = CustomNode::Mapping {
            pairs: IndexMap::from([(CustomNode::plain_scalar("k"), CustomNode::plain_scalar("v"))]),
            comment: None,
            anchor: Some("x".to_string()),
            tag: None,
            flow_style: false,
            source_range: None,
        };
        assert!(!crate::serializer::is_compact_item(&anchored));
        // Flow-style mappings are never compact.
        let flow = CustomNode::Mapping {
            pairs: IndexMap::from([(CustomNode::plain_scalar("k"), CustomNode::plain_scalar("v"))]),
            comment: None,
            anchor: None,
            tag: None,
            flow_style: true,
            source_range: None,
        };
        assert!(!crate::serializer::is_compact_item(&flow));
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
