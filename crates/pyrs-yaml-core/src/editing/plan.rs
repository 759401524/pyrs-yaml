//! Splice-plan orchestration: AST mutation paired with `DirtyUnit`
//! generation for the layout-pinned edit path.
//!
//! Moved down from the Python bindings so every consumer (bindings and
//! the `pyq` CLI) shares one edit engine instead of mirroring its
//! eligibility/region logic.

pub use crate::editing::metadata::with_metadata_from;
pub use crate::editing::{
    DirtyKind, DirtyUnit, NavigateError, Segment, eligible_path, extend_delete_over_comments,
    key_eq, line_aligned, line_end, line_indent, line_start, mapping_key_index, navigate_mut,
    normalize_index, path_nodes, precompute, regenerate_region_text, region_unit,
};

use crate::ast::{CustomNode, NodeMap};
use crate::parser::yaml::compute_line_offsets;
use alloc::borrow::Cow;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ops::Range;

/// Use the caller's precomputed line-offset table, or compute one from
/// `source` when not supplied. Borrowing avoids re-scanning the source on
/// every edit inside a splice burst.
fn resolve_line_offsets<'a>(
    line_offsets: Option<&'a [usize]>,
    source: &'a str,
) -> Cow<'a, [usize]> {
    match line_offsets {
        Some(offs) => Cow::Borrowed(offs),
        None => Cow::Owned(compute_line_offsets(source)),
    }
}

pub fn set_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    new_value: CustomNode,
    preserve_metadata: bool,
    source: &str,
    line_offsets: Option<&[usize]>,
    create_missing: bool,
) -> Result<DirtyUnit, String> {
    if segments.is_empty() {
        return set_path_root(node, new_value, source);
    }
    // One invariant, checked where the drop happens rather than per verb: a replacement that does not
    // carry the replaced node's metadata also carries its anchor away, and an anchor is a name the rest
    // of the document points at. Measured on `base: &x 1\nuse: *x`: setting `$.base` (or moving `$.use`
    // onto it, which is one `set_path` call with `preserve_metadata: false`) emitted `base: *x` - a
    // document whose only remaining reference names an anchor that no longer exists, refused by this
    // library's own reader. `move_path` and `set_many_path` reach it through here, so the guard is
    // written once.
    if !preserve_metadata
        && let Some(previous) = anchor_at(node, segments)
        && any_alias_names(node, previous)
    {
        return Err("anchor-is-referenced".to_string());
    }
    // The wider half of the same invariant, asked at the door `set` walks through. `preserve_metadata`
    // carries the replaced node's own anchor onto the new value; nothing can carry a definition from
    // *inside* the subtree that value replaces, because the new value never had it. Measured on
    // `: abase: &x 1\nuse: *x` (libFuzzer `edit_yaml` crash-fbd364b3): setting the path whose node is
    // the mapping holding `abase: &x 1` - a node carrying no anchor of its own - emitted
    // `~: v0\nuse: *x`, and this library's own reader refused it with `found unknown anchor`. The
    // over-refusal `delete_path` accepts is accepted here for the same reason: a subtree that holds
    // both an anchor and the alias naming it is safe to write, and this refuses it too, because
    // deciding that needs the document order of a tree that is about to change.
    if let Ok(target) = crate::editing::navigate(node, segments) {
        let mut defined = Vec::new();
        anchors_in(target, &mut defined);
        let own = target.anchor();
        if defined
            .iter()
            .any(|name| Some(name.as_str()) != own && any_alias_names(node, name))
        {
            return Err("anchor-is-referenced".to_string());
        }
    }
    let offsets = resolve_line_offsets(line_offsets, source);
    let line_offsets: &[usize] = &offsets;

    if matches!(node, CustomNode::Null { .. }) {
        *node = CustomNode::Mapping {
            pairs: NodeMap::default(),
            flow_style: false,
            meta: Default::default(),
        };
    }
    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let last = &last[0];
    let depth = segments.len().saturating_sub(1);

    let (eligible, compact_override) =
        precompute(node, segments, parent_segments, line_offsets, source);

    let parent = match navigate_mut(node, parent_segments) {
        Ok(parent) => parent,
        Err(NavigateError::Missing(_)) if create_missing => {
            return set_path_create_missing(node, segments, new_value, source, line_offsets);
        }
        Err(e) => return Err(e.to_string()),
    };
    let parent_is_alias = matches!(parent, CustomNode::Alias { .. });

    if parent_is_alias {
        return Err("cannot-edit-alias".to_string());
    }
    let ctx = SetPathContext {
        segments,
        parent_segments,
        compact_override: &compact_override,
        eligible,
        depth,
        line_offsets,
        source,
    };
    match last {
        Segment::Key(_) => set_path_mapping_key(node, last, new_value, preserve_metadata, &ctx),
        Segment::Index(_) => {
            set_path_sequence_index(node, last, new_value, preserve_metadata, &ctx)
        }
    }
}

/// Replace the whole document (empty segment path): swap in the new value and
/// mark the entire source range dirty.
fn set_path_root(
    node: &mut CustomNode,
    new_value: CustomNode,
    source: &str,
) -> Result<DirtyUnit, String> {
    let eligible = eligible_path(&path_nodes(node, &[]).map_err(|e| e.to_string())?);
    *node = new_value;
    let text = crate::serializer::to_yaml_with_options(
        &*node,
        &crate::serializer::SerializeOptions::default(),
    )
    .map_err(|e| e.to_string())?;
    Ok(DirtyUnit {
        kind: DirtyKind::Region {
            range: 0..source.len(),
            indent: 0,
            text,
        },
        eligible,
    })
}

