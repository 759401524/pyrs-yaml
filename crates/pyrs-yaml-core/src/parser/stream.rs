use crate::ast::{NodeMap, ScalarStyle, Tag};
use crate::parser::yaml::anchor_name_within;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use granit_parser::{
    Event, Parser as SaphyrParser, ScalarStyle as SaphyrScalarStyle, Span, SpannedEventReceiver,
};

pub use pyrs_event::{EventSink, NoteOwner, StreamEvent, StreamEventType};

/// A note that has been read but not yet placed: it will lead the next node
/// the consumer builds.
struct PendingNote {
    /// The note's text, sanitised the way the tree sanitises it.
    text: Arc<str>,
    /// Whether the source gave the note a line of its own.
    owns_line: bool,
    /// The note's own place in the source, kept so the event carries it.
    line: usize,
    column: usize,
}

/// Where a same-line note lands, in the vocabulary's terms.
enum InlineBinding {
    /// The note trails the node the consumer built last, in its inline slot.
    Preceding,
    /// The note leads the container the consumer built last, in its leading list.
    PrecedingLeading,
    /// The note annotates a node that has not been built yet.
    Following,
}

/// Event receiver that collects `StreamEvent`s from granit-parser.
///
/// Mirrors the pattern used by `AstReceiver` in `parser::mod.rs` but
/// emits lightweight structural events instead of building an AST.
///
/// A note is reported with the two facts the tree knows separately: whether
/// the source gave it a line of its own (`owns_line`), and where the tree
/// files it (`owner`). The tree's own reader is the reference for the second
/// one, so a note that trails the node before it and a note that leads the
/// node after it are told apart even though the source reads alike.
pub struct StreamReceiver<'a> {
    /// Raw source text, needed to read an anchored node's `&name` back at its
    /// span (granit surfaces only the numeric anchor id, never the name).
    yaml_text: &'a str,
    /// Char-index to byte-offset table for non-ASCII input (`None` when ASCII,
    /// so a span char index is already a byte offset).
    char_offsets: Option<Vec<usize>>,
    events: Vec<StreamEvent<'a>>,
    /// Notes read but not yet placed: each leads the next node the consumer
    /// builds.
    ///
    /// A stack rather than a single slot, because a document may stack any
    /// number of comment lines above one key and the single slot kept only the
    /// last of them - a loss the tree does not have.
    pending_notes: Vec<PendingNote>,
    /// Whether the source can hold a note at all.
    ///
    /// Everything below - the pending stack, the node the consumer built last,
    /// the root, the document's end - is only ever read when a note arrives,
    /// and a YAML note starts with `#`, so a source without one cannot produce
    /// a note and the placement machine is dead weight for it. One pass over
    /// the source buys that answer, and it is the same laziness the tree's own
    /// reader has: `AstReceiver` touches a note only when granit hands it one.
    ///
    /// The pass is a scalar byte loop, and the two spellings of the question
    /// are the same loop: `str::contains` and `as_bytes().contains` measured
    /// 252,210,277 instructions each over `stream_medium`'s 2000 parses, the
    /// two builds agreeing to the instruction. An earlier version of this
    /// comment called the `str` form the standard library's vectorised search
    /// and put its cost at "less than a tenth" of the byte-slice form; the
    /// measurement says otherwise, and the claim is withdrawn.
    ///
    /// What the pass costs and what it buys, measured on one tree by rewriting
    /// this line four ways and reading `stream_medium` (a 298-byte document
    /// with no note) once per rewrite:
    ///
    /// - the pass over the document: 483 instructions per parse (966,000 over
    ///   2000 parses, about 1.6 per byte);
    /// - the placement machine it skips: 1,624 instructions per parse;
    /// - so the gate pays for itself by 1,141 instructions per parse, and the
    ///   price of the gate is the scan - not the difference between the two
    ///   `contains` spellings, which is zero.
    has_notes: bool,
    /// The byte end of the node the consumer built last: the candidate a
    /// same-line note binds to.
    last_node_end: Option<usize>,
    /// Whether that node already carries an inline note.
    last_node_noted: bool,
    /// Whether that node is a container the reader has not filled yet.
    last_node_open_container: bool,
    /// Whether the current document has produced its root.
    has_root: bool,
    /// Whether the current document's root is a container.
    root_is_container: bool,
    /// Whether the current document has ended.
    document_ended: bool,
    anchors: NodeMap<usize, String>,
    /// The byte this reader has seen text end at, which is the floor the anchor-name recovery may not
    /// read below.
    ///
    /// It is a single monotone number rather than the tree's per-container tally because that is all
    /// the bound needs: granit reports a node's span, so the most recent non-empty span end is behind
    /// this node's own text, and an `&` further left belongs to something already read - whichever
    /// quoting or comment reading the line suggests (quality-ledger (dr)). Empty spans are not folded
    /// in: that is how granit writes both a block container's own start and its `End` event, which sits
    /// on the token that *follows* the container (`map_end 23..23` between `k2: v2` and `next: 3`), so
    /// folding one would fence the next sibling out of its own `&` and turn a rename into a lost anchor.
    /// The tree reaches the same bound by another route: for a block container it computes the node's
    /// end as the greatest child end, and only a flow's `End` span end counts - which is what skipping
    /// empty spans reproduces.
    window_floor: usize,
    /// Maximum allowed nesting depth for mapping/sequence containers.
    max_depth: usize,
    /// Number of currently open mapping/sequence containers.
    depth: usize,
    /// Set to true when the maximum nesting depth is exceeded.
    max_depth_exceeded: bool,
}

