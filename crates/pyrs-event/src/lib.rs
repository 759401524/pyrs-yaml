//! # pyrs-event
//!
//! The event vocabulary the document stream is described in: `StreamStart` / `StreamEnd`,
//! `DocumentStart` / `DocumentEnd`, scalar, mapping and sequence open/close, alias, and comment.
//! It sits beside `pyrs-ast` - the node model - and,
//! like it, is `no_std` + `alloc` and free of any format engine's own types.
//!
//! ## What this crate is, at this step
//!
//! The shape here is YAML's, moved verbatim out of `crates/pyrs-yaml-core/src/parser/stream.rs`.
//! That is deliberate: a neutral vocabulary has to be designed against more than one producer, and
//! the producers do not exist yet. The sequence this step belongs to is recorded in
//! `docs/dev/event-stream.md`:
//!
//! 1. **this crate**, holding the type, with the YAML adapter left where it is and re-exporting -
//!    no behaviour change, so the instruction-count gate proves the move cost nothing.
//! 2. a JSON producer, which is the first thing that says which fields are shared and which are a
//!    format's payload.
//! 3. a TOML producer, same question again.
//! 4. a replay property test - events fed back through one reader must rebuild the AST the direct
//!    parsers build - because that is the only test that can show the vocabulary is neutral rather
//!    than merely renamed.
//!
//! So a field like `anchor`, `tag` or `Alias` is not a claim that JSON has anchors; it is YAML's
//! grammar still visible, and step 2 is where it either generalises or moves into a payload.

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::sync::Arc;
use pyrs_ast::ast::{ScalarStyle, Tag};

/// Line and column information for a YAML stream event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEvent {
    /// The type of event that occurred.
    pub event_type: StreamEventType,
    /// The line number (0-indexed) where the event starts.
    pub line: usize,
    /// The column number (0-indexed) where the event starts.
    pub column: usize,
}

/// The type of a streaming YAML event, representing a single occurrence
/// in the YAML document stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEventType {
    /// The beginning of the YAML stream.
    StreamStart,
    /// The end of the YAML stream.
    StreamEnd,
    /// The beginning of a YAML document.
    DocumentStart,
    /// The end of a YAML document.
    DocumentEnd,
    /// A scalar value.
    Scalar {
        /// The scalar text value.
        value: String,
        /// The scalar style (plain, quoted, literal, folded).
        style: ScalarStyle,
        /// The anchor name, if present.
        anchor: Option<String>,
        /// The YAML tag, if present.
        tag: Option<Tag>,
    },
    /// The beginning of a mapping node.
    MappingStart {
        /// The anchor name, if present.
        anchor: Option<String>,
        /// The YAML tag, if present.
        tag: Option<Tag>,
    },
    /// The end of a mapping node.
    MappingEnd,
    /// The beginning of a sequence node.
    SequenceStart {
        /// The anchor name, if present.
        anchor: Option<String>,
        /// The YAML tag, if present.
        tag: Option<Tag>,
    },
    /// The end of a sequence node.
    SequenceEnd,
    /// An alias reference.
    Alias {
        /// The alias name.
        name: String,
    },
    /// A comment line or inline comment.
    Comment {
        /// The comment text (without the `#` prefix and leading space).
        text: Arc<str>,
        /// Whether this is a standalone line comment (`true`) or an inline
        /// comment at the end of a line (`false`).
        standalone: bool,
    },
}
