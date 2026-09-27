//! Multi-document splice orchestration for `pyq` edit commands.
//!
//! Route 2 from the design (`pyq-multidoc-edit-design.md`): every
//! document owns an independent `SpliceState` over its text segment, so
//! a layout-dirty or splice-rejected document falls back alone and never
//! de-pins its neighbours. Segment boundaries need no parser support -
//! `parse_all` already yields globally-offset `source_range`s, and each
//! segment runs from the previous document's line-end to this document's
//! line-end, which puts every `---` marker and its leading notes in the
//! following prelude untouched by design (plan units only ever address
//! node ranges inside one document).
//!
//! Failure semantics (matching the Python CLI's per-doc try/skip):
//! a plan error on one document skips it (skipped documents keep their
//! original segment bytes, so a half-mutated AST can never leak into the
//! output); zero applied documents is the command error.

use pyrs_yaml_core::ast::CustomNode;
use pyrs_yaml_core::editing::DirtyUnit;
use pyrs_yaml_core::parser::yaml::compute_line_offsets;
use pyrs_yaml_core::{parser, serializer, splice::SpliceState};
use std::ops::Range;
use std::sync::Arc;

/// One document's text segment plus its root node range, both absolute
/// offsets into the stream source.
#[derive(Debug, Clone, PartialEq)]
pub struct DocSegment {
    pub seg: Range<usize>,
    pub root: Range<usize>,
}

/// Derive per-document segments from globally-offset root ranges.
pub fn doc_segments(src: &str, docs: &[CustomNode]) -> Vec<DocSegment> {
    let roots: Vec<Range<usize>> = docs
        .iter()
        .map(|d| d.source_range().cloned().unwrap_or(0..0))
        .collect();
    let line_end = |at: usize| match src[at..].find('\n') {
        Some(i) => at + i + 1,
        None => src.len(),
    };
    let mut out = Vec::with_capacity(docs.len());
    let mut start = 0;
    for (i, root) in roots.iter().enumerate() {
        let end = if i + 1 < roots.len() {
            line_end(root.end)
        } else {
            src.len()
        };
        out.push(DocSegment {
            seg: start..end,
            root: root.clone(),
        });
        start = end;
    }
    out
}

/// Accumulates per-document splice state across one command's plan calls
/// and renders the final stream.
pub struct MultiDocEditor {
    src: Arc<str>,
    segs: Vec<DocSegment>,
    offsets: Vec<usize>,
    states: Vec<Option<SpliceState>>,
    touched: Vec<bool>,
}

impl MultiDocEditor {
    /// Eligibility is computed per document against the whole source
    /// (offsets are global); each candidate state bases on the document's
    /// own segment slice.
    pub fn new(src: &str, docs: &[CustomNode]) -> Self {
        let segs = doc_segments(src, docs);
        let offsets = compute_line_offsets(src);
        let states = segs
            .iter()
            .zip(docs)
            .map(|(s, d)| {
                parser::check_default_layout(d, src)
                    .then(|| SpliceState::new(Arc::from(&src[s.seg.clone()])))
            })
            .collect();
        Self {
            src: Arc::from(src),
            segs,
            offsets,
            states,
            touched: vec![false; docs.len()],
        }
    }

    pub fn offsets(&self) -> &[usize] {
        &self.offsets
    }

    /// Run `plan` for document `i` against the whole-stream coordinates,
    /// then splice its (shifted) units into that document's segment.
    /// A plan error propagates for the caller to skip-or-abort; on Ok the
    /// document is marked touched.
    pub fn edit(
        &mut self,
        i: usize,
        doc: &mut CustomNode,
        plan: impl FnOnce(&mut CustomNode, Option<&[usize]>) -> Result<Vec<DirtyUnit>, String>,
    ) -> Result<(), String> {
        let units = plan(doc, Some(&self.offsets))?;
        self.touched[i] = true;
        let base = self.segs[i].seg.start;
        if let Some(state) = self.states[i].as_mut() {
            for unit in &units {
                if state.apply(&unit.shifted(base)).is_err() {
                    self.states[i] = None;
                    break;
                }
            }
        }
        Ok(())
    }

