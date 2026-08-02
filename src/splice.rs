//! Segment-based splice assembly: accumulate `DirtyUnit`s from the editing
//! primitives against an immutable original source and reassemble the
//! document on flush by splicing regenerated text into untouched bytes.
//!
//! Segments partition the ORIGINAL byte space `[0, base.len()]` (zero-width
//! `Owned` segments are insertion points). Untouched ranges stay `Borrowed`
//! slices of `base` (zero copy); regenerated ranges become `Owned` text.

use crate::py::editing::{DirtyKind, DirtyUnit};
use std::ops::Range;
use std::sync::Arc;

/// One contiguous piece of the reassembled document.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceSegment {
    /// Slice of the untouched original: `base[start..end]` (original coords).
    Borrowed { start: usize, end: usize },
    /// Regenerated text replacing ORIGINAL bytes `start..end`; a zero-width
    /// segment (`start == end`) is an insertion point.
    Owned {
        start: usize,
        end: usize,
        text: String,
    },
}

/// Marker error for a rejected splice unit. The caller falls back to a full
/// serialize; the state is left unchanged, so the rejection reason is
/// deliberately discarded.
#[derive(Debug)]
pub struct SpliceReject;

/// Accumulates edits against an immutable base until `materialize` reassembles
/// the full text. A rejected unit (ineligible, or an unresolvable overlap with
/// previously regenerated text) leaves the state untouched so the caller can
/// fall back to full serialization.
#[derive(Debug, Clone)]
pub struct SpliceState {
    pub segments: Vec<SourceSegment>,
    pub base: Arc<str>,
}

impl SpliceState {
    /// A fresh state borrowing the whole base as a single segment.
    pub fn new(base: Arc<str>) -> Self {
        Self {
            segments: vec![SourceSegment::Borrowed {
                start: 0,
                end: base.len(),
            }],
            base,
        }
    }

    /// Fold one edit unit into the segment list. `Err` means the unit cannot
    /// be spliced (ineligible, or it would overwrite previously regenerated
    /// text in an unresolvable way) and the state is UNCHANGED.
    pub fn apply(&mut self, unit: &DirtyUnit) -> Result<(), SpliceReject> {
        if !unit.eligible {
            return Err(SpliceReject);
        }
        match &unit.kind {
            DirtyKind::Insert { at, text } => self.insert(*at, text),
            DirtyKind::Delete { range } => self.replace_range(range, None),
            DirtyKind::Region { range, text, .. } => self.replace_range(range, Some(text)),
        }
    }

    /// Insert `text` at original-space offset `at` (a zero-width Owned point).
    fn insert(&mut self, at: usize, text: &str) -> Result<(), SpliceReject> {
        if at > self.base.len() {
            return Err(SpliceReject);
        }
        let mut out: Vec<SourceSegment> = Vec::with_capacity(self.segments.len() + 2);
        let mut inserted = false;
        for seg in &self.segments {
            if inserted {
                out.push(seg.clone());
                continue;
            }
            match seg {
                SourceSegment::Borrowed { start, end } => {
                    if at <= *start {
                        // At-or-before this segment's start: place the point here.
                        out.push(SourceSegment::Owned {
                            start: at,
                            end: at,
                            text: text.to_string(),
                        });
                        out.push(seg.clone());
                        inserted = true;
                    } else if at < *end {
                        // Strictly inside: split the borrowed slice.
                        out.push(SourceSegment::Borrowed {
                            start: *start,
                            end: at,
                        });
                        out.push(SourceSegment::Owned {
                            start: at,
                            end: at,
                            text: text.to_string(),
                        });
                        out.push(SourceSegment::Borrowed {
                            start: at,
                            end: *end,
                        });
                        inserted = true;
                    } else {
                        // at >= end: keep looking at the next segment.
                        out.push(seg.clone());
                    }
                }
                SourceSegment::Owned { start, end, .. } => {
                    if at > *start && at < *end {
                        return Err(SpliceReject);
                    }
                    if at == *start {
                        out.push(SourceSegment::Owned {
                            start: at,
                            end: at,
                            text: text.to_string(),
                        });
                        out.push(seg.clone());
                        inserted = true;
                    } else {
                        out.push(seg.clone());
                    }
                }
            }
        }
        if !inserted {
            out.push(SourceSegment::Owned {
                start: at,
                end: at,
                text: text.to_string(),
            });
        }
        self.segments = out;
        Ok(())
    }