impl<'a> StreamReceiver<'a> {
    fn new_with_options(yaml_text: &'a str, max_depth: usize) -> Self {
        let is_ascii = yaml_text.is_ascii();
        Self {
            yaml_text,
            char_offsets: (!is_ascii).then(|| super::char_to_byte_offsets(yaml_text)),
            events: Vec::new(),
            pending_notes: Vec::new(),
            has_notes: yaml_text.contains('#'),
            anchors: NodeMap::default(),
            window_floor: 0,
            max_depth,
            depth: 0,
            max_depth_exceeded: false,
            last_node_end: None,
            last_node_noted: false,
            last_node_open_container: false,
            has_root: false,
            root_is_container: false,
            document_ended: false,
        }
    }

    /// Emit one note in the slot the tree files it under.
    fn emit_note(
        &mut self,
        text: Arc<str>,
        owns_line: bool,
        owner: NoteOwner,
        line: usize,
        column: usize,
    ) {
        self.events.push(StreamEvent {
            event_type: StreamEventType::Comment {
                text,
                owns_line,
                owner,
            },
            line,
            column,
        });
    }

    /// Emit the notes waiting to lead the next node, in source order.
    fn flush_pending_notes(&mut self) {
        if self.pending_notes.is_empty() {
            return;
        }
        for note in core::mem::take(&mut self.pending_notes) {
            self.emit_note(
                note.text,
                note.owns_line,
                NoteOwner::FollowingLeading,
                note.line,
                note.column,
            );
        }
    }

    /// Emit the notes still pending when the document ends.
    ///
    /// The tree files the first of them in the root's inline slot, because that
    /// is the slot the writer read it from: the writer cannot hang an inline
    /// note on a block container's last line, so it spills the note onto its
    /// own trailing line and the reader reports it back from there. The stream
    /// reports every one of them in that slot - a consumer that rebuilds a tree
    /// keeps the first, which is the tree's own answer.
    fn flush_trailing_notes(&mut self) {
        if self.pending_notes.is_empty() {
            return;
        }
        let owner = if self.has_root {
            NoteOwner::PrecedingInline
        } else {
            // No node of its own: the note leads the null root the tree builds
            // for such a document, and the consumer builds that root next.
            NoteOwner::FollowingLeading
        };
        for note in core::mem::take(&mut self.pending_notes) {
            self.emit_note(note.text, note.owns_line, owner, note.line, note.column);
        }
    }

    /// Emit the notes still pending when the stream ends.
    ///
    /// Nothing followed them, so they lead the document's root - where the tree
    /// files them too, since a document with no node of its own becomes a null
    /// root carrying the notes it read. A note that arrives after the document
    /// has ended lands on a root the consumer has already built, so it is the
    /// root's *leading* list of a node built last.
    fn flush_final_notes(&mut self) {
        let owner = if self.has_root {
            NoteOwner::PrecedingLeading
        } else {
            NoteOwner::FollowingLeading
        };
        for note in core::mem::take(&mut self.pending_notes) {
            self.emit_note(note.text, note.owns_line, owner, note.line, note.column);
        }
    }

    /// Record the document's root: the first node the reader produced, and the
    /// node a trailing note falls back on.
    fn note_the_root(&mut self, is_container: bool) {
        if !self.has_root {
            self.has_root = true;
            self.root_is_container = is_container;
        }
    }

    /// Decide where a same-line note lands.
    ///
    /// Mirrors the tree's `attach_inline_comment`: the note trails the node the
    /// consumer built last, unless a line break separates it from that node -
    /// then it annotates a node that has not arrived yet and leads it instead.
    fn bind_inline(&mut self, comment_byte: usize) -> InlineBinding {
        // The document has ended, so nothing it could trail remains - unless the
        // finished root is a container, which really does report the note back
        // from the last line of its body (the tree's rule, crash-7918272c).
        if self.document_ended && !self.root_is_container {
            return InlineBinding::Following;
        }
        // The node the consumer built last is a container with nothing in it
        // yet: there is no child for the note to trail, so it leads the
        // container - the slot the tree files it in.
        if self.last_node_open_container {
            return InlineBinding::PrecedingLeading;
        }
        let Some(node_end) = self.last_node_end else {
            return InlineBinding::Following;
        };
        // A note on a later line than the candidate annotates a node that has
        // not been produced yet.
        if crate::parser::line_break_between(self.yaml_text, node_end, comment_byte) {
            return InlineBinding::Following;
        }
        // The candidate already carries an inline note; the tree moves this one
        // forward rather than overwriting the first, because two same-line notes
        // cannot share the one inline slot.
        if self.last_node_noted {
            return InlineBinding::Following;
        }
        self.last_node_noted = true;
        InlineBinding::Preceding
    }

