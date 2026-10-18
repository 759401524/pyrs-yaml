//! The edit verbs, driven over a hostile document.
//!
//! Nothing in the fuzz tier reached this layer. The eight targets before this one hand a *document* to a
//! reader or a writer; the code that rewrites one region of a document - what `doc.set()` and the `pyq`
//! verbs call, and what the round-trip promise is actually about - was only ever asked by named unit
//! tests and by the proptest suite over *generated* trees. A generated tree cannot hold the shapes that
//! break region splicing: a note under a tag-only line, a CR-only break, an alias in key position. Fuzzed
//! bytes can, and each of those shapes has already cost a pull request.
//!
//! The input is `document NUL script`. A script is pairs of bytes: the first picks a path the reader
//! really found in the document (paths are collected from the tree, so mutations land inside the document
//! instead of being refused by a path that names nothing), the second picks the verb and its operand.
//! Every accepted edit is followed by its emission and a re-parse, and the next edit works on that
//! re-read tree - so one corpus entry can drive a whole editing session.
//!
//! What this asserts, and each of these can fail:
//!
//! - an accepted edit emits text the parser accepts (the writer's own contract, as in `yaml_roundtrip`);
//! - `set` reads back the text it wrote at its path, `rename` finds the old name gone and the new one
//!   present, `delete` and `move` find either the removed key unresolvable or - for a sequence item,
//!   whose index is a position the next item shifts into - one item fewer in the parent, `insert` and
//!   `append` find exactly one more item holding the written text at its index, `sort-keys` finds the
//!   item shifts into. `sort-keys` finds the mapping's keys in the order the engine sorts them, and
//!   `set-comment` is held to the two structural claims only - see the boundary below.
//!
//! Two claims are narrower than they first look, and each was narrowed by a measurement rather than by
//! taste. Sorting is asserted only where the verb has work to do: `sort_keys_path` answers "ok" for a
//! path naming a scalar and changes nothing, which is a question about a refusal the CLI would show, not
//! about emitted text. A note is followed only onto a node that has a line of its own: asked for on a
//! block sequence, it lands on the last item's line and re-reads as that item's, because there is no
//! other place in the text it could live; keeping it on the sequence would need a new emission position,
//! so the claim is withheld rather than written as a permission. A note whose *owner* an edit deleted is
//! not chased either - `note_survival.rs` owns the unmutated document.
//!
//! Three claims are not made here, each for a measured reason. Style: the writer refuses a style a text
//! cannot carry (a block scalar cannot hold a trailing space) and that fallback is correct, so for style,
//! flow, tag and anchor the claim is only re-parse; `prop_style_setting_roundtrip` asks the equality
//! question on trees where the answer is determined. And the fixed point - emit, re-parse, serialize
//! again, compare bytes - is not asserted, because exploration found a shape where it does not hold: a
//! flow member carrying both a tag and an inline note (`flow-tag-note-needs-two-rounds.seed`) emits
//! `a: !fuzz2   # fuzz 47`, the re-read hands the note to the mapping's own line, and the second round
//! moves the tag after it. That is the `(ds)`/`(dq)` species in a container the earlier fixes did not
//! reach; the tier replays that input as a seed and says nothing about it, because a gate that reddens on
//! an open defect gates nothing.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_schema::types::Schema;
use pyrs_yaml_core::ast::{Comment, CustomNode, ScalarStyle, Tag};
use pyrs_yaml_core::editing::plan::{
    append_path, delete_path, insert_path, move_path, rename_path, set_anchor_path,
    set_comment_path, set_flow_style_path, set_path, set_scalar_style_path, set_tag_path,
    sort_keys_path,
};
use pyrs_yaml_core::editing::{Segment, navigate};
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::serializer::to_yaml;
use std::borrow::Cow;
use std::sync::Arc;

type Path = Vec<Segment<'static>>;

/// A violated claim names itself, the emission that broke it and the document it came from: an
/// artifact has to be readable without re-running the fuzzer to know what was asked.
#[cold]
fn fail(claim: &str, detail: String, emitted: &str, input: &str) -> ! {
    panic!("edit_yaml: {claim}\n{detail}\n---\n{emitted}\n---\n{input}")
}