/// Set the value at `last` (a `Key` segment) of the mapping reached via
/// `parent_segments`. Extracted from `set_path` to keep that function small.
struct SetPathContext<'a> {
    segments: &'a [Segment<'a>],
    parent_segments: &'a [Segment<'a>],
    compact_override: &'a Option<(Range<usize>, usize, usize)>,
    eligible: bool,
    depth: usize,
    line_offsets: &'a [usize],
    source: &'a str,
}

/// If the regenerated region text carries a leading standalone note
/// (`pair_to_string`/`item_to_string` emit the node's own comment first),
/// widen the byte range over the note lines already in the old text -
/// otherwise the splice keeps the old note AND writes the new one: a
/// duplicated comment line.
fn widen_over_note(
    range: Range<usize>,
    text: &str,
    line_offsets: &[usize],
    source: &str,
) -> Range<usize> {
    if text.trim_start().starts_with('#') {
        extend_delete_over_comments(range, line_offsets, source)
    } else {
        range
    }
}

fn set_path_mapping_key(
    node: &mut CustomNode,
    last: &Segment<'_>,
    new_value: CustomNode,
    preserve_metadata: bool,
    ctx: &SetPathContext<'_>,
) -> Result<DirtyUnit, String> {
    let k = match last {
        Segment::Key(k) => k,
        _ => return Err("create-needs-mapping".to_string()),
    };
    let parent = match navigate_mut(node, ctx.parent_segments) {
        Ok(p) => p,
        Err(e) => return Err(e.to_string()),
    };
    let CustomNode::Mapping { pairs, .. } = parent else {
        return Err("create-needs-mapping".to_string());
    };
    let key_node = CustomNode::plain_scalar(k.as_ref());
    match mapping_key_index(pairs, &key_node) {
        Some(idx) => {
            let stored_key = pairs.get_index(idx).map(|(k, _)| k.clone());
            let stored_key = match stored_key {
                Some(k) => k,
                None => key_node.clone(),
            };
            let (old_key_start, old_value_end, old_indent) = pairs
                .get_index(idx)
                .map(|(k, v)| {
                    let s = k.source_range().map(|r| r.start).unwrap_or(0);
                    let e = v.source_range().map(|r| r.end).unwrap_or(s);
                    (s, e, line_indent(ctx.line_offsets, ctx.source, s))
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
            let text = regenerate_region_text(
                node,
                ctx.segments,
                ctx.parent_segments,
                None,
                ctx.compact_override,
                old_indent,
                ctx.depth,
            )
            .map_err(|e| e.to_string())?;
            let region = widen_over_note(
                old_key_start..old_value_end,
                &text,
                ctx.line_offsets,
                ctx.source,
            );
            Ok(DirtyUnit {
                kind: region_unit(
                    region.start,
                    region.end,
                    old_indent,
                    ctx.line_offsets,
                    ctx.source,
                    ctx.compact_override.clone(),
                    &text,
                ),
                eligible: ctx.eligible,
            })
        }
        None => {
            let (at, indent) = match pairs.iter().last() {
                Some((lk, lv)) => {
                    let key_start = lk.source_range().map(|r| r.start).unwrap_or(0);
                    let val_end = lv.source_range().map(|r| r.end).unwrap_or(key_start);
                    (
                        line_end(ctx.line_offsets, val_end, ctx.source.len()),
                        line_indent(ctx.line_offsets, ctx.source, key_start),
                    )
                }
                None => (0, 0),
            };
            // The probe above is only good for *finding* a spelling; a key the document does not
            // hold yet is emitted, so its style comes from the conversion guard.
            let new_key = named_key_node(k.as_ref());
            let text = crate::serializer::pair_to_string(&new_key, &new_value, indent, ctx.depth)
                .map_err(|e| e.to_string())?;
            pairs.insert(new_key, new_value);
            Ok(DirtyUnit {
                kind: DirtyKind::Insert { at, text },
                eligible: ctx.eligible,
            })
        }
    }
}

/// Set the value at `last` (an `Index` segment) of the sequence reached via
/// `parent_segments`. Extracted from `set_path` to keep that function small.
fn set_path_sequence_index(
    node: &mut CustomNode,
    last: &Segment<'_>,
    new_value: CustomNode,
    preserve_metadata: bool,
    ctx: &SetPathContext<'_>,
) -> Result<DirtyUnit, String> {
    let i = match last {
        Segment::Index(i) => *i,
        _ => return Err("create-needs-mapping".to_string()),
    };
    let parent = match navigate_mut(node, ctx.parent_segments) {
        Ok(p) => p,
        Err(e) => return Err(e.to_string()),
    };
    let CustomNode::Sequence { items, .. } = parent else {
        return Err("create-needs-mapping".to_string());
    };
    let idx =
        normalize_index(i, items.len()).ok_or_else(|| format!("index-out-of-range-edit:{i}"))?;
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
    let range = line_aligned(raw, ctx.line_offsets, ctx.source);
    let indent = line_indent(ctx.line_offsets, ctx.source, raw_start);
    let text = crate::serializer::item_to_string(&items[idx], indent, ctx.depth)
        .map_err(|e| e.to_string())?;
    let range = widen_over_note(range, &text, ctx.line_offsets, ctx.source);
    Ok(DirtyUnit {
        kind: DirtyKind::Region {
            range,
            indent,
            text,
        },
        eligible: ctx.eligible,
    })
}

/// A key the caller named in a path, styled so it keeps the type that name resolved to.
///
/// `rename` and the segments `set(create_missing=True)` synthesize built a plain node from the
/// `&str`, so the stored key was whatever a reader of the emitted document made of the spelling.
/// A path segment is not a data string: issue #336 ruled that it addresses the key a reader
/// *resolves*, so `$.~` names the null key and `$.42` the integer one, and those must stay plain —
/// quoting them moves the entry to a different key. A YAML 1.1 spelling is the opposite case:
/// `yes` resolves to the string here, to `true` in PyYAML and in this engine's own
/// `schema="1.1"` profile, so plain emission loses the name the caller gave. Only that second half
/// is asked of the guard, which is why this calls
/// [`resolved_by_a_yaml_1_1_reader`](pyrs_schema::schema::resolved_by_a_yaml_1_1_reader) rather
/// than [`needs_quotes`](pyrs_schema::schema::needs_quotes) as the conversion layer does. Node
/// identity ignores style, so the duplicate-key check in `rename_path` is unaffected.
fn named_key_node(name: &str) -> CustomNode {
    if pyrs_schema::schema::resolved_by_a_yaml_1_1_reader(name) {
        CustomNode::double_quoted_scalar(name.to_string())
    } else {
        CustomNode::plain_scalar(name.to_string())
    }
}

/// `set_path` with `create_missing`: walk as deep as the path allows and
/// synthesize the missing intermediate mappings as a single nested Insert at
/// the end of the deepest existing mapping. Only key segments may be created;
/// an out-of-range index, an alias, or a non-container intermediate still
/// errors.
fn set_path_create_missing(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    new_value: CustomNode,
    source: &str,
    line_offsets: &[usize],
) -> Result<DirtyUnit, String> {
    let mut consumed = 0;
    {
        let mut cur: &CustomNode = node;
        for (i, seg) in segments.iter().enumerate() {
            match (cur, seg) {
                (CustomNode::Mapping { pairs, .. }, Segment::Key(k)) => {
                    let key_node = CustomNode::plain_scalar(k.as_ref());
                    match mapping_key_index(pairs, &key_node) {
                        Some(idx) => {
                            cur = pairs
                                .get_index(idx)
                                .map(|(_, v)| v)
                                .ok_or_else(|| "create-needs-mapping".to_string())?;
                            consumed = i + 1;
                        }
                        None => break,
                    }
                }
                (CustomNode::Sequence { items, .. }, Segment::Index(idx)) => {
                    let idx = normalize_index(*idx, items.len())
                        .ok_or_else(|| format!("index-out-of-range-edit:{idx}"))?;
                    cur = items
                        .get(idx)
                        .ok_or_else(|| format!("index-out-of-range-edit:{idx}"))?;
                    consumed = i + 1;
                }
                (CustomNode::Alias { .. }, _) => return Err("cannot-edit-alias".to_string()),
                _ => return Err("create-needs-mapping".to_string()),
            }
        }
    }
    let tail = &segments[consumed..];
    if tail.is_empty() {
        return Err("create-needs-mapping".to_string());
    }
    let mut nested = new_value;
    for seg in tail[1..].iter().rev() {
        let Segment::Key(k) = seg else {
            return Err("create-needs-mapping".to_string());
        };
        let mut pairs = NodeMap::default();
        pairs.insert(named_key_node(k.as_ref()), nested);
        nested = CustomNode::plain_mapping(pairs);
    }
    let top_key = match &tail[0] {
        Segment::Key(k) => named_key_node(k.as_ref()),
        Segment::Index(_) => return Err("create-needs-mapping".to_string()),
    };
    let path = path_nodes(node, &segments[..consumed]).map_err(|e| e.to_string())?;
    let eligible = eligible_path(&path);
    let parent = navigate_mut(node, &segments[..consumed]).map_err(|e| e.to_string())?;
    let CustomNode::Mapping { pairs, .. } = parent else {
        return Err("create-needs-mapping".to_string());
    };
    let (at, indent) = match pairs.iter().last() {
        Some((lk, lv)) => {
            let key_start = lk.source_range().map(|r| r.start).unwrap_or(0);
            let val_end = lv.source_range().map(|r| r.end).unwrap_or(key_start);
            (
                line_end(line_offsets, val_end, source.len()),
                line_indent(line_offsets, source, key_start),
            )
        }
        None => (0, 0),
    };
    let depth = consumed;
    let text = crate::serializer::pair_to_string(&top_key, &nested, indent, depth)
        .map_err(|e| e.to_string())?;
    pairs.insert(top_key, nested);
    Ok(DirtyUnit {
        kind: DirtyKind::Insert { at, text },
        eligible,
    })
}

/// Whether the node at `segments` is eligible for compact inline editing.
/// Shared by the path-edit helpers to avoid repeating the `path_nodes` +
/// `eligible_path` computation.
fn path_eligible(node: &CustomNode, segments: &[Segment<'_>]) -> Result<bool, String> {
    let path = path_nodes(node, segments).map_err(|e| e.to_string())?;
    Ok(eligible_path(&path))
}

pub fn insert_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    index: i64,
    value: CustomNode,
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    let offsets = resolve_line_offsets(line_offsets, source);
    let line_offsets: &[usize] = &offsets;
    let depth = segments.len().saturating_sub(1);
    let eligible = path_eligible(node, segments)?;
    let seq = navigate_mut(node, segments).map_err(|e| e.to_string())?;
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
            let (at, indent) = if insert_at < items.len() {
                let next_item = items[insert_at].source_range().cloned().unwrap_or(0..0);
                (
                    line_start(line_offsets, next_item.start),
                    line_indent(line_offsets, source, next_item.start),
                )
            } else {
                let last_item = items[items.len() - 1]
                    .source_range()
                    .cloned()
                    .unwrap_or(0..0);
                (
                    line_end(line_offsets, last_item.end, source.len()),
                    line_indent(line_offsets, source, last_item.start),
                )
            };
            let text = crate::serializer::item_to_string(&value, indent, depth)
                .map_err(|e| e.to_string())?;
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

pub fn append_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    value: CustomNode,
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    let offsets = resolve_line_offsets(line_offsets, source);
    let line_offsets: &[usize] = &offsets;
    let depth = segments.len().saturating_sub(1);
    let eligible = path_eligible(node, segments)?;
    let seq = navigate_mut(node, segments).map_err(|e| e.to_string())?;
    match seq {
        CustomNode::Sequence { items, .. } => {
            if items.is_empty() {
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
            let at = line_end(line_offsets, last_item.end, source.len());
            let indent = line_indent(line_offsets, source, last_item.start);
            let text = crate::serializer::item_to_string(&value, indent, depth)
                .map_err(|e| e.to_string())?;
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

pub fn delete_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    let offsets = resolve_line_offsets(line_offsets, source);
    let line_offsets: &[usize] = &offsets;
    if segments.is_empty() {
        return Err("edit-error".to_string());
    }
    // The same invariant the anchor verbs ask, from the other side of the door: an anchor defined
    // inside the subtree about to be removed may be the only thing some alias points at, and the
    // emitted document would then name an anchor nothing defines. `move_path` reaches it through here
    // as well, because its second half is a delete - measured on `base: &x 1\nuse: *x`, moving
    // `$.base` elsewhere emitted `use: *x` alone, text this library's own reader refuses.
    //
    // The answer is deliberately wider than the set of aliases that really dangle. A subtree holding
    // both an anchor and the alias that names it is safe to remove, and this refuses that too:
    // deciding it needs the document order of a tree that is about to change, and over-refusing costs
    // the caller one more edit while under-refusing writes a file nothing can read.
    if let Ok(target) = crate::editing::navigate(node, segments) {
        let mut defined = Vec::new();
        anchors_in(target, &mut defined);
        if defined.iter().any(|name| any_alias_names(node, name)) {
            return Err("anchor-is-referenced".to_string());
        }
    }
    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let last = &last[0];
    let (eligible, compact_override) =
        precompute(node, segments, parent_segments, line_offsets, source);
    let parent = navigate_mut(node, parent_segments).map_err(|e| e.to_string())?;
    let parent_is_alias = matches!(parent, CustomNode::Alias { .. });

    match (parent, last) {
        (CustomNode::Mapping { pairs, .. }, Segment::Key(k)) => {
            let key_node = CustomNode::plain_scalar(k.as_ref());
            let idx =
                mapping_key_index(pairs, &key_node).ok_or_else(|| "missing-path".to_string())?;
            let compact_first_delete = compact_override
                .as_ref()
                .map(|(_, _, prefix_len)| *prefix_len == segments.len() - 1 && idx == 0)
                .unwrap_or(false);
            let eligible = eligible && !compact_first_delete;
            let raw = pairs.get_index(idx).map(|(k, v)| {
                let s = k.source_range().map(|r| r.start).unwrap_or(0);
                let e = v.source_range().map(|r| r.end).unwrap_or(s);
                s..e
            });
            pairs.shift_remove_index(idx);
            let range = line_aligned(raw.unwrap_or(0..0), line_offsets, source);
            let range = extend_delete_over_comments(range, line_offsets, source);
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
            let range = line_aligned(item_range.unwrap_or(0..0), line_offsets, source);
            let range = extend_delete_over_comments(range, line_offsets, source);
            Ok(DirtyUnit {
                kind: DirtyKind::Delete { range },
                eligible,
            })
        }
        _ if parent_is_alias => Err("cannot-edit-alias".to_string()),
        _ => Err("missing-path".to_string()),
    }
}

pub fn rename_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    new_key: &str,
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    let offsets = resolve_line_offsets(line_offsets, source);
    let line_offsets: &[usize] = &offsets;
    if segments.is_empty() {
        return Err("cannot-rename-root".to_string());
    }
    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let last = &last[0];
    let depth = segments.len().saturating_sub(1);
    let (eligible, compact_override) =
        precompute(node, segments, parent_segments, line_offsets, source);
    let parent = navigate_mut(node, parent_segments).map_err(|e| e.to_string())?;
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
            let new_key_node = named_key_node(new_key);
            if !key_eq(&new_key_node, &old_key_node)
                && mapping_key_index(pairs, &new_key_node).is_some()
            {
                return Err("rename-key-exists".to_string());
            }
            let old_range = pairs.get_index(idx).map(|(k, v)| {
                let s = k.source_range().map(|r| r.start).unwrap_or(0);
                let e = v.source_range().map(|r| r.end).unwrap_or(s);
                (s, e)
            });
            let old_indent = line_indent(line_offsets, source, old_range.map(|r| r.0).unwrap_or(0));
            let value_node = pairs
                .shift_remove_index(idx)
                .map(|(_, v)| v)
                .ok_or_else(|| "missing-path".to_string())?;
            pairs.shift_insert(idx, new_key_node, value_node);
            let (key_start, value_end) = old_range.unwrap_or((0, 0));
            let text = regenerate_region_text(
                node,
                segments,
                parent_segments,
                Some(new_key),
                &compact_override,
                old_indent,
                depth,
            )
            .map_err(|e| e.to_string())?;
            Ok(DirtyUnit {
                kind: region_unit(
                    key_start,
                    value_end,
                    old_indent,
                    line_offsets,
                    source,
                    compact_override,
                    &text,
                ),
                eligible,
            })
        }
        _ if parent_is_alias => Err("cannot-edit-alias".to_string()),
        _ => Err("cannot-rename-complex-key".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Metadata (comment / anchor / tag) path-based setters
// ---------------------------------------------------------------------------

/// Apply a metadata mutation (comment/anchor/tag set or remove) to the node
/// at `segments`, then regenerate the affected source region.
///
/// Reuses the same splice eligibility + region regeneration machinery as
/// `rename_path`: navigate to the parent, apply the mutation to the target
/// node, re-serialize the pair/item (or whole document for root), emit a
/// `DirtyKind::Region`.
fn apply_metadata_path<F>(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    source: &str,
    line_offsets: Option<&[usize]>,
    mut mutate: F,
) -> Result<DirtyUnit, String>
where
    F: FnMut(&mut CustomNode),
{
    let offsets = resolve_line_offsets(line_offsets, source);
    let line_offsets: &[usize] = &offsets;

    // Root: mutate then full re-serialize.
    if segments.is_empty() {
        let eligible = eligible_path(&path_nodes(node, &[]).map_err(|e| e.to_string())?);
        mutate(node);
        let text = crate::serializer::to_yaml_with_options(
            &*node,
            &crate::serializer::SerializeOptions::default(),
        )
        .map_err(|e| e.to_string())?;
        return Ok(DirtyUnit {
            kind: DirtyKind::Region {
                range: 0..source.len(),
                indent: 0,
                text,
            },
            eligible,
        });
    }

    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let last = &last[0];
    let depth = segments.len().saturating_sub(1);
    let (eligible, compact_override) =
        precompute(node, segments, parent_segments, line_offsets, source);
    let parent = navigate_mut(node, parent_segments).map_err(|e| e.to_string())?;
    if matches!(parent, CustomNode::Alias { .. }) {
        return Err("cannot-edit-alias".to_string());
    }

    match (parent, last) {
        (CustomNode::Mapping { pairs, .. }, Segment::Key(k)) => {
            let key_node = CustomNode::plain_scalar(k.as_ref());
            let idx =
                mapping_key_index(pairs, &key_node).ok_or_else(|| "missing-path".to_string())?;
            // Borrow the value mutably to apply the mutation.
            let (_, value) = pairs
                .get_index_mut(idx)
                .ok_or_else(|| "missing-path".to_string())?;
            if matches!(value, CustomNode::Alias { .. }) {
                return Err("cannot-edit-alias".to_string());
            }
            mutate(value);
            // Regenerate the pair text.
            let (k_ref, v_ref) = pairs
                .get_index(idx)
                .ok_or_else(|| "missing-path".to_string())?;
            let old_range = {
                let s = k_ref.source_range().map(|r| r.start).unwrap_or(0);
                let e = v_ref.source_range().map(|r| r.end).unwrap_or(s);
                s..e
            };
            let old_indent = line_indent(line_offsets, source, old_range.start);
            let text = regenerate_region_text(
                node,
                segments,
                parent_segments,
                None,
                &compact_override,
                old_indent,
                depth,
            )
            .map_err(|e| e.to_string())?;
            Ok(DirtyUnit {
                kind: region_unit(
                    old_range.start,
                    old_range.end,
                    old_indent,
                    line_offsets,
                    source,
                    compact_override,
                    &text,
                ),
                eligible,
            })
        }
        (CustomNode::Sequence { items, .. }, Segment::Index(i)) => {
            let idx = normalize_index(*i, items.len())
                .ok_or_else(|| "index-out-of-range-edit".to_string())?;
            if matches!(items[idx], CustomNode::Alias { .. }) {
                return Err("cannot-edit-alias".to_string());
            }
            mutate(&mut items[idx]);
            let item_range = items[idx].source_range().cloned().unwrap_or(0..0);
            let old_indent = line_indent(line_offsets, source, item_range.start);
            let text = crate::serializer::item_to_string(&items[idx], old_indent, depth)
                .map_err(|e| e.to_string())?;
            Ok(DirtyUnit {
                kind: region_unit(
                    item_range.start,
                    item_range.end,
                    old_indent,
                    line_offsets,
                    source,
                    compact_override,
                    &text,
                ),
                eligible,
            })
        }
        _ => Err("missing-path".to_string()),
    }
}

/// Set the comment on the node at `segments`.
pub fn set_comment_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    comment: crate::ast::Comment,
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    apply_metadata_path(node, segments, source, line_offsets, |n| {
        n.set_comment(comment.clone());
    })
}

/// Set the leading (standalone) comment on the node at `segments`.
///
/// PR #117 exposes the `decor.leading_comment` slot to the Python
/// layer. The write is atomic: `set_leading_comment` clears the legacy
/// `comment(standalone = true)` slot the YAML receiver still uses, so
/// the two conventions never conflict.
pub fn set_leading_comment_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    comment: crate::ast::Comment,
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    apply_metadata_path(node, segments, source, line_offsets, |n| {
        n.set_leading_comment(comment.clone());
    })
}

/// Remove the leading (standalone) comment on the node at `segments`.
pub fn remove_leading_comment_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    apply_metadata_path(node, segments, source, line_offsets, |n| {
        n.remove_leading_comment();
    })
}

/// Remove the comment on the node at `segments`.
pub fn remove_comment_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    apply_metadata_path(node, segments, source, line_offsets, |n| {
        n.remove_comment();
    })
}

/// Set the YAML tag on the node at `segments`.
pub fn set_tag_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    tag: crate::ast::Tag,
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    apply_metadata_path(node, segments, source, line_offsets, |n| {
        n.set_tag(tag.clone());
    })
}

