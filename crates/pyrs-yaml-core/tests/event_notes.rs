//! The YAML dialect's half of decision 6: one note flag, two questions.
//!
//! A note carries two facts that a single boolean could not answer:
//!
//! - **where the tree files it** — the node's leading list or its one inline
//!   slot. The tree's `standalone` flag *is* that slot, so the tree answers
//!   this question and only this one.
//! - **whether the source gave it a line of its own** — a fact about the text,
//!   which the tree never reads and therefore cannot answer.
//!
//! The stream reports both: `owner` names the slot and the direction, and
//! `owns_line` carries the source fact. These tests are the per-dialect
//! invariant decision 6 asks for — for every note the tree files on a node, the
//! stream reports the same note in the same slot, and the two walks answer the
//! same two questions.
//!
//! The tree is the reference for the placement and the source text is the
//! reference for the own-line fact, so the two are checked against different
//! oracles on purpose: the tree cannot be asked about `owns_line`, and the
//! source cannot be asked about the slot.

use std::ops::Range;

use pyrs_yaml_core::ast::CustomNode;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::parser::{NoteOwner, StreamEvent, StreamEventType, parse, parse_stream};

/// A note the tree files, with the slot it files it in.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeNote {
    text: String,
    /// True for the node's leading list, false for its one inline slot.
    leading: bool,
    /// The node the note is filed on, as a byte range in the source.
    node: Range<usize>,
}

/// A note the stream reports.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StreamNote {
    text: String,
    owns_line: bool,
    owner: NoteOwner,
    /// The byte offset of the note's `#` in the source.
    offset: usize,
}

/// Every note the tree files, with the slot and the node it is filed on.
fn tree_notes(node: &CustomNode, into: &mut Vec<TreeNote>) {
    let range = node.source_range().cloned().unwrap_or(0..0);
    // The inline slot is the `comment` field with `standalone = false`; the
    // leading list is the decor's list. A note the tree files in the leading
    // list carries `standalone = true`, which is why the flag answers the
    // slot question and not the source's own-line question.
    if let Some(comment) = node.comment().filter(|c| !c.standalone) {
        into.push(TreeNote {
            text: comment.text.to_string(),
            leading: false,
            node: range.clone(),
        });
    }
    for comment in node.leading_comments() {
        into.push(TreeNote {
            text: comment.text.to_string(),
            leading: true,
            node: range.clone(),
        });
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs.iter() {
                tree_notes(key, into);
                tree_notes(value, into);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                tree_notes(item, into);
            }
        }
        _ => {}
    }
}

/// Every note the stream reports, with the source fact and the owner.
fn stream_notes(events: &[StreamEvent<'_>], source: &str) -> Vec<StreamNote> {
    let starts = line_starts(source);
    events
        .iter()
        .filter_map(|event| match &event.event_type {
            StreamEventType::Comment {
                text,
                owns_line,
                owner,
            } => Some(StreamNote {
                text: text.to_string(),
                owns_line: *owns_line,
                owner: *owner,
                offset: offset_of(source, &starts, event.line, event.column),
            }),
            _ => None,
        })
        .collect()
}

/// The byte offset of every line start in `source`, using YAML's break set.
///
/// A CR is a line break, so a CR-only document has as many lines as an
/// LF-only one. Counting only `\n` would put every note of such a document on
/// the wrong line and turn the source-fact check below into a false red.
fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\n' => {
                index += 1;
                starts.push(index);
            }
            b'\r' => {
                index += 1;
                if index < bytes.len() && bytes[index] == b'\n' {
                    index += 1;
                }
                starts.push(index);
            }
            _ => index += 1,
        }
    }
    starts
}

/// The byte offset of the `column`-th character of `line`.
///
/// The column counts characters, not bytes, so the line is walked by character
/// and the byte offset is read off the walk.
fn offset_of(source: &str, starts: &[usize], line: usize, column: usize) -> usize {
    let Some(&start) = starts.get(line) else {
        return 0;
    };
    let rest = &source[start..];
    match rest.char_indices().nth(column) {
        Some((byte, _)) => start + byte,
        None => start + rest.len(),
    }
}

