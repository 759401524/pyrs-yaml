mod metadata;

pub use crate::editing::{
    eligible_path, extend_delete_over_comments, key_eq, line_aligned, line_end, line_indent,
    line_start, mapping_get_mut, mapping_key_index, nav_err, navigate, navigate_mut,
    normalize_index, parse_path_segments, path_nodes, precompute, regenerate_region_text,
    region_unit, DirtyKind, DirtyUnit, NavigateError, Segment,
};
pub use metadata::with_metadata_from;

pub mod segment_py;

use crate::ast::CustomNode;
use crate::parser::yaml::compute_line_offsets;
use indexmap::IndexMap;

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
        let eligible = eligible_path(&path_nodes(node, segments).map_err(nav_err)?);
        *node = new_value;
        let text = crate::serializer::to_yaml_with_options(
            &*node,
            &crate::serializer::SerializeOptions::default(),
        )?;
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

    let (eligible, compact_override) =
        precompute(node, segments, parent_segments, line_offsets, source);

    let parent = match navigate_mut(node, parent_segments) {
        Ok(parent) => parent,
        Err(NavigateError::Missing(_)) if create_missing => {
            return set_path_create_missing(node, segments, new_value, source, line_offsets);
        }
        Err(e) => return Err(nav_err(e)),
    };
    let parent_is_alias = matches!(parent, CustomNode::Alias { .. });

    match (parent, last) {
        (CustomNode::Mapping { pairs, .. }, Segment::Key(k)) => {
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
                            (s, e, line_indent(line_offsets, source, s))
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
                        segments,
                        parent_segments,
                        None,
                        &compact_override,
                        old_indent,
                        depth,
                    )?;
                    Ok(DirtyUnit {
                        kind: region_unit(
                            old_key_start,
                            old_value_end,
                            old_indent,
                            line_offsets,
                            source,
                            compact_override,
                            &text,
                        ),
                        eligible,
                    })
                }
                None => {
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
            let range = line_aligned(raw, line_offsets, source);
            let indent = line_indent(line_offsets, source, raw_start);
            let text = crate::serializer::item_to_string(&items[idx], indent, true, depth)?;
            Ok(DirtyUnit {
                kind: DirtyKind::Region {
                    range,
                    indent,
                    text,
                },
                eligible,
            })
        }
        _ if parent_is_alias => Err("cannot-edit-alias".to_string()),
        _ => Err("create-needs-mapping".to_string()),
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
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar(k.as_ref()), nested);
        nested = CustomNode::plain_mapping(pairs);
    }
    let top_key = match &tail[0] {
        Segment::Key(k) => CustomNode::plain_scalar(k.as_ref()),
        Segment::Index(_) => return Err("create-needs-mapping".to_string()),
    };
    let path = path_nodes(node, &segments[..consumed]).map_err(nav_err)?;
    let eligible = eligible_path(&path);
    let parent = navigate_mut(node, &segments[..consumed]).map_err(nav_err)?;
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
    let text = crate::serializer::pair_to_string(&top_key, &nested, indent, true, depth)?;
    pairs.insert(top_key, nested);
    Ok(DirtyUnit {
        kind: DirtyKind::Insert { at, text },
        eligible,
    })
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
            let (at, indent, is_last) = if insert_at < items.len() {
                let next_item = items[insert_at].source_range().cloned().unwrap_or(0..0);
                (
                    line_start(line_offsets, next_item.start),
                    line_indent(line_offsets, source, next_item.start),
                    false,
                )
            } else {
                let last_item = items[items.len() - 1]
                    .source_range()
                    .cloned()
                    .unwrap_or(0..0);
                (
                    line_end(line_offsets, last_item.end, source.len()),
                    line_indent(line_offsets, source, last_item.start),
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
    let eligible = {
        let path = path_nodes(node, segments).map_err(nav_err)?;
        eligible_path(&path)
    };
    let seq = navigate_mut(node, segments).map_err(nav_err)?;
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
    let parent = navigate_mut(node, parent_segments).map_err(nav_err)?;
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
            )?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse, parse_with_options, yaml::YamlSchema};
    use std::borrow::Cow;

    fn doc(yaml: &str) -> CustomNode {
        parse(yaml, YamlSchema::Core).unwrap()
    }

    fn doc_with_ranges(yaml: &str) -> CustomNode {
        parse_with_options(yaml, true, YamlSchema::Core, 1000, false).unwrap()
    }

    #[test]
    fn probe_eligible_root() {
        let node = doc_with_ranges("a: 1\n");
        let path = path_nodes(&node, &[]).unwrap();
        eprintln!(
            "root_flow={} root_range={:?}",
            crate::editing::region::node_is_flow(&node),
            node.source_range()
        );
        if let CustomNode::Mapping { pairs, .. } = &node {
            for (k, v) in pairs.iter() {
                eprintln!(
                    "k_range={:?} v_range={:?}",
                    k.source_range(),
                    v.source_range()
                );
            }
        }
        eprintln!("eligible={}", eligible_path(&path));
        let path_full = path_nodes(&node, &[Segment::Key(Cow::Borrowed("b"))]);
        eprintln!("path_full={:?}", path_full);
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
            None,
            false,
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
            None,
            false,
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
            None,
            false,
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
            None,
            false,
        )
        .unwrap_err();
        assert_eq!(err, "cannot-edit-alias");
    }

    #[test]
    fn delete_path_negative_index() {
        let mut node = doc("- a\n- b\n- c");
        delete_path(&mut node, &[Segment::Index(-2)], "- a\n- b\n- c", None).unwrap();
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
            None,
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
            None,
            false,
        )
        .unwrap();
        match unit.kind {
            DirtyKind::Region { range, indent, .. } => {
                assert_eq!(range, 5..10);
                assert_eq!(indent, 0);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn delete_item_region_is_item_lines() {
        let mut node = doc_with_ranges("- a\n- b\n- c\n");
        let unit = delete_path(&mut node, &[Segment::Index(1)], "- a\n- b\n- c\n", None).unwrap();
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
            None,
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
            None,
            false,
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
            None,
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
            None,
            false,
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
            None,
            false,
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
    fn create_missing_nested_insert_at_deepest_mapping() {
        let mut node = doc_with_ranges("a: 1\n");
        let unit = set_path(
            &mut node,
            &[
                Segment::Key(Cow::Borrowed("b")),
                Segment::Key(Cow::Borrowed("c")),
            ],
            CustomNode::plain_scalar("2"),
            true,
            "a: 1\n",
            None,
            true,
        )
        .unwrap();
        match unit.kind {
            DirtyKind::Insert { at, text } => {
                assert_eq!(at, 5);
                assert_eq!(text, "b:\n  c: 2\n");
            }
            _ => panic!(),
        }
        assert!(unit.eligible);
        assert_eq!(crate::serializer::to_yaml(&node), "a: 1\nb:\n  c: 2\n");
    }

    #[test]
    fn create_missing_mixed_existing_and_missing() {
        let mut node = doc_with_ranges("a:\n  x: 1\n");
        let unit = set_path(
            &mut node,
            &[
                Segment::Key(Cow::Borrowed("a")),
                Segment::Key(Cow::Borrowed("y")),
                Segment::Key(Cow::Borrowed("z")),
            ],
            CustomNode::plain_scalar("2"),
            true,
            "a:\n  x: 1\n",
            None,
            true,
        )
        .unwrap();
        match unit.kind {
            DirtyKind::Insert { at, text } => {
                assert_eq!(at, 10);
                assert_eq!(text, "  y:\n    z: 2\n");
            }
            _ => panic!(),
        }
        assert_eq!(
            crate::serializer::to_yaml(&node),
            "a:\n  x: 1\n  y:\n    z: 2\n"
        );
    }

    #[test]
    fn create_missing_empty_document_builds_full_chain() {
        let mut node = doc("");
        let unit = set_path(
            &mut node,
            &[
                Segment::Key(Cow::Borrowed("a")),
                Segment::Key(Cow::Borrowed("b")),
            ],
            CustomNode::plain_scalar("1"),
            true,
            "",
            None,
            true,
        )
        .unwrap();
        match unit.kind {
            DirtyKind::Insert { at, text } => {
                assert_eq!(at, 0);
                assert_eq!(text, "a:\n  b: 1\n");
            }
            _ => panic!(),
        }
        assert!(!unit.eligible);
        assert_eq!(crate::serializer::to_yaml(&node), "a:\n  b: 1\n");
    }

    #[test]
    fn create_missing_index_segment_still_errors() {
        let mut node = doc("a: 1\n");
        let err = set_path(
            &mut node,
            &[Segment::Key(Cow::Borrowed("b")), Segment::Index(0)],
            CustomNode::plain_scalar("2"),
            true,
            "a: 1\n",
            None,
            true,
        )
        .unwrap_err();
        assert_eq!(err, "create-needs-mapping");
        let mut node = doc("a:\n  - 1\n");
        let err = set_path(
            &mut node,
            &[Segment::Key(Cow::Borrowed("a")), Segment::Index(5)],
            CustomNode::plain_scalar("2"),
            true,
            "a:\n  - 1\n",
            None,
            true,
        )
        .unwrap_err();
        assert_eq!(err, "index-out-of-range-edit:5");
    }

    #[test]
    fn create_missing_scalar_intermediate_errors() {
        let mut node = doc("a: 1\n");
        let err = set_path(
            &mut node,
            &[
                Segment::Key(Cow::Borrowed("a")),
                Segment::Key(Cow::Borrowed("b")),
            ],
            CustomNode::plain_scalar("2"),
            true,
            "a: 1\n",
            None,
            true,
        )
        .unwrap_err();
        assert_eq!(err, "create-needs-mapping");
    }

    #[test]
    fn append_uses_last_item_end() {
        let mut node = doc_with_ranges("- a\n- b\n");
        let unit = append_path(
            &mut node,
            &[],
            CustomNode::plain_scalar("c"),
            "- a\n- b\n",
            None,
        )
        .unwrap();
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
            None,
            false,
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
            None,
            false,
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
            None,
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
            None,
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
        let anchored = CustomNode::Mapping {
            pairs: IndexMap::from([(CustomNode::plain_scalar("k"), CustomNode::plain_scalar("v"))]),
            comment: None,
            anchor: Some("x".to_string()),
            tag: None,
            flow_style: false,
            source_range: None,
        };
        assert!(!crate::serializer::is_compact_item(&anchored));
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

    #[test]
    fn cached_offsets_match_fresh_compute() {
        let source = "a: 1\nb: 2\nc: 3\n";
        let offsets = compute_line_offsets(source);
        let segs = &[Segment::Key(Cow::Borrowed("b"))];
        let mut n1 = crate::ast::CustomNode::plain_mapping(indexmap::IndexMap::new());
        let mut n2 = n1.clone();
        let unit1 = set_path(
            &mut n1,
            segs,
            crate::ast::CustomNode::plain_scalar("9"),
            true,
            source,
            Some(&offsets),
            false,
        )
        .unwrap();
        let unit2 = set_path(
            &mut n2,
            segs,
            crate::ast::CustomNode::plain_scalar("9"),
            true,
            source,
            None,
            false,
        )
        .unwrap();
        assert_eq!(unit1, unit2);
    }
}