    /// Handle a note granit reported: an own-line note leads the next node, and
    /// a same-line note trails the node the consumer built last.
    fn on_comment(
        &mut self,
        text: &str,
        placement: &granit_parser::Placement,
        span: Span,
        line: usize,
        column: usize,
    ) {
        let trimmed = text.trim_matches(pyrs_schema::is_yaml_blank);
        // granit drops a contentless `#`/`# ` on re-read, so a consumer
        // round-tripping through these events would drift on it; surface
        // nothing for an empty comment (mirrors the AST receiver; libFuzzer
        // `yaml_roundtrip` crash-0de6be17).
        if trimmed.is_empty() {
            return;
        }
        // The tree sanitises a note on ingest too, and the two readers have to
        // report the same text: a code point that cannot appear inside a
        // document makes the whole document unparseable (libFuzzer
        // `yaml_roundtrip` crash-2d14c6f6).
        let text: Arc<str> = if trimmed.chars().all(pyrs_schema::is_yaml_document_char) {
            Arc::from(trimmed)
        } else {
            let cleaned: String = trimmed
                .chars()
                .filter(|&c| pyrs_schema::is_yaml_document_char(c))
                .collect();
            let cleaned = cleaned.trim_matches(pyrs_schema::is_yaml_blank);
            if cleaned.is_empty() {
                return;
            }
            Arc::from(cleaned)
        };
        // The source fact: granit reports a note as `Right` exactly when the
        // line carried content before the `#`, so the placement answers "does
        // this note own its line" on its own.
        let owns_line = crate::parser::is_standalone_placement(placement);
        if owns_line {
            self.pending_notes.push(PendingNote {
                text,
                owns_line: true,
                line,
                column,
            });
            return;
        }
        if *placement != granit_parser::Placement::Right {
            // Placement is #[non_exhaustive]: other variants are ignored.
            return;
        }
        let comment_byte = self.byte_start_of(&span);
        match self.bind_inline(comment_byte) {
            InlineBinding::Preceding => {
                self.emit_note(text, false, NoteOwner::PrecedingInline, line, column);
            }
            InlineBinding::PrecedingLeading => {
                self.emit_note(text, false, NoteOwner::PrecedingLeading, line, column);
            }
            InlineBinding::Following => {
                self.pending_notes.push(PendingNote {
                    text,
                    owns_line: false,
                    line,
                    column,
                });
            }
        }
    }

    /// Byte offset of a span's start char index (identity for ASCII input).
    fn byte_start_of(&self, span: &Span) -> usize {
        match &self.char_offsets {
            None => span.start.index(),
            Some(t) => t[span.start.index()],
        }
    }

    /// Byte offset of a span's end char index, the same table read forwards.
    fn byte_end_of(&self, span: &Span) -> usize {
        match &self.char_offsets {
            None => span.end.index(),
            Some(t) => t[span.end.index()],
        }
    }
}

impl<'a> SpannedEventReceiver<'a> for StreamReceiver<'a> {
    fn on_event(&mut self, event: Event<'a>, span: Span) {
        let line = span.start.line().saturating_sub(1);
        let column = span.start.col();

        // Track nesting depth (mirrors AstReceiver: flag when a new container
        // would push the open-container count to or beyond max_depth).
        match &event {
            Event::MappingStart { .. } | Event::SequenceStart { .. } => {
                if self.depth >= self.max_depth {
                    self.max_depth_exceeded = true;
                } else {
                    self.depth += 1;
                }
            }
            Event::MappingEnd | Event::SequenceEnd => {
                self.depth = self.depth.saturating_sub(1);
            }
            _ => {}
        }

        // Handle native comments from granit-parser
        if let Event::Comment(text, placement) = &event {
            self.on_comment(text, placement, span, line, column);
            return;
        }

        let byte_start = self.byte_start_of(&span);
        let yaml = self.yaml_text;
        // The floor is read before this event's own span is folded in: it is where the text *behind*
        // this node ended, which is what a `Start` event of an anchored container needs in order to be
        // named from its parent's window, as `begin_container` does on the tree path.
        let floor = self.window_floor;
        let Some(stream_event) =
            event_to_stream_event(event, span, &mut self.anchors, &mut |_id| {
                anchor_name_within(yaml, byte_start, Some(floor))
            })
        else {
            return; // non_exhaustive wildcard
        };
        if span.start.index() != span.end.index() {
            let end = self.byte_end_of(&span);
            if end > self.window_floor {
                self.window_floor = end;
            }
        }

        // The placement machine runs only for a source that can hold a note; a
        // source without one never reaches it (see `has_notes`).
        if self.has_notes {
            // A note read earlier leads the node the consumer builds next, and
            // the tree files it in that node's leading list - so the two agree.
            // An alias is not such a node: the tree's alias carries no metadata
            // slot, so the note passes over it and leads whatever comes after.
            //
            // One match answers both questions - which node leads, and what the
            // node the consumer builds last now is - because the two used to
            // cost two dispatches per event.
            match &stream_event.event_type {
                StreamEventType::Scalar { .. } => {
                    self.flush_pending_notes();
                    self.last_node_end = Some(self.byte_end_of(&span));
                    self.last_node_noted = false;
                    self.last_node_open_container = false;
                    self.note_the_root(false);
                }
                StreamEventType::MappingStart { .. } | StreamEventType::SequenceStart { .. } => {
                    self.flush_pending_notes();
                    self.last_node_noted = false;
                    self.last_node_open_container = true;
                    self.note_the_root(true);
                }
                StreamEventType::Alias { .. } => {
                    self.last_node_end = Some(self.byte_end_of(&span));
                    self.last_node_noted = false;
                    self.last_node_open_container = false;
                    self.note_the_root(false);
                }
                StreamEventType::MappingEnd | StreamEventType::SequenceEnd => {
                    self.last_node_noted = false;
                    self.last_node_open_container = false;
                    // A flow container's own end carries the boundary. A block
                    // container's end event sits on the token that follows it, so
                    // the last child's end stands - which is the tree's own answer
                    // for a block collection.
                    if span.start.index() != span.end.index() {
                        self.last_node_end = Some(self.byte_end_of(&span));
                    }
                }
                StreamEventType::DocumentStart => {
                    self.has_root = false;
                    self.root_is_container = false;
                    self.document_ended = false;
                    self.last_node_end = None;
                    self.last_node_noted = false;
                    self.last_node_open_container = false;
                }
                StreamEventType::DocumentEnd => {
                    // The document's trailing note: the tree files the first pending
                    // note in the root's inline slot, so the stream reports that
                    // slot - after the root was built and before the document ends.
                    self.flush_trailing_notes();
                    self.document_ended = true;
                }
                _ => {}
            }
        }

        self.events.push(stream_event);
    }
}

