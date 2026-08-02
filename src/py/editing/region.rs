use crate::ast::CustomNode;
use std::ops::Range;

use super::navigate::{mapping_key_index, normalize_index, NavigateError, Segment};

#[derive(Debug, Clone, PartialEq)]
pub enum DirtyKind {
    Region {
        range: Range<usize>,
        indent: usize,
        text: String,
    },
    Insert {
        at: usize,
        text: String,
    },
    Delete {
        range: Range<usize>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DirtyUnit {
    pub kind: DirtyKind,
    pub eligible: bool,
}

pub fn path_nodes<'a>(
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

pub fn node_is_flow(node: &CustomNode) -> bool {
    match node {
        CustomNode::Mapping { flow_style, .. } | CustomNode::Sequence { flow_style, .. } => {
            *flow_style
        }
        _ => false,
    }
}

pub fn eligible_path(path: &[&CustomNode]) -> bool {
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

pub fn compact_item_ancestor<'a>(path: &'a [&'a CustomNode]) -> Option<(usize, &'a CustomNode)> {
    for i in 1..path.len() {
        let node = path[i];
        let parent = path[i - 1];
        if matches!(parent, CustomNode::Sequence { .. }) && crate::serializer::is_compact_item(node)
        {
            return Some((i, node));
        }
    }
    None
}

pub fn region_unit(
    key_start: usize,
    value_end: usize,
    pair_indent: usize,
    line_offsets: &[usize],
    source: &str,
    compact_override: Option<(Range<usize>, usize, usize)>,
    text: &str,
) -> DirtyKind {
    if let Some((range, indent, _)) = compact_override {
        DirtyKind::Region {
            range,
            indent,
            text: text.to_string(),
        }
    } else {
        let range = line_aligned(key_start..value_end, line_offsets, source);
        DirtyKind::Region {
            range,
            indent: pair_indent,
            text: text.to_string(),
        }
    }
}

pub fn precompute(
    node: &CustomNode,
    segments: &[Segment<'_>],
    parent_segments: &[Segment<'_>],
    line_offsets: &[usize],
    source: &str,
) -> (bool, Option<(Range<usize>, usize, usize)>) {
    let path = match path_nodes(node, segments) {
        Ok(p) => p,
        Err(_) => path_nodes(node, parent_segments).unwrap_or_default(),
    };
    let eligible = eligible_path(&path);
    let compact_override = compact_item_ancestor(&path).and_then(|(idx, item)| {
        item.source_range().cloned().map(|r| {
            (
                line_aligned(r.clone(), line_offsets, source),
                line_indent(line_offsets, source, r.start),
                idx,
            )
        })
    });
    (eligible, compact_override)
}

pub fn regenerate_region_text(
    node: &CustomNode,
    segments: &[Segment<'_>],
    parent_segments: &[Segment<'_>],
    new_key_override: Option<&str>,
    compact_override: &Option<(Range<usize>, usize, usize)>,
    indent: usize,
    depth: usize,
) -> Result<String, String> {
    if let Some((_, override_indent, prefix_len)) = compact_override {
        let path = path_nodes(node, &segments[..*prefix_len]).map_err(nav_err)?;
        let item = path.last().ok_or_else(|| "edit-error".to_string())?;
        crate::serializer::item_to_string(item, *override_indent, true, depth)
    } else {
        let parent_path = path_nodes(node, parent_segments).map_err(nav_err)?;
        let parent = parent_path.last().ok_or_else(|| "edit-error".to_string())?;
        match parent {
            CustomNode::Mapping { pairs, .. } => {
                let key_text = match new_key_override {
                    Some(k) => k,
                    None => match segments.last() {
                        Some(Segment::Key(k)) => k.as_ref(),
                        _ => return Err("edit-error".to_string()),
                    },
                };
                let key_node = CustomNode::plain_scalar(key_text.to_string());
                let idx = mapping_key_index(pairs, &key_node)
                    .ok_or_else(|| "missing-path".to_string())?;
                let (k, v) = pairs
                    .get_index(idx)
                    .ok_or_else(|| "missing-path".to_string())?;
                crate::serializer::pair_to_string(k, v, indent, true, depth)
            }
            _ => Err("edit-error".to_string()),
        }
    }
}

pub fn line_index_of(line_offsets: &[usize], byte_offset: usize) -> usize {
    match line_offsets.binary_search(&byte_offset) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    }
}

pub fn line_start(line_offsets: &[usize], byte_offset: usize) -> usize {
    line_offsets[line_index_of(line_offsets, byte_offset)]
}

pub fn line_end(line_offsets: &[usize], byte_offset: usize, text_len: usize) -> usize {
    let idx = line_index_of(line_offsets, byte_offset);
    line_offsets.get(idx + 1).copied().unwrap_or(text_len)
}

pub fn line_indent(line_offsets: &[usize], text: &str, byte_offset: usize) -> usize {
    let start = line_start(line_offsets, byte_offset);
    let bytes = text.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    i - start
}

pub fn line_aligned(range: Range<usize>, line_offsets: &[usize], text: &str) -> Range<usize> {
    let start = line_start(line_offsets, range.start);
    let end = line_end(line_offsets, range.end, text.len());
    start..end
}

pub fn extend_delete_over_comments(
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

pub fn nav_err(e: NavigateError) -> String {
    match e {
        NavigateError::Missing(s) => format!("missing-path:{s}"),
        NavigateError::CannotDescend(t) => format!("cannot-descend-into-scalar:{t}"),
        NavigateError::NotContainer => "create-needs-mapping".to_string(),
    }
}