/// Remove the YAML tag on the node at `segments`.
pub fn remove_tag_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    apply_metadata_path(node, segments, source, line_offsets, |n| {
        n.remove_tag();
    })
}

/// Whether any alias in this document names `anchor`.
///
/// An alias is a reference the node being edited cannot see, so a verb that changes or deletes an
/// anchor name has to ask the whole tree. Key positions are walked too: an alias can be a key
/// (`*defaults: 1`), and renaming the anchor it points at breaks that line as surely as a value.
fn any_alias_names(node: &CustomNode, anchor: &str) -> bool {
    match node {
        CustomNode::Alias { name } => name == anchor,
        CustomNode::Mapping { pairs, .. } => pairs
            .iter()
            .any(|(key, value)| any_alias_names(key, anchor) || any_alias_names(value, anchor)),
        CustomNode::Sequence { items, .. } => {
            items.iter().any(|item| any_alias_names(item, anchor))
        }
        _ => false,
    }
}

/// The anchor name of the node at `segments`, if it carries one.
fn anchor_at<'a>(node: &'a CustomNode, segments: &[Segment<'_>]) -> Option<&'a str> {
    crate::editing::navigate(node, segments).ok()?.anchor()
}

/// The anchor names a subtree defines, at any depth and in key positions too.
fn anchors_in(node: &CustomNode, into: &mut Vec<String>) {
    if let Some(name) = node.anchor() {
        into.push(name.to_string());
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs.iter() {
                anchors_in(key, into);
                anchors_in(value, into);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                anchors_in(item, into);
            }
        }
        _ => {}
    }
}

