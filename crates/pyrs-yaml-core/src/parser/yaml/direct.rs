//! The direct-load surface: which documents the value-only fast path can answer.
//!
//! The fast path materializes a YAML value straight from granit's event stream, without
//! building the round-trip AST. That is sound only for documents whose *value* the event
//! stream determines completely. Anchors, aliases, tags, merge keys, multi-document
//! streams and parse errors have to fall back to the AST pipeline, which owns the
//! authoritative diagnostics.
//!
//! The decision belongs to the engine rather than to the Python binding: it is a property
//! of the document and the grammar, not of the object the value is materialized into. The
//! binding crate supplies the materialization and asks this module whether the document is
//! inside the surface.
//!
//! Two cheap pre-vetoes run before the event stream is collected, because bailing late
//! costs a full collection pass plus the follow-up AST parse:
//!
//! - a single `&` byte rejects anchored documents. An alias cannot exist without an anchor
//!   definition, so the anchor marker covers both. The known false positive (`&` inside a
//!   quoted value, e.g. a URL) only costs that document the fast path.
//! - a separator *line* after the first line rejects multi-document streams. A leading
//!   `---` (an explicit single-document start) stays on the fast path.
//!
//! Both false positives are accepted: they cost one document its fast path, never a wrong
//! answer.

use alloc::vec::Vec;
use granit_parser::{Event, Parser as SaphyrParser, Span, SpannedEventReceiver};

/// A collected event with the span position granit reports at.
pub struct DirectEvent<'a> {
    /// The event itself; its scalar text borrows from the source.
    pub event: Event<'a>,
    /// Where granit reported the event.
    pub span: Span,
}

/// Phase 1: drain the parser into a Vec, bailing on anything the direct surface cannot
/// reproduce. Parse errors also bail: the AST path reruns and produces the authoritative,
/// byte-identical diagnostic.
struct Collector<'a> {
    events: Vec<DirectEvent<'a>>,
    bail: bool,
    docs: usize,
}

impl<'a> Collector<'a> {
    fn push(&mut self, event: Event<'a>, span: Span) {
        self.events.push(DirectEvent { event, span });
    }
}

impl<'a> SpannedEventReceiver<'a> for Collector<'a> {
    fn on_event(&mut self, event: Event<'a>, span: Span) {
        if self.bail {
            return;
        }
        match &event {
            Event::StreamStart | Event::StreamEnd | Event::Comment(..) => {}
            Event::DocumentStart(..) => {
                self.docs += 1;
                if self.docs > 1 {
                    self.bail = true;
                }
            }
            Event::DocumentEnd => {}
            // Anchors and aliases stay on the AST surface: measured
            // event-span replay never beat parse+to_dict (anchored docs
            // regressed ~+48% paying collection overhead *plus* the AST
            // fallback), and a byte-level pre-veto is impossible since
            // aliases reference anchors by id. Revisit with parser-level
            // integration, not stream-level.
            Event::Alias(_) => self.bail = true,
            Event::Scalar(value, _, anchor_id, tag) => {
                if tag.is_some() || *anchor_id != 0 {
                    self.bail = true;
                    return;
                }
                // Merge keys veto the fast path (the AST layer owns `<<`
                // semantics). Rejected regardless of position: a literal
                // "<<" scalar is vanishingly rare outside merge keys, and
                // losing the fast path there is harmless.
                if value == "<<" {
                    self.bail = true;
                    return;
                }
                self.push(event, span);
            }
            Event::MappingStart(_, anchor_id, tag) | Event::SequenceStart(_, anchor_id, tag) => {
                if tag.is_some() || anchor_id != &0 {
                    self.bail = true;
                    return;
                }
                self.push(event, span);
            }
            Event::MappingEnd | Event::SequenceEnd => self.push(event, span),
            _ => self.bail = true, // granit Event is #[non_exhaustive]
        }
    }
}

/// True when a document separator line (`---` / `...`) appears anywhere
/// after the first line: such streams may hold multiple documents and the
/// collector's late bail is too expensive to gamble on. False positives
/// (separator-looking lines inside block scalars) only lose the fast path.
fn has_later_document_separator(src: &str) -> bool {
    let mut lines = src.lines();
    lines.next();
    lines.any(|l| l == "---" || l == "..." || l.starts_with("--- ") || l.starts_with("... "))
}

/// The events of a document the direct surface can answer, or `None` when the document is
/// outside it.
///
/// `None` is the whole answer for the caller: the AST pipeline owns the diagnostic, so the
/// fast path has nothing to say about *why* a document was refused.
pub fn replicable_events(src: &str) -> Option<Vec<DirectEvent<'_>>> {
    // Anchored documents stay on the AST path; a single `&` byte scan
    // rejects them here so bail costs nanoseconds, not a partial granit
    // pass + full AST rerun (measured +59% on small anchored documents
    // without this gate). Aliases cannot exist without an anchor
    // definition, so scanning for the anchor marker covers both; the
    // known false positive (`&` inside quoted values, e.g. URLs) only
    // costs that document the fast path. Tags and merge keys are vetoed
    // precisely at the event layer.
    if src.as_bytes().contains(&b'&') {
        return None;
    }
    // Cheap structural veto for multi-document streams: bailing only when
    // the collector reaches the second DocumentStart wastes a full
    // collection pass (and the follow-up AST parse re-reads everything),
    // which measured +58% on multi-document loads. A line-start scan for
    // separators costs microseconds; a leading `---` (explicit single-doc
    // start) stays on the fast path.
    if has_later_document_separator(src) {
        return None;
    }
    let mut collector = Collector {
        events: Vec::new(),
        bail: false,
        docs: 0,
    };
    let mut parser = SaphyrParser::new_from_str(src);
    match parser.load(&mut collector, true) {
        Ok(()) if !collector.bail => Some(collector.events),
        // A parse error bails here and the AST path reruns for the authoritative diagnostic.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The veto exactly as [`replicable_events`] reaches it: the two cheap byte/line
    /// pre-vetoes, then the collector's own answer.
    ///
    /// Pinned here rather than only in `tests/test_reader_parity.py`, which mirrors these rules
    /// to floor its corpus: a mirror is only trustworthy next to the decision it mirrors. The
    /// decision lives in this crate, so its test does too — and this crate's test tier builds
    /// no Python runtime, so the surface the fast path answers on is asserted without one.
    fn would_bail(src: &str) -> bool {
        replicable_events(src).is_none()
    }

    #[test]
    fn value_only_documents_stay_on_the_direct_surface() {
        for src in [
            "a: 1\n",
            "a:\n  b: [1, 2]\n  c: {d: e}\n",
            "list:\n  - one\n  - two\n",
            "block: |\n  line\n  more\n",
            "quoted: \"an exclamation mark!\"\n",
        ] {
            assert!(!would_bail(src), "{src:?} is replicable and must not bail");
        }
    }

    #[test]
    fn everything_outside_the_replicable_surface_bails() {
        for (src, reason) in [
            ("a: &x 1\n", "anchor definition"),
            ("a: &x 1\nb: *x\n", "alias"),
            ("a: !!str 1\n", "tag on a scalar"),
            ("a: !!seq [1]\n", "tag on a collection"),
            ("a: 1\n<<: {b: 2}\n", "merge key"),
            ("a: 1\n---\nb: 2\n", "second document"),
            ("a: [1\n", "parse error"),
            // The byte scan's accepted false positive: a quoted `&` costs this document the fast path.
            ("a: \"x & y\"\n", "ampersand inside a quoted scalar"),
        ] {
            assert!(would_bail(src), "{src:?} ({reason}) must bail");
        }
    }
}