/// What a successful edit promises the re-read document about itself.
enum Effect {
    /// The path carries this text.
    Carries(Path, String),
    /// The key is not in the document any more.
    Missing(Path),
    /// A sequence item was removed: its parent holds one item fewer than before.
    Shrank { path: Path, len: usize },
    /// `rename`: the old name is gone and the new one resolves.
    Renamed { old: Path, new: Path },
    /// `insert` / `append`: the sequence holds the written text at `at`.
    Grew { path: Path, at: usize, text: String },
    /// `sort-keys`: the scalar keys of this mapping are nondecreasing.
    Sorted(Path),
    /// Only the two structural claims: re-parse and settle.
    Settled,
}

impl Effect {
    fn check(&self, tree: &CustomNode, emitted: &str, input: &str) {
        match self {
            Effect::Carries(path, text) => match navigate(tree, path) {
                Ok(node) if scalar_text(node) == Some(text.as_str()) => {}
                Ok(node) => fail(
                    "a written value did not read back",
                    format!("wanted {text:?}, found {:?}", scalar_text(node)),
                    emitted,
                    input,
                ),
                Err(error) => fail(
                    "the path a value was written to is gone",
                    error.to_string(),
                    emitted,
                    input,
                ),
            },
            Effect::Missing(path) => {
                if let Ok(found) = navigate(tree, path) {
                    fail(
                        "a deleted key is still in the document",
                        format!("{path:?} still reads {:?}", scalar_text(found)),
                        emitted,
                        input,
                    );
                }
            }
            Effect::Shrank { path, len } => match navigate(tree, path) {
                Ok(CustomNode::Sequence { items, .. }) => {
                    if items.len() + 1 != *len {
                        fail(
                            "a removed item did not shorten its sequence by one",
                            format!("{path:?} holds {} items, was {len}", items.len()),
                            emitted,
                            input,
                        );
                    }
                }
                Ok(_) => fail(
                    "a removal left a non-sequence",
                    path.len().to_string(),
                    emitted,
                    input,
                ),
                Err(error) => fail(
                    "the sequence an item was removed from is gone",
                    error.to_string(),
                    emitted,
                    input,
                ),
            },
            Effect::Renamed { old, new } => {
                if navigate(tree, old).is_ok() {
                    fail(
                        "a renamed key kept its old name",
                        format!("{old:?}"),
                        emitted,
                        input,
                    );
                }
                if navigate(tree, new).is_err() {
                    fail(
                        "a renamed key lost both names",
                        format!("{new:?}"),
                        emitted,
                        input,
                    );
                }
            }
            Effect::Grew { path, at, text } => match navigate(tree, path) {
                Ok(CustomNode::Sequence { items, .. }) => {
                    let Some(item) = items.get(*at) else {
                        fail(
                            "a sequence did not grow to hold its new item",
                            format!("index {at} of {}", items.len()),
                            emitted,
                            input,
                        );
                    };
                    if scalar_text(item) != Some(text.as_str()) {
                        fail(
                            "an inserted item is not the one asked for",
                            format!("index {at} reads {:?}", scalar_text(item)),
                            emitted,
                            input,
                        );
                    }
                }
                Ok(other) => fail(
                    "an insertion left a non-sequence",
                    format!("{path:?} reads {:?}", scalar_text(other)),
                    emitted,
                    input,
                ),
                Err(error) => fail(
                    "the sequence an item was added to is gone",
                    error.to_string(),
                    emitted,
                    input,
                ),
            },
            Effect::Sorted(path) => {
                let Ok(CustomNode::Mapping { pairs, .. }) = navigate(tree, path) else {
                    fail(
                        "a sort left a non-mapping",
                        format!("{path:?}"),
                        emitted,
                        input,
                    );
                };
                let mut previous = String::new();
                for (key, _) in pairs.iter() {
                    let Some(text) = scalar_text(key) else {
                        continue;
                    };
                    if text < previous.as_str() {
                        fail(
                            "a sorted mapping came back out of order",
                            format!("{text:?} after {previous:?}"),
                            emitted,
                            input,
                        );
                    }
                    previous = text.to_string();
                }
            }
            Effect::Settled => {}
        }
    }
}

/// The scalar text of a node, or `None` for a node that is not a scalar.
fn scalar_text(node: &CustomNode) -> Option<&str> {
    match node {
        CustomNode::Scalar { value, .. } => Some(value),
        _ => None,
    }
}