/// Set the anchor name on the node at `segments`.
///
/// Renaming a name the document already refers to is refused, for the reason [`with_metadata_from`]
/// states about replacing a value: an anchor is a name the rest of the document points at, and the
/// alias is not rewritten by this call. Measured before the guard: on `base: &x 1\nuse: *x`, setting
/// the anchor to `a0` emitted `base: &a0 1\nuse: *x`, text this library's own reader refuses with
/// `found unknown anchor` - a `set` that turned a readable document into an unreadable one, which the
/// `edit_yaml` target now catches per document (`fuzz/seeds/edit_yaml/former-crash-3cc4245d.seed`).
///
/// The guard is deliberately wider than the set of aliases that would really dangle. An alias binds
/// the *nearest preceding* anchor of its name, so when a second `&x` follows this node, a rename here
/// keeps that alias readable and this call refuses it anyway. Over-refusing costs the caller one more
/// edit; under-refusing writes a document nothing can read. Making the rename carry its aliases -
/// which is what an author typing a new name may want - is a feature decision about the verb's meaning,
/// not a bug fix, and it is recorded as one.
///
/// [`with_metadata_from`]: crate::editing::metadata::with_metadata_from
pub fn set_anchor_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    name: &str,
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    if let Some(previous) = anchor_at(node, segments)
        && previous != name
        && any_alias_names(node, previous)
    {
        return Err("anchor-is-referenced".to_string());
    }
    apply_metadata_path(node, segments, source, line_offsets, |n| {
        n.set_anchor(name.to_string());
    })
}