    /// Replace (or, with `text == None`, delete) original bytes `range` with
    /// regenerated text. Fully-overlapped segments are discarded; a range
    /// crossing a previously-Owned segment's interior is rejected (P5).
    fn replace_range(
        &mut self,
        range: &Range<usize>,
        text: Option<&str>,
    ) -> Result<(), SpliceReject> {
        let mut out: Vec<SourceSegment> = Vec::with_capacity(self.segments.len() + 1);
        let mut inserted = false;
        for seg in &self.segments {
            match seg {
                SourceSegment::Borrowed { start, end } => {
                    if *end <= range.start {
                        out.push(seg.clone());
                    } else if *start >= range.end {
                        if !inserted {
                            if let Some(t) = text {
                                out.push(SourceSegment::Owned {
                                    start: range.start,
                                    end: range.end,
                                    text: t.to_string(),
                                });
                            }
                            inserted = true;
                        }
                        out.push(seg.clone());
                    } else if *start >= range.start && *end <= range.end {
                        if !inserted {
                            if let Some(t) = text {
                                out.push(SourceSegment::Owned {
                                    start: range.start,
                                    end: range.end,
                                    text: t.to_string(),
                                });
                            }
                            inserted = true;
                        }
                    } else {
                        // Partially covered: keep the parts outside the range.
                        let s2 = range.start.max(*start);
                        let e2 = range.end.min(*end);
                        if s2 >= e2 {
                            out.push(seg.clone());
                            continue;
                        }
                        if *start < range.start {
                            out.push(SourceSegment::Borrowed {
                                start: *start,
                                end: range.start,
                            });
                        }
                        if !inserted {
                            if let Some(t) = text {
                                out.push(SourceSegment::Owned {
                                    start: range.start,
                                    end: range.end,
                                    text: t.to_string(),
                                });
                            }
                            inserted = true;
                        }
                        if range.end < *end {
                            out.push(SourceSegment::Borrowed {
                                start: range.end,
                                end: *end,
                            });
                        }
                    }
                }
                SourceSegment::Owned { start, end, .. } => {
                    if *start >= range.start && *end <= range.end {
                        // Fully inside the range: replaced by the unit.
                        if !inserted {
                            if let Some(t) = text {
                                out.push(SourceSegment::Owned {
                                    start: range.start,
                                    end: range.end,
                                    text: t.to_string(),
                                });
                            }
                            inserted = true;
                        }
                        continue;
                    }
                    if *start < range.start && range.start < *end {
                        return Err(SpliceReject);
                    }
                    if *start < range.end && range.end < *end {
                        return Err(SpliceReject);
                    }
                    out.push(seg.clone());
                }
            }
        }
        if !inserted {
            if let Some(t) = text {
                out.push(SourceSegment::Owned {
                    start: range.start,
                    end: range.end,
                    text: t.to_string(),
                });
            }
        }
        self.segments = out;
        Ok(())
    }