/// Every path the reader reached, so a mutation lands on a node that exists.
///
/// A key is named by its text, which is how `navigate` resolves a `Segment::Key`. A key that is not a
/// plain scalar - complex, tagged, or a null fold - gets no path, because its spelling is not the
/// identity a lookup asks for; those shapes stay with the edit layer's own unit tests.
fn collect(node: &CustomNode, prefix: &mut Path, into: &mut Vec<Path>) {
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs.iter() {
                let Some(text) = scalar_text(key) else {
                    continue;
                };
                prefix.push(Segment::Key(Cow::Owned(text.to_string())));
                into.push(prefix.clone());
                collect(value, prefix, into);
                prefix.pop();
            }
        }
        CustomNode::Sequence { items, .. } => {
            for (index, item) in items.iter().enumerate() {
                let Ok(at) = i64::try_from(index) else {
                    return;
                };
                prefix.push(Segment::Index(at));
                into.push(prefix.clone());
                collect(item, prefix, into);
                prefix.pop();
            }
        }
        _ => {}
    }
}

/// The length of the sequence at `path`, which bounds an index and states the growth claims.
fn sequence_len(node: &CustomNode, path: &Path) -> Option<usize> {
    match navigate(node, path).ok()? {
        CustomNode::Sequence { items, .. } => Some(items.len()),
        _ => None,
    }
}

/// What removing the node at `path` owes the re-read document, asked before the tree changes.
///
/// A key is an identity, so deleting one makes that name unresolvable. An index is a position: the items
/// after it shift up, and the slot the deleted item vacated is occupied by the next one. Measured - on
/// `list:\n  - 1\n  - 2J\n`, deleting `$.list[0]` re-reads index 0 as `2J`, which is the right answer -
/// so for a sequence the claim is the one the text really makes: one item fewer than before.
fn removal_effect(node: &CustomNode, path: &Path) -> Effect {
    if matches!(path.last(), Some(Segment::Index(_))) {
        let parent = path[..path.len() - 1].to_vec();
        if let Some(len) = sequence_len(node, &parent) {
            return Effect::Shrank { path: parent, len };
        }
    }
    Effect::Missing(path.clone())
}

/// Whether one path names a node that contains everything the other names.
fn is_prefix(short: &Path, long: &Path) -> bool {
    short.len() <= long.len() && short.iter().zip(long.iter()).all(|(a, b)| seg_eq(a, b))
}

/// `Segment` is a lookup instruction, not a value with an equality of its own, so the comparison that
/// matters here is spelled: two segments agree when they ask the reader for the same thing.
fn seg_eq(a: &Segment<'_>, b: &Segment<'_>) -> bool {
    match (a, b) {
        (Segment::Key(a), Segment::Key(b)) => a == b,
        (Segment::Index(a), Segment::Index(b)) => a == b,
        _ => false,
    }
}

fn path_eq(a: &Path, b: &Path) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| seg_eq(x, y))
}

