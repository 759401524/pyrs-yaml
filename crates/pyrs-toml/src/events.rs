//! The tree-free event stream: one document described as events instead of a node model.
//!
//! This is the second producer of the shared vocabulary after `pyrs-json`, and it is deliberately
//! *not* built the way the first one was. JSON's producer is a walk of its own over the scanners,
//! because a JSON document is one nested value and a second walk re-derives nothing there. TOML is
//! not shaped like that: which tables a header may open, which a dotted key closed, and where an
//! array-of-tables gains a tail are a policy this engine already authors once
//! (`parser::register_header`, `parser::register_kv`). A second scanning pass would author it twice,
//! and that is the divergence `docs/dev/event-stream.md` refuses to accept on a guess.
//!
//! So the events come from the same intermediate representation the AST projection reads: the
//! scanner runs once, one verdict is reached, and two readers describe it. Accept/reject parity
//! between `from_toml` and `parse_stream` is therefore a property of the architecture rather than a
//! claim a test suite has to chase.
//!
//! ## What a TOML stream reports that the tree cannot
//!
//! Every event carries the offset of the construct it describes, converted to the same 0-indexed
//! line and character column the engine's own diagnostics use. Two of those reports have no home in
//! the AST:
//!
//! - **A close event carries the offset its construct began at.** TOML records where a table or an
//!   array opened; where it ended is decided by whatever header follows it, which the IR does not
//!   keep. So `MappingEnd` and `SequenceEnd` repeat their construct's opening offset, and the event
//!   *order* - not the offsets - is what a consumer walks. A stream with monotone positions would
//!   have to invent an end offset the document never wrote.
//! - **A header names its table inside the brackets that open it**, so a table's key event sits at
//!   a larger offset than the `MappingStart` that follows it in structural order. YAML and JSON have
//!   no such inversion; TOML's syntax does.
//!
//! ## What it cannot report, and why each answer is the vocabulary's, not TOML's
//!
//! - `tag` turns out not to be YAML's private field. A date-time arrives as `!!timestamp` and a bare
//!   time as `!time`, a binary integer as `!!int` - exactly what the AST carries - so the field means
//!   "the type the document states", and a second format fills it without anything being renamed.
//! - `anchor` stays empty and `Alias` is never emitted, the same answer JSON gave: those are YAML's
//!   grammar.
//! - A literal string (`'a\nb'`) and a basic one (`"a\nb"`) both arrive `DoubleQuoted`, because the
//!   IR keeps only whether the form was multi-line. Which quote was typed is not stored, so it is
//!   not something this stream may claim - unlike `pyrs-json`, whose walk still sees the byte.
//! - Digit groups written with `_`, and an explicit leading `+`, are reported in the canonical form
//!   the IR keeps rather than the document's bytes, which is the stripping the writer already
//!   depends on.
//! - A note that meets no key or header has no slot in the AST (the writer emits no such text, so
//!   the hub discards it), yet the document wrote it: the stream reports it.

use crate::parser::{TomlDialect, parse_ir};
use crate::{CowTable, Note, TomlTable, TomlValue, scalar_projection, toml_key_style};
use alloc::borrow::Cow;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::ControlFlow;
use pyrs_ast::error::ParseError;
use pyrs_event::{EventSink, LineTable, NoteOwner, StreamEvent, StreamEventType};

/// Describe a TOML document as the events it is made of, in the default dialect.
pub fn parse_stream(src: &str) -> Result<Vec<StreamEvent<'_>>, ParseError> {
    parse_stream_with_dialect(src, TomlDialect::default())
}

/// Describe a TOML document as events, with the grammar pinned to `dialect`.
///
/// The document is scanned exactly as [`crate::from_toml_with_options`] scans it: both routes reach
/// the same verdict with the same `ParseError`, and a document that parses yields the events of the
/// very tree the other route builds.
pub fn parse_stream_with_dialect(
    src: &str,
    dialect: TomlDialect,
) -> Result<Vec<StreamEvent<'_>>, ParseError> {
    let mut events = Vec::new();
    parse_stream_with_dialect_to(src, dialect, &mut events)?;
    Ok(events)
}

/// Describe a TOML document as events, handing each one to `sink`, in the default dialect.
pub fn parse_stream_to<'a, S: EventSink<'a>>(src: &'a str, sink: &mut S) -> Result<(), ParseError> {
    parse_stream_with_dialect_to(src, TomlDialect::default(), sink)
}