    /// Render the (possibly edited) stream. Per segment: materialized
    /// splice result, else original bytes when untouched, else prelude +
    /// re-serialized root + trailing (a dirty doc still keeps its `---`
    /// marker and leading notes verbatim).
    pub fn finalize(&self, docs: &[CustomNode]) -> String {
        let mut out = String::with_capacity(self.src.len());
        for (i, seg) in self.segs.iter().enumerate() {
            if let Some(text) = self.states[i].as_ref().and_then(|s| s.materialize()) {
                out.push_str(&text);
                continue;
            }
            let sliced = &self.src[seg.seg.clone()];
            if !self.touched[i] {
                out.push_str(sliced);
                continue;
            }
            let pre = &sliced[..seg.root.start - seg.seg.start];
            let post = &sliced[seg.root.end - seg.seg.start..];
            let mut body = serializer::to_yaml(&docs[i]);
            if post.is_empty() && !body.ends_with('\n') {
                body.push('\n');
            }
            out.push_str(pre);
            out.push_str(&body);
            out.push_str(post);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan;
    use pyrs_yaml_core::parser::yaml::Schema;
    use std::borrow::Cow;

    fn parse_docs(src: &str) -> Vec<CustomNode> {
        parser::parse_all(src, Schema::Core).unwrap()
    }

    #[test]
    fn segments_partition_whole_text_and_own_separators() {
        let src = "---\nname: one\n---\nname: two\n";
        let docs = parse_docs(src);
        assert_eq!(docs.len(), 2);
        let segs = doc_segments(src, &docs);
        // contiguous coverage
        assert_eq!(segs[0].seg.start, 0);
        assert_eq!(segs[0].seg.end, segs[1].seg.start);
        assert_eq!(segs[1].seg.end, src.len());
        // each doc's own `---` line lives in ITS segment prelude
        assert!(src[segs[0].seg.clone()].starts_with("---\n"));
        assert!(src[segs[1].seg.clone()].starts_with("---\n"));
    }

    #[test]
    fn untouched_docs_keep_exact_bytes_when_one_is_edited() {
        let src = "---\nname: one\nextra: 1\n---\nname: two\nweird:   2\n";
        let mut docs = parse_docs(src);
        let mut ed = MultiDocEditor::new(src, &docs);
        let seg = [Segment::Key(Cow::Borrowed("name"))];
        let value = parser::parse("\"updated\"", Schema::Core).unwrap();
        ed.edit(1, &mut docs[1], |node, offs| {
            plan::set_path(node, &seg, value.clone(), true, src, offs, false).map(|u| vec![u])
        })
        .unwrap();
        let out = ed.finalize(&docs);
        // doc 0 verbatim (incl. its separator); doc 1's edited line plus
        // its odd-spacing neighbour pinned byte-for-byte
        assert_eq!(
            out,
            "---\nname: one\nextra: 1\n---\nname: \"updated\"\nweird:   2\n"
        );
    }

    #[test]
    fn dirty_doc_falls_back_alone_and_keeps_prelude_and_neighbours() {
        // doc 2 uses non-default indentation for its nested mapping
        let src = "a: 1\n---\nb:\n    deep: 1   # four-space child\nc: 2\n---\nd: 3\n";
        let mut docs = parse_docs(src);
        assert_eq!(docs.len(), 3);
        let mut ed = MultiDocEditor::new(src, &docs);
        let seg = [Segment::Key(Cow::Borrowed("b"))];
        let value = parser::parse("99", Schema::Core).unwrap();
        ed.edit(1, &mut docs[1], |node, offs| {
            plan::set_path(node, &seg, value.clone(), true, src, offs, false).map(|u| vec![u])
        })
        .unwrap();
        let out = ed.finalize(&docs);
        // neighbours byte-identical
        assert!(out.starts_with("a: 1\n---\n"), "{out:?}");
        assert!(out.contains("---\nd: 3\n"), "{out:?}");
        // the dirty doc re-dumped but kept its separator prelude verbatim
        assert!(out.contains("---\nb: 99"), "{out:?}");
    }

    #[test]
    fn skipped_half_mutated_doc_never_leaks_into_output() {
        let src = "a: 1\n---\nno_target: 1\n";
        let mut docs = parse_docs(src);
        let mut ed = MultiDocEditor::new(src, &docs);
        let seg = [Segment::Key(Cow::Borrowed("missing"))];
        let r = ed.edit(1, &mut docs[1], |node, offs| {
            plan::delete_path(node, &seg, src, offs).map(|u| vec![u])
        });
        assert!(r.is_err(), "missing path must error for the caller to skip");
        // simulate caller skip: doc 1 never marked touched, its mutated AST
        // must not appear in the output
        let out = ed.finalize(&docs);
        assert_eq!(out, src);
    }
}