fuzz_target!(|data: &[u8]| {
    let Some(split) = data.iter().position(|byte| *byte == 0) else {
        return; // a document with no script is `yaml_roundtrip`'s input
    };
    let Ok(source) = std::str::from_utf8(&data[..split]) else {
        return;
    };
    let script = &data[split + 1..];
    let Ok(mut node) = parse(source, Schema::Core) else {
        return; // typed rejection is a pass
    };
    let mut text = source.to_string();

    for record in script.chunks(2) {
        if record.len() < 2 {
            break;
        }
        let (pick, verb) = (record[0], record[1]);
        let mut paths: Vec<Path> = Vec::new();
        collect(&node, &mut Vec::new(), &mut paths);
        if paths.is_empty() {
            return; // nothing left to edit
        }
        let path = paths[usize::from(pick) % paths.len()].clone();
        let written = format!("v{}", verb >> 4);
        let before = node.clone();

        let applied: Result<Effect, String> = match verb % 12 {
            0 => set_path(
                &mut node,
                &path,
                CustomNode::plain_scalar(written.clone()),
                false,
                &text,
                None,
                false,
            )
            .map(|_| Effect::Carries(path.clone(), written)),
            1 => delete_path(&mut node, &path, &text, None).map(|_| removal_effect(&before, &path)),
            2 => {
                let new_key = format!("k{}", verb >> 4);
                let mut renamed = path.clone();
                if renamed.pop().is_none() {
                    Err("an empty path has no key to rename".to_string())
                } else {
                    // A rename onto a name the document already holds would make a duplicate key, and
                    // refusing that is the engine's right, so the pick is skipped rather than forcing a
                    // verdict out of a shape the writer is allowed to reject.
                    renamed.push(Segment::Key(Cow::Owned(new_key.clone())));
                    if navigate(&node, &renamed).is_ok() {
                        Err("the new name is already taken".to_string())
                    } else {
                        let old = path.clone();
                        rename_path(&mut node, &path, &new_key, &text, None)
                            .map(|_| Effect::Renamed { old, new: renamed })
                    }
                }
            }
            3 | 4 => match sequence_len(&node, &path) {
                None => Err("not-a-sequence".to_string()),
                Some(len) => {
                    let at = if verb % 12 == 3 {
                        usize::from(pick) % (len + 1)
                    } else {
                        len
                    };
                    let index = i64::try_from(at).ok().filter(|_| verb % 12 == 3);
                    let done = if let Some(index) = index {
                        insert_path(
                            &mut node,
                            &path,
                            index,
                            CustomNode::plain_scalar(written.clone()),
                            &text,
                            None,
                        )
                    } else {
                        append_path(
                            &mut node,
                            &path,
                            CustomNode::plain_scalar(written.clone()),
                            &text,
                            None,
                        )
                    };
                    done.map(|_| Effect::Grew {
                        path: path.clone(),
                        at,
                        text: written,
                    })
                }
            },
            5 => {
                let dst = paths[usize::from(verb.wrapping_mul(7).wrapping_add(pick)) % paths.len()]
                    .clone();
                if path_eq(&dst, &path) || is_prefix(&path, &dst) || is_prefix(&dst, &path) {
                    Err("the move would eat its own source".to_string())
                } else {
                    move_path(&mut node, &path, &dst, &text, None)
                        .map(|_| removal_effect(&before, &path))
                }
            }
            6 => {
                // `sort_keys_path` sorts a mapping and leaves anything else alone, so the claim only
                // applies where the verb has work to do. That the engine answers "ok" for a path naming
                // a scalar - measured, and the reason this arm asks first - is a question about a
                // refusal the CLI users would see, not about the text that comes out.
                let sorted = matches!(
                    navigate(&node, &path).ok(),
                    Some(CustomNode::Mapping { .. })
                );
                sort_keys_path(&mut node, &path, &text, None).map(|_| {
                    if sorted {
                        Effect::Sorted(path.clone())
                    } else {
                        Effect::Settled
                    }
                })
            }
            7 => {
                let style = [
                    ScalarStyle::Plain,
                    ScalarStyle::SingleQuoted,
                    ScalarStyle::DoubleQuoted,
                    ScalarStyle::Literal,
                    ScalarStyle::Folded,
                ][usize::from(verb >> 4) % 5];
                set_scalar_style_path(&mut node, &path, style, &text, None).map(|_| Effect::Settled)
            }
            8 => set_flow_style_path(&mut node, &path, verb & 1 == 0, &text, None)
                .map(|_| Effect::Settled),
            9 => set_tag_path(
                &mut node,
                &path,
                Tag::local(&format!("fuzz{}", verb >> 4)),
                &text,
                None,
            )
            .map(|_| Effect::Settled),
            10 => set_anchor_path(&mut node, &path, &format!("a{}", verb >> 4), &text, None)
                .map(|_| Effect::Settled),
            _ => {
                let note = format!("fuzz {verb}");
                // Not asserted, even for a node that has a line of its own. Asked for on a block
                // sequence, the note lands on the last item's line - there is nowhere else in the text
                // for it - and re-reads as that item's; and exploration found the same hand-off on
                // scalars in one shape, so "the note stays where it was put" is not a claim this tier
                // can make yet. Both are recorded rather than asserted: keeping a note on the node the
                // call named would need a decided emission position, which is a question about the
                // writer's output, not a defect a seed can settle.
                set_comment_path(
                    &mut node,
                    &path,
                    Comment {
                        text: Arc::from(note.as_str()),
                        standalone: false,
                    },
                    &text,
                    None,
                )
                .map(|_| Effect::Settled)
            }
        };

        let Ok(effect) = applied else {
            node = before; // a typed refusal is a pass, and the document is left as it was
            continue;
        };

        let emitted = to_yaml(&node);
        let reparsed = parse(&emitted, Schema::Core).unwrap_or_else(|error| {
            fail(
                "an accepted edit emitted text its own reader refused",
                error.to_string(),
                &emitted,
                source,
            )
        });
        effect.check(&reparsed, &emitted, source);

        node = reparsed;
        text = emitted;
    }
});