/// Describe a TOML document as events, handing each one to `sink`, with the grammar pinned to
/// `dialect`.
///
/// The scan is *not* incremental, and that is the one place this producer differs from JSON's.
/// Which tables a header may open and where an array-of-tables gains a tail are a policy the
/// engine authors once while it builds the intermediate representation; the events are a reading
/// of that representation, so it is complete before the first event exists. `Break` therefore stops
/// the emission, not the scan: a document that will be refused is refused before any event reaches
/// the sink, and `Err` is reported whatever the sink answered.
///
/// A sink that consumes everything sees exactly the events the collecting entry point returns.
pub fn parse_stream_with_dialect_to<'a, S: EventSink<'a>>(
    src: &'a str,
    dialect: TomlDialect,
    sink: &mut S,
) -> Result<(), ParseError> {
    let (root, leftover) = parse_ir(src, dialect)?;
    let mut out = Events::new(src, sink);
    out.at(0, StreamEventType::StreamStart);
    out.at(0, StreamEventType::DocumentStart);
    // The root mapping is opened by no header: its construct is the document itself.
    out.at(
        0,
        StreamEventType::MappingStart {
            anchor: None,
            tag: None,
        },
    );
    for (name, entry) in root.entries.iter() {
        if out.stopped {
            return Ok(());
        }
        out.entry(name, entry);
    }
    out.at(0, StreamEventType::MappingEnd);
    for note in leftover.iter() {
        if out.stopped {
            return Ok(());
        }
        // A note no key or header claimed has no slot in a document that has keys of its own; the
        // tree drops it and the stream keeps it. The node built last is the root, and the root's
        // leading list is where the tree puts it when the document is nothing but that note.
        out.note(note, NoteOwner::PrecedingLeading);
    }
    if out.stopped {
        return Ok(());
    }
    let end = src.len();
    out.at(end, StreamEventType::DocumentEnd);
    out.at(end, StreamEventType::StreamEnd);
    Ok(())
}

/// The writer for one document's events.
struct Events<'a, 's, S: ?Sized> {
    text: &'a str,
    lines: LineTable<'a>,
    sink: &'s mut S,
    /// Set when the sink answered `Break`: every later emit is a no-op. The IR is already built
    /// by then, so this stops the reading rather than the scan.
    stopped: bool,
}