/// Convert a granit-parser Tag to our Tag format.
fn convert_tag(tag: Option<&granit_parser::Tag>) -> Option<Tag> {
    tag.map(|t| super::convert_tag(t).clone())
}

/// Pure function: granit `Event` + `Span` → `StreamEvent`.
///
/// Contains no comment dependencies (those stay in `StreamReceiver::on_event`);
/// anchor name injected via `resolve_anchor` callback. Returns `None` for
/// non_exhaustive wildcard.
pub fn event_to_stream_event<'a, F>(
    event: Event<'a>,
    span: Span,
    anchor_map: &mut NodeMap<usize, String>,
    resolve_anchor: &mut F,
) -> Option<StreamEvent<'a>>
where
    F: FnMut(usize) -> Option<String>,
{
    let line = span.start.line().saturating_sub(1);
    let column = span.start.col();

    let event_type = match event {
        // granit-parser doesn't have Event::Nothing, use wildcard at end of match
        Event::StreamStart => StreamEventType::StreamStart,
        Event::StreamEnd => StreamEventType::StreamEnd,
        Event::DocumentStart(_, _) => StreamEventType::DocumentStart,
        Event::DocumentEnd => StreamEventType::DocumentEnd,
        Event::Scalar(value, style, anchor_id, tag) => {
            let scalar_style = match style {
                SaphyrScalarStyle::Plain => ScalarStyle::Plain,
                SaphyrScalarStyle::SingleQuoted => ScalarStyle::SingleQuoted,
                SaphyrScalarStyle::DoubleQuoted => ScalarStyle::DoubleQuoted,
                SaphyrScalarStyle::Literal => ScalarStyle::Literal,
                SaphyrScalarStyle::Folded => ScalarStyle::Folded,
            };
            // granit already delivers the decoded double-quoted value; a
            // second unescape here would decode `"\\n"`-style raw pairs wrong
            // (see create_scalar in the tree parser). The `Cow` moves in rather
            // than being copied out: a plain, quoted-without-escapes or block
            // scalar is a slice of the source, and an escaped double-quoted one
            // already owns the string granit decoded — either way this line used
            // to allocate a second copy of every scalar in the document.
            let anchor = resolve_anchor_name(anchor_id, anchor_map, resolve_anchor);
            let tag_obj = convert_tag(tag.as_deref());
            StreamEventType::Scalar {
                value,
                style: scalar_style,
                anchor,
                tag: tag_obj,
            }
        }
        Event::MappingStart(_, anchor_id, tag) => {
            let anchor = resolve_anchor_name(anchor_id, anchor_map, resolve_anchor);
            let tag_obj = convert_tag(tag.as_deref());
            StreamEventType::MappingStart {
                anchor,
                tag: tag_obj,
            }
        }
        Event::MappingEnd => StreamEventType::MappingEnd,
        Event::SequenceStart(_, anchor_id, tag) => {
            let anchor = resolve_anchor_name(anchor_id, anchor_map, resolve_anchor);
            let tag_obj = convert_tag(tag.as_deref());
            StreamEventType::SequenceStart {
                anchor,
                tag: tag_obj,
            }
        }
        Event::SequenceEnd => StreamEventType::SequenceEnd,
        Event::Alias(anchor_id) => {
            let name = anchor_map
                .get(&anchor_id)
                .cloned()
                .unwrap_or_else(|| format!("alias_{}", anchor_id));
            StreamEventType::Alias { name }
        }
        _ => return None, // granit-parser is non_exhaustive
    };

    Some(StreamEvent {
        event_type,
        line,
        column,
    })
}

/// Resolve and register an anchor name for a non-zero anchor id.
///
/// A `None` from `resolve_anchor` means there is no anchor to name (the source ran out), so
/// nothing is registered — the same reading the string path gives for `next_anchor_name()`.
fn resolve_anchor_name<F>(
    anchor_id: usize,
    anchor_map: &mut NodeMap<usize, String>,
    resolve_anchor: &mut F,
) -> Option<String>
where
    F: FnMut(usize) -> Option<String>,
{
    if anchor_id == 0 {
        return None;
    }
    match resolve_anchor(anchor_id) {
        Some(name) => {
            anchor_map.insert(anchor_id, name.clone());
            Some(name)
        }
        None => None,
    }
}

/// Parse a YAML string into a stream of `StreamEvent`s.
///
/// Consumes the YAML string and returns a vector of streaming events
/// that represent every structural element in the document.
///
/// # Arguments
/// * `yaml` - The YAML content string.
///
/// # Returns
/// Success: a `Vec<StreamEvent>` borrowing `yaml`, representing the complete event stream.
/// Failure: a `ParseError` with line/column information.
///
/// # Examples
/// ```rust
/// use pyrs_yaml_core::parser::stream::parse_stream;
/// let events = parse_stream("key: value").unwrap();
/// assert!(!events.is_empty());
/// ```
pub fn parse_stream(yaml: &str) -> Result<Vec<StreamEvent<'_>>, super::ParseError> {
    parse_stream_with_options(yaml, 1000)
}