/// Remove the anchor on the node at `segments`.
///
/// Refused under the same condition as [`set_anchor_path`]: with an alias naming it, removal leaves
/// the emitted document pointing at a name that is no longer defined anywhere.
pub fn remove_anchor_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    if let Some(previous) = anchor_at(node, segments)
        && any_alias_names(node, previous)
    {
        return Err("anchor-is-referenced".to_string());
    }
    apply_metadata_path(node, segments, source, line_offsets, |n| {
        n.remove_anchor();
    })
}

/// Set the scalar style on the node at `segments`.
pub fn set_scalar_style_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    style: crate::ast::ScalarStyle,
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    apply_metadata_path(node, segments, source, line_offsets, |n| {
        n.set_scalar_style(style);
    })
}

/// Set the flow style on the node at `segments`.
pub fn set_flow_style_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    flow: bool,
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    apply_metadata_path(node, segments, source, line_offsets, |n| {
        n.set_flow_style(flow);
    })
}

/// Set the chomping indicator on the node at `segments`.
pub fn set_chomping_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    chomp: crate::ast::Chomping,
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    apply_metadata_path(node, segments, source, line_offsets, |n| {
        n.set_chomping(chomp);
    })
}

/// Parse a scalar style string into `ScalarStyle`.
///
/// - `"plain"` → `Plain`
/// - `"single_quoted"` → `SingleQuoted`
/// - `"double_quoted"` → `DoubleQuoted`
/// - `"literal"` → `Literal`
/// - `"folded"` → `Folded`
pub fn parse_style(s: &str) -> Result<crate::ast::ScalarStyle, String> {
    match s {
        "plain" => Ok(crate::ast::ScalarStyle::Plain),
        "single_quoted" => Ok(crate::ast::ScalarStyle::SingleQuoted),
        "double_quoted" => Ok(crate::ast::ScalarStyle::DoubleQuoted),
        "literal" => Ok(crate::ast::ScalarStyle::Literal),
        "folded" => Ok(crate::ast::ScalarStyle::Folded),
        _ => Err(format!(
            "unknown scalar style '{}': expected one of plain, single_quoted, double_quoted, literal, folded",
            s
        )),
    }
}