/// Whether the source gave the note a line of its own.
///
/// The fact is read off the text: nothing but blanks precedes the `#` on that
/// line. granit answers the same question with its placement — a `Right` note
/// is the one whose line carried content before the `#` — so the two agree by
/// construction and disagree only if one of them is wrong.
fn owns_line_in_source(source: &str, starts: &[usize], note: &StreamNote) -> bool {
    let line = match starts.iter().rposition(|&start| start <= note.offset) {
        Some(index) => index,
        None => return false,
    };
    source[starts[line]..note.offset]
        .chars()
        .all(|c| c == ' ' || c == '\t')
}

/// The one stream note with this text. The documents below use distinct texts,
/// so a second one with the same text is a failure, not an ambiguity.
fn the_note<'a>(notes: &'a [StreamNote], text: &str) -> &'a StreamNote {
    let mut found = notes.iter().filter(|note| note.text == text);
    let first = found
        .next()
        .unwrap_or_else(|| panic!("the stream does not report {text:?}"));
    assert!(
        found.next().is_none(),
        "the stream reports {text:?} more than once"
    );
    first
}

/// The two walks answer the same two questions.
///
/// The tree's flag answers "which slot", so a note the source gave a line of
/// its own and the tree files inline reads as "not standalone" — and a note
/// the source gave no line of its own but the tree files in the leading list
/// reads as "standalone". Both are the tree's own answer to the slot question,
/// and both look like the opposite answer to the source's question. The split
/// reports the slot and the source fact separately, so the two walks answer
/// the same two questions.
#[test]
fn the_two_walks_answer_the_same_two_questions() {
    // An own-line note at the end of the document: the tree files it in the
    // root's inline slot (the writer hangs exactly one note on the document's
    // last line), so the tree's flag says "not standalone" while the source
    // gave the note a line of its own.
    let trailing = "a: 1\n# trailing\n";
    let tree = parse(trailing, Schema::Core).expect("the document parses");
    let mut kept = Vec::new();
    tree_notes(&tree, &mut kept);
    assert_eq!(
        kept,
        vec![TreeNote {
            text: "trailing".to_string(),
            leading: false,
            node: kept[0].node.clone(),
        }],
        "the tree files the trailing note inline"
    );
    let reported = stream_notes(
        &parse_stream(trailing).expect("the document streams"),
        trailing,
    );
    assert_eq!(
        reported,
        vec![StreamNote {
            text: "trailing".to_string(),
            owns_line: true,
            owner: NoteOwner::PrecedingInline,
            offset: 5,
        }],
        "the stream reports the slot and the source fact separately"
    );

    // A note on a property-only root: the tree cannot hang it on the node
    // (there is no node yet), so it carries it forward into the leading list
    // and the flag says "standalone" — while the source gave the note no line
    // of its own.
    let property = "!x # note\n";
    let tree = parse(property, Schema::Core).expect("the document parses");
    let mut kept = Vec::new();
    tree_notes(&tree, &mut kept);
    assert!(
        kept.iter().all(|note| note.leading),
        "the tree carries the note into the leading list: {kept:?}"
    );
    let reported = stream_notes(
        &parse_stream(property).expect("the document streams"),
        property,
    );
    assert_eq!(
        reported,
        vec![StreamNote {
            text: "note".to_string(),
            owns_line: false,
            owner: NoteOwner::FollowingLeading,
            offset: 3,
        }],
        "the stream reports the slot and the source fact separately"
    );
}