    /// Reassemble the full document. `None` when every byte was deleted.
    pub fn materialize(&self) -> Option<String> {
        if self.segments.is_empty() {
            return None;
        }
        let mut out = String::with_capacity(self.base.len() + 16);
        for seg in &self.segments {
            match seg {
                SourceSegment::Borrowed { start, end } => out.push_str(&self.base[*start..*end]),
                SourceSegment::Owned { text, .. } => out.push_str(text),
            }
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(kind: DirtyKind, eligible: bool) -> DirtyUnit {
        DirtyUnit { kind, eligible }
    }

    fn region(base: &str, range: Range<usize>, text: &str) -> SpliceState {
        let mut s = SpliceState::new(Arc::from(base));
        s.apply(&unit(
            DirtyKind::Region {
                range,
                indent: 0,
                text: text.to_string(),
            },
            true,
        ))
        .unwrap();
        s
    }

    #[test]
    fn set_splices_only_region() {
        let s = region("a: 1\nb: 2\n", 5..10, "b: 9\n");
        assert_eq!(s.materialize(), Some("a: 1\nb: 9\n".to_string()));
    }

    #[test]
    fn insert_shifts_tail_bytes() {
        let mut s = SpliceState::new(Arc::from("- a\n- b\n"));
        s.apply(&unit(
            DirtyKind::Insert {
                at: 4,
                text: "- z\n".to_string(),
            },
            true,
        ))
        .unwrap();
        assert_eq!(s.materialize(), Some("- a\n- z\n- b\n".to_string()));
    }

    #[test]
    fn insert_at_eof_appends() {
        let mut s = SpliceState::new(Arc::from("- a\n- b\n"));
        s.apply(&unit(
            DirtyKind::Insert {
                at: 8,
                text: "- c\n".to_string(),
            },
            true,
        ))
        .unwrap();
        assert_eq!(s.materialize(), Some("- a\n- b\n- c\n".to_string()));
    }

    #[test]
    fn delete_removes_region() {
        let mut s = SpliceState::new(Arc::from("- a\n- b\n- c\n"));
        s.apply(&unit(DirtyKind::Delete { range: 4..8 }, true))
            .unwrap();
        assert_eq!(s.materialize(), Some("- a\n- c\n".to_string()));
    }

    #[test]
    fn materialize_is_identity_on_empty_ops() {
        let s = SpliceState::new(Arc::from("a: 1\nb: 2\n"));
        assert_eq!(s.materialize(), Some("a: 1\nb: 2\n".to_string()));
    }

    #[test]
    fn ineligible_unit_rejected() {
        let mut s = SpliceState::new(Arc::from("a: 1\n"));
        assert!(s
            .apply(&unit(
                DirtyKind::Insert {
                    at: 0,
                    text: "x\n".to_string(),
                },
                false,
            ))
            .is_err());
        // State untouched.
        assert_eq!(s.materialize(), Some("a: 1\n".to_string()));
    }

    #[test]
    fn delete_all_segments_yields_none() {
        let mut s = SpliceState::new(Arc::from("a: 1\n"));
        s.apply(&unit(DirtyKind::Delete { range: 0..5 }, true))
            .unwrap();
        assert_eq!(s.materialize(), None);
    }

    #[test]
    fn owned_replaced_by_covering_region() {
        let mut s = region("a: 1\nb: 2\nc: 3\n", 5..10, "b: 9\n");
        s.apply(&unit(
            DirtyKind::Region {
                range: 5..15,
                indent: 0,
                text: "b: 9\nc: 3\n".to_string(),
            },
            true,
        ))
        .unwrap();
        let out = s.materialize().unwrap();
        assert!(out.starts_with("a: 1\n"));
        assert!(out.ends_with("c: 3\n"));
        assert!(out.contains('b'));
    }

    #[test]
    fn insert_inside_owned_rejected() {
        let mut s = region("a: 1\nb: 2\nc: 3\n", 5..10, "b: 9\n");
        assert!(s
            .apply(&unit(
                DirtyKind::Insert {
                    at: 6,
                    text: "x\n".to_string(),
                },
                true,
            ))
            .is_err());
    }

    #[test]
    fn same_region_twice_replaces_owned() {
        let mut s = region("a: 1\nb: 2\nc: 3\n", 5..10, "b: 9\n");
        s.apply(&unit(
            DirtyKind::Region {
                range: 5..10,
                indent: 0,
                text: "b: 9\n".to_string(),
            },
            true,
        ))
        .unwrap();
        assert_eq!(s.materialize(), Some("a: 1\nb: 9\nc: 3\n".to_string()));
    }

    #[test]
    fn two_inserts_at_same_point_both_materialize() {
        let mut s = SpliceState::new(Arc::from("a: 1\nb: 2\n"));
        for _ in 0..2 {
            s.apply(&unit(
                DirtyKind::Insert {
                    at: 5,
                    text: "x\n".to_string(),
                },
                true,
            ))
            .unwrap();
        }
        assert_eq!(s.materialize(), Some("a: 1\nx\nx\nb: 2\n".to_string()));
    }

    #[test]
    fn sequential_regions_after_insert() {
        let mut s = SpliceState::new(Arc::from("a: 1\nb: 2\nc: 3\n"));
        s.apply(&unit(
            DirtyKind::Insert {
                at: 5,
                text: "x\n".to_string(),
            },
            true,
        ))
        .unwrap();
        s.apply(&unit(
            DirtyKind::Region {
                range: 10..15,
                indent: 0,
                text: "c: 8\n".to_string(),
            },
            true,
        ))
        .unwrap();
        assert_eq!(s.materialize(), Some("a: 1\nx\nb: 2\nc: 8\n".to_string()));
    }

    // ---- integration: parse -> edit -> splice. Mirrors the wiring of
    // `YamlDocument::from_ast` + the `_*_path` primitives; `YamlDocument`
    // itself is private to `mod py`, so these drive the same code path the
    // pyclass constructor seeds without needing a GIL token. ----

    use crate::ast::CustomNode;
    use crate::parser::yaml::YamlSchema;
    use crate::py::editing::{delete_path, parse_path_segments, set_path};

    fn parsed(source: &str) -> crate::ast::CustomNode {
        crate::parser::parse_with_options(source, true, YamlSchema::Core, 1000, false).unwrap()
    }

    fn is_eligible(ast: &CustomNode, source: &str) -> bool {
        crate::parser::check_default_layout(ast, source)
    }

    #[test]
    fn preserves_untouched_bytes() {
        let source = "a: 1\nb: 2\nc: 3\n";
        let mut ast = parsed(source);
        assert!(is_eligible(&ast, source));
        let mut splice = SpliceState::new(Arc::from(source));
        let segs = parse_path_segments("$.b").unwrap();
        let unit = set_path(&mut ast, &segs, CustomNode::plain_scalar("9"), true, source).unwrap();
        splice.apply(&unit).unwrap();
        assert_eq!(splice.materialize(), Some("a: 1\nb: 9\nc: 3\n".to_string()));
    }

    #[test]
    fn flow_region_edit_falls_back() {
        let source = "a: {b: 1}\n";
        let mut ast = parsed(source);
        assert!(is_eligible(&ast, source)); // the doc gate passes; flow fallback is per-unit
        let mut splice = SpliceState::new(Arc::from(source));
        // Editing inside a flow container is ineligible at the unit level, so
        // the splice rejects it and the caller falls back to a full serialize.
        let segs = parse_path_segments("$.a.b").unwrap();
        let unit = set_path(&mut ast, &segs, CustomNode::plain_scalar("2"), true, source).unwrap();
        assert!(!unit.eligible);
        assert!(splice.apply(&unit).is_err());
    }

    #[test]
    fn compact_roundtrip() {
        let source = "- a: 1\n  b: 2\n- c: 3\n";
        let mut ast = parsed(source);
        assert!(is_eligible(&ast, source));
        let mut splice = SpliceState::new(Arc::from(source));
        let segs = parse_path_segments("$[0].a").unwrap();
        let unit = set_path(&mut ast, &segs, CustomNode::plain_scalar("9"), true, source).unwrap();
        splice.apply(&unit).unwrap();
        assert_eq!(
            splice.materialize(),
            Some("- a: 9\n  b: 2\n- c: 3\n".to_string())
        );
    }

    #[test]
    fn set_creating_pair_inserts() {
        let source = "a: 1\n";
        let mut ast = parsed(source);
        assert!(is_eligible(&ast, source));
        let mut splice = SpliceState::new(Arc::from(source));
        let segs = parse_path_segments("$.b").unwrap();
        let unit = set_path(&mut ast, &segs, CustomNode::plain_scalar("2"), true, source).unwrap();
        splice.apply(&unit).unwrap();
        assert_eq!(splice.materialize(), Some("a: 1\nb: 2\n".to_string()));
    }

    #[test]
    fn multi_edit_burst_then_fallback() {
        let source = "- a: 1\n- b: 2\n- c: 3\n";
        let mut ast = parsed(source);
        assert!(is_eligible(&ast, source));
        let mut splice = SpliceState::new(Arc::from(source));
        for (path, val) in [("$[0]", "x"), ("$[1]", "y")] {
            let segs = parse_path_segments(path).unwrap();
            let unit =
                set_path(&mut ast, &segs, CustomNode::plain_scalar(val), true, source).unwrap();
            splice.apply(&unit).unwrap();
        }
        assert_eq!(splice.materialize(), Some("- x\n- y\n- c: 3\n".to_string()));
        // Deleting the FIRST key of a compact item orphans its dash line, so
        // delete_path marks the unit ineligible; apply rejects it and the
        // caller falls back to a full serialize (flush_source splice=None).
        let compact = "- a: 1\n  b: 2\n- c: 3\n";
        let mut ast2 = parsed(compact);
        assert!(is_eligible(&ast2, compact));
        let mut splice2 = SpliceState::new(Arc::from(compact));
        let segs = parse_path_segments("$[0].a").unwrap();
        let unit = delete_path(&mut ast2, &segs, compact).unwrap();
        assert!(!unit.eligible);
        assert!(splice2.apply(&unit).is_err());
    }

    #[test]
    fn root_scalar_replacement_splices_correctly() {
        let mut s = SpliceState::new(Arc::from("hello\n"));
        let unit = DirtyUnit {
            kind: DirtyKind::Region {
                range: 0..6,
                indent: 0,
                text: "world\n".into(),
            },
            eligible: true,
        };
        s.apply(&unit).unwrap();
        assert_eq!(s.materialize().unwrap(), "world\n");
    }
}