/// Parse a chomping string into `Chomping`.
///
/// - `"strip"` → `Strip`
/// - `"clip"` → `Clip`
/// - `"keep"` → `Keep`
pub fn parse_chomping(s: &str) -> Result<crate::ast::Chomping, String> {
    match s {
        "strip" => Ok(crate::ast::Chomping::Strip),
        "clip" => Ok(crate::ast::Chomping::Clip),
        "keep" => Ok(crate::ast::Chomping::Keep),
        _ => Err(format!(
            "unknown chomping '{}': expected one of strip, clip, keep",
            s
        )),
    }
}

/// Format a `ScalarStyle` as the string used in the Python API.
pub fn style_to_str(style: crate::ast::ScalarStyle) -> &'static str {
    match style {
        crate::ast::ScalarStyle::Plain => "plain",
        crate::ast::ScalarStyle::SingleQuoted => "single_quoted",
        crate::ast::ScalarStyle::DoubleQuoted => "double_quoted",
        crate::ast::ScalarStyle::Literal => "literal",
        crate::ast::ScalarStyle::Folded => "folded",
    }
}

/// Format a `Chomping` as the string used in the Python API.
pub fn chomping_to_str(chomp: crate::ast::Chomping) -> &'static str {
    match chomp {
        crate::ast::Chomping::Strip => "strip",
        crate::ast::Chomping::Clip => "clip",
        crate::ast::Chomping::Keep => "keep",
    }
}