/// Parse a YAML string into a stream of events with a maximum nesting depth.
///
/// # Arguments
/// * `yaml` - The YAML content string.
/// * `max_depth` - Maximum allowed nesting depth for mapping/sequence containers.
///
/// # Returns
/// Success: a `Vec<StreamEvent>` borrowing `yaml`, representing the complete event stream.
/// Failure: a `ParseError` with line/column information.
pub fn parse_stream_with_options(
    yaml: &str,
    max_depth: usize,
) -> Result<Vec<StreamEvent<'_>>, super::ParseError> {
    if pyrs_schema::is_yaml_blank_only(yaml) {
        return Ok(Vec::new());
    }

    let mut receiver = StreamReceiver::new_with_options(yaml, max_depth);
    let mut parser = SaphyrParser::new_from_str(yaml);

    parser
        .load(&mut receiver, true)
        .map_err(|e| super::ParseError::Syntax {
            message: format!("YAML parse error: {}", e),
            line: 0,
            col: 0,
        })?;

    if receiver.max_depth_exceeded {
        return Err(super::ParseError::MaxDepthExceeded(super::DepthError(
            max_depth,
        )));
    }

    // A note nothing followed leads the document's root, which is where the
    // tree files it too.
    receiver.flush_final_notes();

    Ok(receiver.events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_stream_start_and_end() {
        let events = parse_stream("hello").unwrap();
        assert!(!events.is_empty());
        assert!(matches!(
            &events[0].event_type,
            StreamEventType::StreamStart
        ));
        assert!(matches!(
            events.last().unwrap().event_type,
            StreamEventType::StreamEnd
        ));
    }

    /// What the `stream_medium` scenario is priced against: a scalar that needs no decode carries
    /// the source's own bytes, and `into_owned` is the step that crosses a boundary which cannot
    /// hold the borrow. Reintroducing a per-scalar copy here fails this test by name.
    #[test]
    fn a_plain_scalar_borrows_the_source_and_a_decoded_one_owns_its_text() {
        let src = "plain: 1\nquoted: \"a\\nb\"\n";
        let source_range = src.as_ptr() as usize..src.as_ptr() as usize + src.len();
        let events = parse_stream(src).unwrap();

        let scalar_of = |wanted: &str| {
            events.iter().find_map(|event| match &event.event_type {
                StreamEventType::Scalar { value, .. } if &**value == wanted => Some(value),
                _ => None,
            })
        };

        let plain = scalar_of("plain").expect("the plain key is in the stream");
        assert!(
            matches!(plain, std::borrow::Cow::Borrowed(_)),
            "a plain scalar must not be copied out of the source"
        );
        assert!(
            source_range.contains(&(plain.as_ptr() as usize)),
            "the borrowed scalar must point into the source it was parsed from"
        );

        // granit decodes an escaped double-quoted value, so that one owns its text already.
        assert!(matches!(
            scalar_of("a\nb").expect("the escaped value is in the stream"),
            std::borrow::Cow::Owned(_)
        ));

        // Detaching keeps the text: what the `#[pyclass]` boundary needs and nothing else pays.
        let owned: Vec<_> = events.into_iter().map(StreamEvent::into_owned).collect();
        let detached = owned
            .iter()
            .find_map(|event| match &event.event_type {
                StreamEventType::Scalar { value, .. } if &**value == "plain" => Some(value),
                _ => None,
            })
            .expect("the key survives the detach");
        assert!(matches!(detached, std::borrow::Cow::Owned(_)));
    }

    #[test]
    fn test_stream_simple_scalar() {
        let events = parse_stream("hello").unwrap();
        let scalar = events
            .iter()
            .find(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }));
        assert!(scalar.is_some());
        if let StreamEventType::Scalar {
            value,
            style,
            anchor,
            tag,
        } = &scalar.unwrap().event_type
        {
            assert_eq!(value, "hello");
            assert_eq!(style, &ScalarStyle::Plain);
            assert!(anchor.is_none());
            assert!(tag.is_none());
        }
    }

    #[test]
    fn test_stream_mapping() {
        let events = parse_stream("key: value").unwrap();
        let mapping_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::MappingStart { .. }))
            .collect();
        assert_eq!(mapping_starts.len(), 1);
        let mapping_ends: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::MappingEnd))
            .collect();
        assert_eq!(mapping_ends.len(), 1);
    }

    #[test]
    fn test_stream_sequence() {
        let events = parse_stream("- item1\n- item2").unwrap();
        let seq_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::SequenceStart { .. }))
            .collect();
        assert_eq!(seq_starts.len(), 1);
        let seq_ends: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::SequenceEnd))
            .collect();
        assert_eq!(seq_ends.len(), 1);
    }

    #[test]
    fn test_stream_scalar_with_anchor() {
        let events = parse_stream("key: &anchor value").unwrap();
        let scalars: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }))
            .collect();
        // The value scalar (with anchor) is the second scalar
        assert!(scalars.len() >= 2);
        if let StreamEventType::Scalar { anchor, .. } = &scalars[1].event_type {
            assert!(anchor.is_some());
            assert_eq!(anchor.as_deref(), Some("anchor"));
        }
    }

    #[test]
    fn test_stream_scalar_with_tag() {
        let events = parse_stream("key: !!str value").unwrap();
        let scalars: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }))
            .collect();
        // The value scalar (with tag) is the second scalar
        assert!(scalars.len() >= 2);
        if let StreamEventType::Scalar { tag, .. } = &scalars[1].event_type {
            assert!(tag.is_some());
            assert_eq!(tag.as_ref().unwrap().suffix, "str");
        }
    }

    #[test]
    fn test_stream_alias() {
        let events = parse_stream("a: &x 1\nb: *x").unwrap();
        let alias_events: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Alias { .. }))
            .collect();
        assert_eq!(alias_events.len(), 1);
        if let StreamEventType::Alias { name } = &alias_events[0].event_type {
            assert_eq!(name, "x");
        }
    }

    #[test]
    fn test_stream_standalone_comment() {
        let events = parse_stream("# standalone comment\nkey: value").unwrap();
        let comment_events: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Comment { .. }))
            .collect();
        assert!(!comment_events.is_empty());
        if let StreamEventType::Comment {
            text,
            owns_line,
            owner,
        } = &comment_events[0].event_type
        {
            assert_eq!(text.as_ref(), "standalone comment");
            assert!(*owns_line, "the note had a line of its own");
            assert_eq!(*owner, NoteOwner::FollowingLeading);
        }
    }

    #[test]
    fn test_stream_inline_comment() {
        let events = parse_stream("key: value  # inline comment").unwrap();
        let comment_events: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Comment { .. }))
            .collect();
        assert!(!comment_events.is_empty());
        if let StreamEventType::Comment {
            text,
            owns_line,
            owner,
        } = &comment_events[0].event_type
        {
            assert_eq!(text.as_ref(), "inline comment");
            assert!(!*owns_line, "the note shared its line with the value");
            assert_eq!(*owner, NoteOwner::PrecedingInline);
        }
    }

    #[test]
    fn test_stream_comment_before_mapping() {
        // Comment before a mapping should be emitted before MappingStart
        let events = parse_stream("# mapping comment\nkey: value").unwrap();
        let comment_pos = events
            .iter()
            .position(|e| matches!(&e.event_type, StreamEventType::Comment { .. }))
            .expect("comment event should exist");
        let mapping_pos = events
            .iter()
            .position(|e| matches!(&e.event_type, StreamEventType::MappingStart { .. }))
            .expect("mapping_start event should exist");
        assert!(
            comment_pos < mapping_pos,
            "comment should come before mapping_start"
        );
        if let StreamEventType::Comment {
            text,
            owns_line,
            owner,
        } = &events[comment_pos].event_type
        {
            assert_eq!(text.as_ref(), "mapping comment");
            assert!(*owns_line);
            assert_eq!(*owner, NoteOwner::FollowingLeading);
        }
    }

    #[test]
    fn test_stream_empty_yaml() {
        let events = parse_stream("").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_stream_multiple_documents() {
        let events = parse_stream("---\nkey1: val1\n---\nkey2: val2").unwrap();
        let doc_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::DocumentStart))
            .collect();
        assert_eq!(doc_starts.len(), 2);
    }

    #[test]
    fn test_stream_single_quoted_scalar() {
        let events = parse_stream("key: 'value'").unwrap();
        let scalars: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }))
            .collect();
        // The value scalar is the second scalar
        assert!(scalars.len() >= 2);
        if let StreamEventType::Scalar { style, .. } = &scalars[1].event_type {
            assert_eq!(style, &ScalarStyle::SingleQuoted);
        }
    }

    #[test]
    fn test_stream_double_quoted_scalar() {
        let events = parse_stream(r#"key: "value""#).unwrap();
        let scalars: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }))
            .collect();
        assert!(scalars.len() >= 2);
        if let StreamEventType::Scalar { value, style, .. } = &scalars[1].event_type {
            assert_eq!(style, &ScalarStyle::DoubleQuoted);
            assert_eq!(value, "value");
        }
    }

    #[test]
    fn test_stream_literal_scalar() {
        let events = parse_stream("key: |\n  line1\n  line2").unwrap();
        let scalars: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }))
            .collect();
        assert!(scalars.len() >= 2);
        if let StreamEventType::Scalar { style, .. } = &scalars[1].event_type {
            assert_eq!(style, &ScalarStyle::Literal);
        }
    }

    #[test]
    fn test_stream_folded_scalar() {
        let events = parse_stream("key: >\n  line1\n  line2").unwrap();
        let scalars: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Scalar { .. }))
            .collect();
        assert!(scalars.len() >= 2);
        if let StreamEventType::Scalar { style, .. } = &scalars[1].event_type {
            assert_eq!(style, &ScalarStyle::Folded);
        }
    }

    #[test]
    fn test_stream_nested_mapping() {
        let events = parse_stream("a:\n  b: 1\n  c: 2").unwrap();
        let mapping_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::MappingStart { .. }))
            .collect();
        assert_eq!(mapping_starts.len(), 2);
    }

    #[test]
    fn test_stream_nested_sequence() {
        let events = parse_stream("- - a\n  - b\n- c").unwrap();
        let seq_starts: Vec<&StreamEvent> = events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::SequenceStart { .. }))
            .collect();
        assert_eq!(seq_starts.len(), 2);
    }

    #[test]
    fn test_stream_line_column() {
        let events = parse_stream("key: value").unwrap();
        for event in &events {
            assert!(event.line < 100);
        }
    }

    #[test]
    fn spike_new_from_iter_matches_str() {
        // 项 1：new_from_iter 与 new_from_str 事件序列一致（值/样式/line/col 逐项相等）
        let mut p1 = SaphyrParser::new_from_str("a: 1\n");
        let mut p2 = SaphyrParser::new_from_iter("a: 1\n".chars());
        loop {
            match (p1.next_event(), p2.next_event()) {
                (None, None) => break,
                (Some(Ok(e1)), Some(Ok(e2))) => assert_eq!(e1, e2),
                _ => panic!("spike 1: 序列不一致 (streams diverged)"),
            }
        }
    }

    #[test]
    fn spike_alias_event_carries_only_id() {
        // 项 1 附加：Event::Alias(usize) 只带 id（设计 A'''' 验证）
        let mut p = SaphyrParser::new_from_iter("a: &x 1\nb: *x\n".chars());
        let mut alias_ids = Vec::new();
        while let Some(Ok((event, _))) = p.next_event() {
            if let Event::Alias(id) = event {
                alias_ids.push(id);
            }
        }
        assert_eq!(alias_ids.len(), 1);
        assert_ne!(alias_ids[0], 0);
    }

    #[test]
    fn spike_eof_returns_none() {
        // 项 2：EOF 后 next_event() → None（stream_end_emitted flag）
        let mut p = SaphyrParser::new_from_iter("a: 1\n".chars());
        let mut count = 0;
        while p.next_event().is_some() {
            count += 1;
        }
        assert!(count > 2);
        assert!(p.next_event().is_none());
        assert!(p.next_event().is_none());
    }

    #[test]
    fn spike_multidoc_sequence() {
        // 项 2a：多文档 `--- a\n--- b\n` 事件序列 = StreamStart→DocStart→…→DocEnd→…→StreamEnd
        let mut p = SaphyrParser::new_from_iter("--- a\n--- b\n".chars());
        let mut types = Vec::new();
        while let Some(Ok((event, _))) = p.next_event() {
            types.push(match event {
                Event::StreamStart => "SS",
                Event::StreamEnd => "SE",
                Event::DocumentStart(..) => "DS",
                Event::DocumentEnd => "DE",
                Event::Scalar(..) => "SC",
                _ => "?",
            });
        }
        assert_eq!(types, ["SS", "DS", "SC", "DE", "DS", "SC", "DE", "SE"]);
    }

    #[test]
    fn spike_empty_input_yields_stream_start_end() {
        // 项 2b：空输入 → [StreamStart, StreamEnd]（与字符串 parse_stream 的 [] 不同，架构性差异）
        let mut p = SaphyrParser::new_from_iter("".chars());
        let mut types = Vec::new();
        while let Some(Ok((event, _))) = p.next_event() {
            types.push(match event {
                Event::StreamStart => "SS",
                Event::StreamEnd => "SE",
                other => panic!("unexpected: {:?}", other),
            });
        }
        assert_eq!(types, ["SS", "SE"]);
    }

    #[test]
    fn spike_crlf_line_col() {
        // 项 4：CRLF 输入 line/col 正确（skip_linebreak → 单次 line 增量，scanner.rs:619-625）。
        // 实测结论（spike）：key 与 value 都是 scalar 事件——key 在 col 0，value 在 col 3；
        // line 1-indexed；每条 CRLF 使 line 仅 +1。
        let mut p = SaphyrParser::new_from_iter("a: 1\r\nb: 2\r\n".chars());
        let mut cols = Vec::new();
        while let Some(Ok((event, span))) = p.next_event() {
            if matches!(event, Event::Scalar(..)) {
                cols.push((span.start.line(), span.start.col()));
            }
        }
        assert_eq!(cols, [(1, 0), (1, 3), (2, 0), (2, 3)]);
    }

    fn mk_span(line: usize, col: usize) -> Span {
        Span::new(
            granit_parser::Marker::new(0, line, col),
            granit_parser::Marker::new(0, line, col),
        )
    }

    #[test]
    fn event_to_stream_event_double_quoted_is_pass_through() {
        // granit delivers double-quoted scalars already decoded; the receiver
        // must NOT unescape a second time (that decode turned raw-pair text
        // like `\n` into LF — the libFuzzer `yaml_roundtrip` double-decode
        // bug). Both shapes therefore flow through byte-identical.
        let mut anchor_map = NodeMap::default();
        let decoded = Event::Scalar("a\n\t".into(), SaphyrScalarStyle::DoubleQuoted, 0, None);
        let ev =
            super::event_to_stream_event(decoded, mk_span(1, 0), &mut anchor_map, &mut |_| None)
                .expect("scalar maps to Some");
        match ev.event_type {
            super::StreamEventType::Scalar {
                value,
                style,
                anchor,
                tag,
            } => {
                assert_eq!(value, "a\n\t");
                assert_eq!(style, ScalarStyle::DoubleQuoted);
                assert_eq!(anchor, None);
                assert_eq!(tag, None);
            }
            other => panic!("wrong variant: {:?}", other),
        }
        let mut anchor_map = NodeMap::default();
        let raw_pair = Event::Scalar("a\\n".into(), SaphyrScalarStyle::DoubleQuoted, 0, None);
        let ev =
            super::event_to_stream_event(raw_pair, mk_span(1, 0), &mut anchor_map, &mut |_| None)
                .expect("scalar maps to Some");
        match ev.event_type {
            super::StreamEventType::Scalar { value, .. } => {
                assert_eq!(value, "a\\n", "second decode must not collapse raw pairs");
            }
            other => panic!("wrong variant: {:?}", other),
        }
    }

    #[test]
    fn event_to_stream_event_plain_scalar_passthrough() {
        let mut anchor_map = NodeMap::default();
        let event = Event::Scalar("hello".into(), SaphyrScalarStyle::Plain, 0, None);
        let ev = super::event_to_stream_event(event, mk_span(2, 4), &mut anchor_map, &mut |_| None)
            .unwrap();
        match ev.event_type {
            super::StreamEventType::Scalar { value, style, .. } => {
                assert_eq!(value, "hello");
                assert_eq!(style, ScalarStyle::Plain);
            }
            other => panic!("wrong variant: {:?}", other),
        }
        assert_eq!((ev.line, ev.column), (1, 4)); // line = span.line - 1
    }

    #[test]
    fn event_to_stream_event_anchor_resolves_and_registers() {
        let mut anchor_map = NodeMap::default();
        let event = Event::Scalar("v".into(), SaphyrScalarStyle::Plain, 7, None);
        let ev = super::event_to_stream_event(event, mk_span(1, 0), &mut anchor_map, &mut |id| {
            Some(format!("anchor_{}", id))
        })
        .unwrap();
        match ev.event_type {
            super::StreamEventType::Scalar {
                anchor: Some(name), ..
            } => {
                assert_eq!(name, "anchor_7");
                assert_eq!(anchor_map.get(&7), Some(&"anchor_7".to_string()));
            }
            other => panic!("wrong variant: {:?}", other),
        }
    }

    #[test]
    fn event_to_stream_event_alias_lookup_and_fallback() {
        let mut anchor_map = NodeMap::default();
        let ev = super::event_to_stream_event(
            Event::Alias(3),
            mk_span(1, 0),
            &mut anchor_map,
            &mut |_| None,
        )
        .unwrap();
        assert!(
            matches!(ev.event_type, super::StreamEventType::Alias { name } if name == "alias_3")
        );

        let mut anchor_map = NodeMap::default();
        anchor_map.insert(3, "real".to_string());
        let ev = super::event_to_stream_event(
            Event::Alias(3),
            mk_span(1, 0),
            &mut anchor_map,
            &mut |_| None,
        )
        .unwrap();
        assert!(matches!(ev.event_type, super::StreamEventType::Alias { name } if name == "real"));
    }

    #[test]
    fn event_to_stream_event_comment_is_none() {
        let mut anchor_map = NodeMap::default();
        // granit-parser emits Comment events; event_to_stream_event returns None for them
        assert!(
            super::event_to_stream_event(
                Event::Comment(Default::default(), Default::default()),
                mk_span(1, 0),
                &mut anchor_map,
                &mut |_| None
            )
            .is_none()
        );
    }

    #[test]
    fn test_stream_max_depth_exceeded() {
        let result = parse_stream_with_options("{a: {a: {a: {a: 1}}}}", 2);
        assert!(matches!(
            result,
            Err(crate::parser::ParseError::MaxDepthExceeded(
                crate::parser::DepthError(2)
            ))
        ));
    }

    #[test]
    fn test_stream_max_depth_ok() {
        let events = parse_stream_with_options("{a: {a: {a: {a: 1}}}}", 5).unwrap();
        assert!(!events.is_empty());
    }

    /// The (dr) skew, at the reader that had no window: on this document the events named the flow
    /// value after the `&` written *inside* the preceding quoted key, because the recovery read left
    /// across the whole line and that line's quote state was skewed by the plain key `$,"` before it.
    /// Each node is now named only from its own window, so the events say what granit attached.
    #[test]
    fn an_event_names_its_anchor_from_its_own_window() {
        let s = r#"$,": {&k "<-&w #q": &v "x"}"#;
        let events = parse_stream(s).unwrap();
        let anchor_of = |wanted: &str| {
            events.iter().find_map(|event| match &event.event_type {
                StreamEventType::Scalar { value, anchor, .. } if &**value == wanted => {
                    Some(anchor.clone())
                }
                _ => None,
            })
        };
        assert_eq!(anchor_of("x"), Some(Some("v".to_string())));
        assert_eq!(anchor_of("<-&w #q"), Some(Some("k".to_string())));
    }

    /// An anchored node whose window the tally must not fence it out of: the anchor is written to
    /// the *left* of the span granit reports, so a container's end may only be taken from a flow's
    /// own bracket - a block `End` event sits on the token that follows, and fencing there would
    /// drop the next sibling's name. Every anchor here survives, on both lines.
    #[test]
    fn a_block_containers_end_never_fences_the_next_anchors() {
        let s = "top:\n  &a1 k1: &v1 v1\n  &a2 k2: &v2 v2\n&nx next: &nv 3\n";
        let events = parse_stream(s).unwrap();
        let named: Vec<Option<String>> = events
            .iter()
            .filter_map(|event| match &event.event_type {
                StreamEventType::Scalar { anchor, .. } => Some(anchor.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            named,
            vec![
                None,
                Some("a1".to_string()),
                Some("v1".to_string()),
                Some("a2".to_string()),
                Some("v2".to_string()),
                Some("nx".to_string()),
                Some("nv".to_string()),
            ]
        );
    }

    /// The invariant the two readers must keep: on one text they name the same nodes with the same
    /// names, in the same order. A table rather than a generated tree because each shape is the one
    /// the recovery has to survive and says which - and because a reader that quietly disagrees with
    /// the other is exactly what no single-reader round-trip can see.
    #[test]
    fn the_tree_and_the_events_name_the_same_anchors() {
        use crate::ast::CustomNode;
        use crate::parser::yaml::Schema;

        fn collect(node: &CustomNode, out: &mut Vec<String>) {
            if let Some(name) = node.anchor() {
                out.push(name.to_string());
            }
            match node {
                CustomNode::Mapping { pairs, .. } => {
                    for (key, value) in pairs {
                        collect(key, out);
                        collect(value, out);
                    }
                }
                CustomNode::Sequence { items, .. } => {
                    for item in items {
                        collect(item, out);
                    }
                }
                _ => {}
            }
        }

        for doc in [
            r#"$,": {&k "<-&w #q": &v "x"}"#,
            "\"q k\": &qv 1\nx: &y 2 # &z\n",
            "\"#h\": &ha 1\n&hb k: &hc 2\n",
            "flow: [&fa 1, \"#&,\": &fb 2]\n",
            "&root key: 1\nplain: &p 2\n",
            "- &a1 one\n- &a2 two\n",
            "top: &seq\n  - &i 1\n",
            "a: &map {p: &q 1, r: &s 2}\n",
            "&x m: &y\n  deep: &z 1\n",
            "k: &n\n",
            "l: &b |\n  body\n",
            "m: &outer {in: &inner [1, 2]}\n",
        ] {
            let tree = crate::parser::parse(doc, Schema::Core).expect("the table parses");
            let mut from_tree = Vec::new();
            collect(&tree, &mut from_tree);
            let from_events: Vec<String> = parse_stream(doc)
                .expect("the same text streams")
                .iter()
                .filter_map(|event| match &event.event_type {
                    StreamEventType::Scalar { anchor, .. }
                    | StreamEventType::MappingStart { anchor, .. }
                    | StreamEventType::SequenceStart { anchor, .. } => anchor.clone(),
                    _ => None,
                })
                .collect();
            assert_eq!(from_tree, from_events, "the readers disagree on {doc:?}");
        }
    }
}
