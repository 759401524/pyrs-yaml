mod metadata;

pub(crate) use crate::editing::{
    DirtyKind, DirtyUnit, NavigateError, Segment, eligible_path, extend_delete_over_comments,
    key_eq, line_aligned, line_end, line_indent, line_start, mapping_key_index, navigate,
    navigate_mut, normalize_index, parse_path_segments, path_nodes, precompute,
    regenerate_region_text, region_unit,
};
pub(crate) use metadata::with_metadata_from;

pub mod segment_py;

use crate::ast::CustomNode;
use crate::parser::yaml::compute_line_offsets;
use indexmap::IndexMap;
use std::ops::Range;

pub fn set_path(
    node: &mut CustomNode,
    segments: &[Segment<'_>],
    new_value: CustomNode,
    preserve_metadata: bool,
    source: &str,
    line_offsets: Option<&[usize]>,
    create_missing: bool,
) -> Result<DirtyUnit, String> {
    let computed;
    let line_offsets: &[usize] = match line_offsets {
        Some(offs) => offs,
        None => {
            computed = compute_line_offsets(source);
            &computed
        }
    };

    if segments.is_empty() {
        let eligible = eligible_path(&path_nodes(node, segments).map_err(|e| e.to_string())?);
        *node = new_value;
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
    if matches!(node, CustomNode::Null { .. }) {
        *node = CustomNode::Mapping {
            pairs: IndexMap::new(),
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
            Ok(DirtyUnit {
                kind: region_unit(
                    old_key_start,
                    old_value_end,
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
            let text = crate::serializer::pair_to_string(&key_node, &new_value, indent, ctx.depth)
                .map_err(|e| e.to_string())?;
            pairs.insert(key_node, new_value);
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
    Ok(DirtyUnit {
        kind: DirtyKind::Region {
            range,
            indent,
            text,
        },
        eligible: ctx.eligible,
    })
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
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar(k.as_ref()), nested);
        nested = CustomNode::plain_mapping(pairs);
    }
    let top_key = match &tail[0] {
        Segment::Key(k) => CustomNode::plain_scalar(k.as_ref()),
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
    let computed;
    let line_offsets: &[usize] = match line_offsets {
        Some(offs) => offs,
        None => {
            computed = compute_line_offsets(source);
            &computed
        }
    };
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
    let computed;
    let line_offsets: &[usize] = match line_offsets {
        Some(offs) => offs,
        None => {
            computed = compute_line_offsets(source);
            &computed
        }
    };
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
    let computed;
    let line_offsets: &[usize] = match line_offsets {
        Some(offs) => offs,
        None => {
            computed = compute_line_offsets(source);
            &computed
        }
    };
    if segments.is_empty() {
        return Err("edit-error".to_string());
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
    let computed;
    let line_offsets: &[usize] = match line_offsets {
        Some(offs) => offs,
        None => {
            computed = compute_line_offsets(source);
            &computed
        }
    };
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
            let new_key_node = CustomNode::plain_scalar(new_key.to_string());
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