impl<'a, 's, S: EventSink<'a> + ?Sized> Events<'a, 's, S> {
    fn new(text: &'a str, sink: &'s mut S) -> Self {
        Self {
            text,
            lines: LineTable::new(text),
            sink,
            stopped: false,
        }
    }

    fn at(&mut self, pos: usize, event_type: StreamEventType<'a>) {
        if self.stopped {
            return;
        }
        let (line, column) = self.lines.line_col(pos);
        if let ControlFlow::Break(()) = self.sink.event(StreamEvent {
            event_type,
            line,
            column,
        }) {
            self.stopped = true;
        }
    }

    /// One note, with the two facts a reader needs to put it back where the tree filed it.
    ///
    /// `owns_line` is the source's own: the note had a line to itself, or it shared one with the
    /// text before it. `owner` is the tree's placement, said in the consumer's build order - the
    /// node the consumer builds next, or the one it built last, and which of the two slots the
    /// note takes there. TOML's projection files a note in exactly one of two slots, so the
    /// direction follows the slot and the only pairings that occur are `FollowingLeading` (a note
    /// above a key, a table or an element) and `PrecedingInline` (a note that ended a line).
    fn note(&mut self, note: &Note, owner: NoteOwner) {
        if self.stopped {
            return;
        }
        let owns_line = self.owns_line(note.at);
        self.at(
            note.at,
            StreamEventType::Comment {
                text: Arc::from(note.text.as_str()),
                owns_line,
                owner,
            },
        );
    }

    /// Whether the note began on a line of its own.
    ///
    /// The IR keeps where the note started, not what stood before it, so the producer reads the
    /// line back: whitespace only up to a line break means the note had the line to itself. A note
    /// the tree files above a key can still fail this test - `# …` written after a comma shares
    /// the line with the value before it, and TOML files it above the *next* element - which is
    /// why the two facts are separate.
    fn owns_line(&self, at: usize) -> bool {
        let bytes = self.text.as_bytes();
        let mut i = at;
        while i > 0 {
            match bytes[i - 1] {
                b' ' | b'\t' => i -= 1,
                b'\n' => return true,
                _ => return false,
            }
        }
        true
    }

    /// A key's text, borrowed from the source wherever the source really says that.
    ///
    /// The IR owns its key names, and an event that outlives the IR cannot borrow from it, so the
    /// only slice available is the document's own bytes. A bare key is exactly its name; a quoted
    /// one is longer, and a dotted inline member is a joined text the source never wrote as one
    /// run. Each of those is copied instead of guessed at.
    fn key_text(&self, name: &str, at: usize) -> Cow<'a, str> {
        let end = at + name.len();
        match self.text.get(at..end) {
            Some(slice) if slice == name => Cow::Borrowed(slice),
            _ => Cow::Owned(String::from(name)),
        }
    }

    fn key(&mut self, name: &str, at: usize) {
        if self.stopped {
            return;
        }
        let style = toml_key_style(name);
        let value = self.key_text(name, at);
        self.at(
            at,
            StreamEventType::Scalar {
                value,
                style,
                anchor: None,
                tag: None,
            },
        );
    }

    /// One table entry: its leading notes, its name, whatever the name holds, its trailing note.
    fn entry(&mut self, name: &str, entry: &TomlTable) {
        if self.stopped {
            return;
        }
        match entry {
            TomlTable::Value(value, anns) => {
                for note in anns.leading.iter() {
                    if self.stopped {
                        return;
                    }
                    self.note(note, NoteOwner::FollowingLeading);
                }
                self.key(name, anns.key_at);
                self.value(value, anns.value_at);
                if self.stopped {
                    return;
                }
                if let Some(note) = anns.trailing.as_ref() {
                    self.note(note, NoteOwner::PrecedingInline);
                }
            }
            TomlTable::Implicit(table) | TomlTable::Explicit(table) => {
                self.key(name, table.key_at);
                if self.stopped {
                    return;
                }
                // The tree files a table's own notes on the *table*, not on its name: the notes
                // above `[a]` lead the mapping `a` holds. The stream reports them where that is
                // true - after the name and before the mapping opens - because the node a consumer
                // builds next is the one the note leads.
                for note in table.leading.iter() {
                    if self.stopped {
                        return;
                    }
                    self.note(note, NoteOwner::FollowingLeading);
                }
                self.body(table);
            }
            TomlTable::ArrayOfTables(list) => {
                // The name is written once per header line but names one thing: the sequence. So it
                // is reported once, before the sequence opens - the shape YAML gives a key whose
                // value is a sequence of mappings, which is also what the tree stores - and each
                // element then arrives as the mapping its own header opened.
                let head = list.first();
                let open = head.map(|table| table.at).unwrap_or(0);
                let key_at = head.map(|table| table.key_at).unwrap_or(0);
                for (index, table) in list.iter().enumerate() {
                    if self.stopped {
                        return;
                    }
                    if index == 0 {
                        self.key(name, key_at);
                        if self.stopped {
                            return;
                        }
                        self.at(
                            open,
                            StreamEventType::SequenceStart {
                                anchor: None,
                                tag: None,
                            },
                        );
                        if self.stopped {
                            return;
                        }
                    }
                    // An element's own notes lead the element's mapping, so they are reported
                    // after the sequence opens and before the element does.
                    for note in table.leading.iter() {
                        if self.stopped {
                            return;
                        }
                        self.note(note, NoteOwner::FollowingLeading);
                    }
                    self.body(table);
                }
                self.at(open, StreamEventType::SequenceEnd);
            }
        }
    }

    /// The container itself and everything inside it.
    fn body(&mut self, table: &CowTable) {
        if self.stopped {
            return;
        }
        self.at(
            table.at,
            StreamEventType::MappingStart {
                anchor: None,
                tag: None,
            },
        );
        if self.stopped {
            return;
        }
        if let Some(note) = table.comment.as_ref() {
            self.note(note, NoteOwner::PrecedingInline);
            if self.stopped {
                return;
            }
        }
        for (name, entry) in table.entries.iter() {
            if self.stopped {
                return;
            }
            self.entry(name, entry);
        }
        // A close event carries its construct's own opening offset: TOML never wrote an end.
        self.at(table.at, StreamEventType::MappingEnd);
    }

    fn value(&mut self, value: &TomlValue, at: usize) {
        if self.stopped {
            return;
        }
        match value {
            TomlValue::Array(items) => {
                self.at(
                    at,
                    StreamEventType::SequenceStart {
                        anchor: None,
                        tag: None,
                    },
                );
                if self.stopped {
                    return;
                }
                for (item, anns) in items.iter() {
                    if self.stopped {
                        return;
                    }
                    for note in anns.leading.iter() {
                        if self.stopped {
                            return;
                        }
                        self.note(note, NoteOwner::FollowingLeading);
                    }
                    self.value(item, anns.value_at);
                    if self.stopped {
                        return;
                    }
                    if let Some(note) = anns.trailing.as_ref() {
                        self.note(note, NoteOwner::PrecedingInline);
                    }
                }
                self.at(at, StreamEventType::SequenceEnd);
            }
            TomlValue::InlineTable(members) => {
                self.at(
                    at,
                    StreamEventType::MappingStart {
                        anchor: None,
                        tag: None,
                    },
                );
                if self.stopped {
                    return;
                }
                for (name, item, anns) in members.iter() {
                    if self.stopped {
                        return;
                    }
                    for note in anns.leading.iter() {
                        if self.stopped {
                            return;
                        }
                        self.note(note, NoteOwner::FollowingLeading);
                    }
                    self.key(name, anns.key_at);
                    if self.stopped {
                        return;
                    }
                    self.value(item, anns.value_at);
                    if self.stopped {
                        return;
                    }
                    if let Some(note) = anns.trailing.as_ref() {
                        self.note(note, NoteOwner::PrecedingInline);
                    }
                }
                self.at(at, StreamEventType::MappingEnd);
            }
            other => {
                // The producer borrows the IR, so this is where the scalar's text is paid for -
                // except for the literals that are `'static`, which the projection hands back
                // borrowed and no reader has to own at all.
                let projection = scalar_projection(other.clone());
                self.at(
                    at,
                    StreamEventType::Scalar {
                        value: projection.text.into_cow(),
                        style: projection.style,
                        anchor: None,
                        tag: projection.tag,
                    },
                );
            }
        }
    }
}