/// Extract the string key of a mapping key node for sorting. Non-scalar keys
/// fall back to a stable deterministic representation.
fn scalar_key_str(key: &CustomNode) -> String {
    match key {
        CustomNode::Scalar { value, .. } => value.as_ref().to_string(),
        CustomNode::Null { .. } => String::new(),
        _ => format!("{:?}", key),
    }
}

/// Sort the keys of a mapping node at `segments` in place.
pub fn sort_keys_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<DirtyUnit, String> {
    apply_metadata_path(node, segments, source, line_offsets, |n| {
        if let CustomNode::Mapping { pairs, .. } = n {
            pairs.sort_by(|ka, _va, kb, _vb| {
                let sa = scalar_key_str(ka);
                let sb = scalar_key_str(kb);
                sa.cmp(&sb)
            });
        }
    })
}

/// Move a subtree from `src_segments` to `dst_segments` (inside the same
/// document). Returns the INSERT unit (destination) and the DELETE unit
/// (source) as a batch: splicing must rewrite both regions - a merged
/// single unit silently left the moved-from key in the text (the bindings
/// applied them in sequence via their batch path).
pub fn move_path(
    node: &mut CustomNode,
    src_segments: &[Segment<'_>],
    dst_segments: &[Segment<'_>],
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<Vec<DirtyUnit>, String> {
    // Clone the source subtree
    let src = crate::editing::navigate(node, src_segments)
        .map_err(|e| e.to_string())?
        .clone();
    // Set at destination
    let set_unit = set_path(node, dst_segments, src, false, source, line_offsets, false)?;
    // Delete source
    let del_unit = delete_path(node, src_segments, source, line_offsets)?;
    Ok(vec![set_unit, del_unit])
}

/// Apply multiple set operations at once. Each pair is a (segments, new_value).
/// Returns a DirtyUnit for each operation.
pub fn set_many_path(
    node: &mut CustomNode,
    pairs: &[(Vec<Segment<'_>>, CustomNode)],
    source: &str,
    line_offsets: Option<&[usize]>,
) -> Result<Vec<DirtyUnit>, String> {
    let mut units = Vec::with_capacity(pairs.len());
    for (segments, new_value) in pairs {
        let unit = set_path(
            node,
            segments,
            new_value.clone(),
            false,
            source,
            line_offsets,
            false,
        )?;
        units.push(unit);
    }
    Ok(units)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::parser::yaml::Schema;
    use crate::serializer::to_yaml;

    fn segments(key: &str) -> Vec<Segment<'static>> {
        vec![Segment::Key(Cow::Owned(key.to_string()))]
    }

    /// The shape `edit_yaml` found: the verb succeeded, and the text it wrote named an anchor that
    /// was no longer defined. A `set` that produces an unreadable document is not a partial success,
    /// so the refusal happens before the tree is touched and the document is left exactly as it was.
    #[test]
    fn renaming_an_anchor_an_alias_names_is_refused_and_changes_nothing() {
        let source = "base: &x 1\nuse: *x\n";
        let mut node = parse(source, Schema::Core).expect("the fixture parses");
        let error = set_anchor_path(&mut node, &segments("base"), "a0", source, None)
            .expect_err("a rename nothing refers to must still be refused");
        assert_eq!(error, "anchor-is-referenced");
        assert_eq!(to_yaml(&node), source, "a refusal must not touch the tree");
    }

    /// Removal breaks the same reference in the same way, so it asks the same question.
    #[test]
    fn removing_an_anchor_an_alias_names_is_refused() {
        let source = "base: &x 1\nuse: *x\n";
        let mut node = parse(source, Schema::Core).expect("the fixture parses");
        let error = remove_anchor_path(&mut node, &segments("base"), source, None)
            .expect_err("an anchored name the document points at cannot be taken away");
        assert_eq!(error, "anchor-is-referenced");
        assert_eq!(to_yaml(&node), source);
    }

    /// An alias is a reference whatever position it sits in: `? *x` reads the anchor as a key, and a
    /// rename would orphan it just as surely as the value case. The spelling is the explicit-key
    /// marker, which is what the engine's own alias-key tests use (`a: &x [1, 2]\n? *x\n: v`).
    #[test]
    fn an_alias_in_key_position_counts_as_a_reference() {
        let source = "a: &x [1, 2]\n? *x\n: v\n";
        let mut node = parse(source, Schema::Core).expect("the fixture parses");
        let error = set_anchor_path(&mut node, &segments("a"), "z", source, None)
            .expect_err("a key alias is still a reference");
        assert_eq!(error, "anchor-is-referenced");
    }

    /// The same rename, one step further: an anchor nothing points at is the caller's to change.
    #[test]
    fn renaming_an_anchor_nothing_refers_to_still_renames() {
        let source = "key: &old value\n";
        let mut node = parse(source, Schema::Core).expect("the fixture parses");
        set_anchor_path(&mut node, &segments("key"), "new", source, None)
            .expect("no alias names `old`, so the rename is the caller's to make");
        assert_eq!(to_yaml(&node), "key: &new value\n");
    }

    /// The class is not about one verb. `set_path` with `preserve_metadata: false` drops the replaced
    /// node's metadata, and a name the document points at must not go that way quietly either.
    #[test]
    fn replacing_a_referenced_anchor_without_metadata_is_refused() {
        let source = "base: &x 1\nuse: *x\n";
        let mut node = parse(source, Schema::Core).expect("the fixture parses");
        let error = set_path(
            &mut node,
            &segments("base"),
            CustomNode::plain_scalar("2"),
            false,
            source,
            None,
            false,
        )
        .expect_err("the emitted `use: *x` would name an anchor that no longer exists");
        assert_eq!(error, "anchor-is-referenced");
        assert_eq!(to_yaml(&node), source);
    }

    /// The other side of the same call: with its metadata carried over, `with_metadata_from` keeps the
    /// document's anchor name, the reference stays alive, and the edit is the caller's to make.
    #[test]
    fn replacing_a_referenced_anchor_with_metadata_keeps_the_name() {
        let source = "base: &x 1\nuse: *x\n";
        let mut node = parse(source, Schema::Core).expect("the fixture parses");
        set_path(
            &mut node,
            &segments("base"),
            CustomNode::plain_scalar("2"),
            true,
            source,
            None,
            false,
        )
        .expect("a replacement that keeps the metadata keeps the anchor with it");
        let emitted = to_yaml(&node);
        assert_eq!(emitted, "base: &x 2\nuse: *x\n");
        parse(&emitted, Schema::Core).expect("the emission still parses");
    }

    /// The hole `preserve_metadata` cannot cover: the anchor is not on the node being replaced, it is
    /// one level *inside* it, so the copy has nothing to carry and the alias is orphaned anyway. Found
    /// by `edit_yaml` (crash-fbd364b3) on `: abase: &x 1\nuse: *x`, written here as the plain shape of
    /// the same tree: the path names a mapping with no anchor of its own.
    #[test]
    fn replacing_a_container_that_holds_a_referenced_anchor_is_refused() {
        let source = "a:\n  base: &x 1\nuse: *x\n";
        let mut node = parse(source, Schema::Core).expect("the fixture parses");
        let error = set_path(
            &mut node,
            &segments("a"),
            CustomNode::plain_scalar("2"),
            true,
            source,
            None,
            false,
        )
        .expect_err("`a: 2` would delete the only definition of the anchor `use: *x` names");
        assert_eq!(error, "anchor-is-referenced");
        assert_eq!(to_yaml(&node), source, "a refusal must not touch the tree");
    }

    /// The guard is about a reference, not about the presence of an anchor: nothing points at the
    /// nested name here, so replacing the container is the caller's to make.
    #[test]
    fn replacing_a_container_whose_anchor_nothing_refers_to_is_allowed() {
        let source = "a:\n  base: &x 1\n";
        let mut node = parse(source, Schema::Core).expect("the fixture parses");
        set_path(
            &mut node,
            &segments("a"),
            CustomNode::plain_scalar("2"),
            true,
            source,
            None,
            false,
        )
        .expect("no alias names `x`, so the nested anchor is not a reference");
        assert_eq!(to_yaml(&node), "a: 2\n");
    }

    /// `move_path` is one `set_path` call over a cloned subtree, so the guard it inherits is the same.
    #[test]
    fn moving_onto_a_referenced_anchor_is_refused() {
        let source = "base: &x 1\nuse: *x\n";
        let mut node = parse(source, Schema::Core).expect("the fixture parses");
        let error = move_path(&mut node, &segments("use"), &segments("base"), source, None)
            .expect_err("the move would overwrite the anchor its own payload names");
        assert_eq!(error, "anchor-is-referenced");
        assert_eq!(to_yaml(&node), source);
    }

    /// The third door: removing the node that *defines* a referenced anchor orphans the alias.
    /// Measured on the same fixture, moving `$.base` away emitted `use: *x` alone - a document naming
    /// an anchor nothing defines, refused by this library's own reader.
    #[test]
    fn deleting_the_definition_of_a_referenced_anchor_is_refused() {
        let source = "base: &x 1\nuse: *x\n";
        let mut node = parse(source, Schema::Core).expect("the fixture parses");
        let error = delete_path(&mut node, &segments("base"), source, None)
            .expect_err("the alias would outlive the anchor it names");
        assert_eq!(error, "anchor-is-referenced");
        assert_eq!(to_yaml(&node), source);
    }

    /// The guard is about the definition, not about every node that mentions the name: deleting the
    /// alias leaves the anchor and the document intact.
    #[test]
    fn deleting_the_alias_itself_is_allowed() {
        let source = "base: &x 1\nuse: *x\n";
        let mut node = parse(source, Schema::Core).expect("the fixture parses");
        delete_path(&mut node, &segments("use"), source, None)
            .expect("nothing refers to the node being removed");
        let emitted = to_yaml(&node);
        assert_eq!(emitted, "base: &x 1\n");
        parse(&emitted, Schema::Core).expect("the emission still parses");
    }
}