/// The per-dialect invariant of decision 6.
///
/// For every note the tree files on a node, the stream reports the same note
/// in the same slot, and the direction agrees with where the note sits
/// relative to the node it leads.
#[test]
fn every_note_the_tree_files_is_reported_with_the_same_owner() {
    let cases: &[&str] = &[
        "# above\na: 1\n",
        "a: 1 # same line\n",
        "a:\n  # above the value\n  1\n",
        "a: 1\nb: 2 # on b\n",
        "# one\n# two\na: 1\n",
        "a: &x 1\nb: *x # on the alias\n",
        "a: [1, 2] # after a flow value\n",
        "a: {b: 1} # after a flow mapping\n",
        "- 1 # on the item\n- 2\n",
        "# above a sequence\n- 1\n",
        "a: 1\n\n# after a blank line\n",
        "# above a flow mapping\n{a: 1}\n",
        "a:\n  # above the nested mapping\n  b: 1\n",
        "value # on the root\n",
    ];
    for text in cases {
        let tree = parse(text, Schema::Core).unwrap_or_else(|e| panic!("{text:?} must parse: {e}"));
        let events = parse_stream(text).unwrap_or_else(|e| panic!("{text:?} must stream: {e}"));
        let mut from_tree = Vec::new();
        tree_notes(&tree, &mut from_tree);
        let from_stream = stream_notes(&events, text);
        let starts = line_starts(text);

        for note in &from_tree {
            let reported = the_note(&from_stream, &note.text);
            // The slot has to agree note for note.
            assert_eq!(
                reported.owner.is_leading(),
                note.leading,
                "{text:?}: {note:?} is filed in one slot and reported in another"
            );
            // The direction names the node the consumer builds next or last, so
            // it is decided by where the note sits relative to the node it
            // leads: a note above the node it leads is read before that node,
            // and a note the tree files on a node it had already built — the
            // root's trailing list, an empty container's own slot — is read
            // after it.
            if note.leading {
                let expected = if reported.offset < note.node.start {
                    NoteOwner::FollowingLeading
                } else {
                    NoteOwner::PrecedingLeading
                };
                assert_eq!(
                    reported.owner, expected,
                    "{text:?}: {note:?} is reported with the wrong direction"
                );
            } else {
                // The inline slot always trails the node it belongs to.
                assert_eq!(
                    reported.owner,
                    NoteOwner::PrecedingInline,
                    "{text:?}: {note:?} takes the inline slot but is not reported as trailing"
                );
            }
        }

        // The source fact has to agree with the text.
        for note in &from_stream {
            assert_eq!(
                note.owns_line,
                owns_line_in_source(text, &starts, note),
                "{text:?}: {note:?} misreports whether the source gave it a line of its own"
            );
        }
    }
}

/// The notes the tree drops are the ones the ledger names.
///
/// The tree files one note per slot: a second trailing note has nowhere to go
/// and is dropped, and an alias carries no slot, so the note the tree would
/// have hung on it is re-homed onto the key that owns the pair. The stream
/// reports every note the source has — the tree's own drops are the
/// difference between the two, not a disagreement about a note both have.
#[test]
fn the_notes_the_tree_drops_are_the_documented_ones() {
    // Two trailing notes: the tree keeps the first (the one the writer hangs
    // on the document's last line) and drops the second.
    let stacked = "a: 1\n# first\n# second\n";
    let tree = parse(stacked, Schema::Core).expect("the document parses");
    let mut kept = Vec::new();
    tree_notes(&tree, &mut kept);
    assert_eq!(
        kept.iter()
            .map(|note| note.text.as_str())
            .collect::<Vec<_>>(),
        vec!["first"],
        "the tree keeps the first trailing note"
    );
    let reported = stream_notes(
        &parse_stream(stacked).expect("the document streams"),
        stacked,
    );
    assert_eq!(
        reported
            .iter()
            .map(|note| (note.text.as_str(), note.owner))
            .collect::<Vec<_>>(),
        vec![
            ("first", NoteOwner::PrecedingInline),
            ("second", NoteOwner::PrecedingInline),
        ],
        "the stream reports every note the source has"
    );

    // An alias carries no slot, so the note the tree would have hung on it is
    // re-homed onto the key that owns the pair. The slot is the same — the
    // inline one — but the node is the key's, not the alias's.
    let aliased = "a: &x 1\nb: *x # on the alias\n";
    let tree = parse(aliased, Schema::Core).expect("the document parses");
    let mut kept = Vec::new();
    tree_notes(&tree, &mut kept);
    assert_eq!(
        kept.iter()
            .map(|note| note.text.as_str())
            .collect::<Vec<_>>(),
        vec!["on the alias"],
        "the tree re-homes the note onto the key"
    );
    assert!(!kept[0].leading, "the re-homed note takes the inline slot");
}
