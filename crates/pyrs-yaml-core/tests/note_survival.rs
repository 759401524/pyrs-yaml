//! Note survival over the committed fuzz corpus.
//!
//! The round-trip tier's oracle is idempotence — `to_yaml(parse(to_yaml(x))) ==
//! to_yaml(x)` — and a document that is stable and merely short a note passes it.
//! Losing a note is therefore invisible to the assertion that has driven every
//! comment fix in this engine, which is how five silent losses (stacked notes,
//! comment-only documents, a folded null key, a consumed merge key and the mapping
//! it consumed) survived behind green tests. This file asserts the other invariant:
//!
//! **every note the reader recorded for an input comes back on the node that carried
//! it, or — where the format cannot say otherwise — on one of its ancestors.** The one exception is
//! the document's own note, whose owner #113 ruled may move down to the line that ends the body; it
//! is named in its own paragraph below rather than folded into this contract.
//!
//! It walks the corpora by spoke, named because the set is closed and grows with each fixed crash
//! rather than with someone's imagination: the YAML spoke reads `fuzz/seeds/yaml_roundtrip`,
//! `fuzz/seeds/parse_yaml` and the document half of `fuzz/seeds/edit_yaml` (whose entries are
//! `document NUL script`), and the TOML spoke reads `fuzz/seeds/toml_roundtrip` and
//! `fuzz/seeds/parse_toml` through `from_toml` / `to_toml`. Measured 2026-11-18 the two spokes
//! charge 120 + 2 + 1 and 11 + 3 note-bearing seeds. Inputs that are not valid UTF-8 or that the
//! parser (correctly) rejects are skipped: they have no AST, so there is no note to keep.
//!
//! **Placement, not presence.** A text search is the weaker question: `# K` re-read above the
//! neighbouring entry is still *in* the emitted text, so `emitted.contains(text)` passes while the
//! note has changed owner. Both spokes here key every note by the route the reader walks to it — a
//! mapping key and its value share one segment (a note on either names the member, not the slot), a
//! sequence item takes its index, and a key the route cannot spell collapses to `?` — flatten it the
//! way the writers spell a multi-line note, and compare multisets. Measured 2026-11-18, the first
//! reading found three placements the presence oracle could not see: a notated array-table element
//! and a notated multi-line inline-table member each wrote the note after the separator, so the
//! reader handed it to the *next* member — both fixed at the writer — and the document's own
//! trailing note (YAML) was appended to the last value's line and re-read as that value's note,
//! which is the ruled movement the next paragraph names rather than a defect to fix.
//!
//! **How far each writer is asked to keep the node.** TOML is asked for the exact multiset, and holds
//! it on all 14 note-bearing seeds. YAML cannot: the writer places a child's leading note on the line
//! above its entry, and granit reports a note line at the head of a block as the *block's*, so a note
//! the tree holds on `?/k` re-reads on the mapping that contains `k`. Measured over the corpus,
//! 59 of the 123 note-bearing documents place every note exactly, 61 more need only the ancestor
//! direction, and the last 3 are the document's own note moving down its body's final line (below).
//! A move can cross several levels, so the contract is "the node or any ancestor", not "one level
//! up", and deciding whether the notes can be paired that way is a bipartite matching rather than a
//! per-note check: a greedy pass reddened this corpus with three notes that the matching pairs
//! without moving anything the emission did not move. That direction, and only that
//! direction, is accepted: every note must return on the node that carried it or on an ancestor of
//! it, never on a sibling, another branch or a child — except for the document's own note, below.
//!
//! **The document's own note is the one downward move, and it is a ruling.** #113 ruled that a
//! mapping's note whose block ends inside a nested sequence takes the node that ends the block as
//! its owner, and that no new emission position should be added for it; (eo) and (ep) recorded the
//! same for the tail slot — the ownership stays, the text settles. The writer therefore leaves the
//! document's note on the last line of the body, and the reader reports it on whichever node that
//! line ends: the last member for `z: 2`, the flow sequence itself for a `]  # note` line. Measured
//! 2026-11-18 over the corpus, three seeds (`former-crash-200342d3`, `-2eaa5599`, `-c2343438`) place
//! it that way. `the_roots_own_note_settles_in_one_round` and
//! `a_key_note_over_a_noted_flow_body_settles_in_one_round` pin the emission. This gate accepts
//! exactly the routes on the body's final line — the last child at each level down from the root,
//! which is what the tail slot can be riding — and nothing else, so the document's note reaching any
//! other branch is still a failure.
//!
//! **The conservation half is here; the YAML spokes' settlement is charged here too.**
//! `crates/pyrs-toml/tests/roundtrip_corpus.rs` and its JSON twin replay their format's corpora for
//! the fixed point on every `cargo nextest` - both TOML corpora, and all three JSON corpora
//! (`json_roundtrip`, `parse_json` and `json_events_replay`, in the three compact and the three
//! pretty writer shapes; `events_replay.rs` compares the two walks of one input, which is not a
//! fixed point) - and this file charges all three YAML spokes
//! for it as well — every seed that parses, note-bearing or not, because a note-less input was the
//! shape that escaped: crash-b3c57280's 20 bytes carry no note, so the conservation branch skipped
//! them before settlement was ever asked. The other two spokes need the charge rather than inherit
//! it: `edit_yaml`'s documents reach a writer only through its fuzz target and through
//! `edit_scripts.rs`, which replays the scripts but asks the fixed point of the default shape
//! alone, and `parse_yaml`'s target asks nothing of a writer. Measured 2026-11-18, 161 + 69 + 4 =
//! 234 seeds parse and settle. That is not the same claim as conservation: an emission that *drops*
//! a note settles immediately, so only charging the tree's notes against the text sees the loss — the
//! failure mode `(hw)` and `(hy)` name for TOML, where a lost line passed re-parseability, the fixed
//! point and tree equality all at once.
//!
//! **Settlement is charged in eight writer shapes, not only the default.** The indent triplet
//! (`indent4`), `indent_offset` (`offset2`), the explicit markers (`explicit`), folding at 20 columns
//! and folding off (`width20`, `width0`), `sort_keys` (`sorted`) and sorting combined with a wider
//! indent and folding (`sorted-indent4-width20`) are all published option sets, and each is its own
//! branch in the writer. The first run of this charge found four defects the default shape hid: a
//! compact sequence item whose second pair sat at `indent_sequence` instead of the column after
//! `- `, which the reader refused at `indent_sequence: 4`; a hoisted note written below its header
//! once `indent_offset` moved both lines off column zero; `sort_keys` emitting an alias before the
//! `&name` it uses, which left the emission unreadable; and the sorted emission's preamble lift
//! reading the source's first pair while the writer wrote a different one first. Measured 2026-11-18
//! after the fixes, every shape settles on all 234 seeds. `note_placement: header_first` stays out
//! of the table because it is documented as *not* settling (`docs/en/features.md`): 17 of the 234
//! seeds move a note across its header on the second round, so charging it would encode a documented
//! non-fixed-point as the expected answer.
//!
//! **Conservation rides the same loop, because settling says nothing about keeping a note.** An
//! emission that drops a note is a fixed point on its very next round, so a shape that settles can
//! still be the shape that silently loses one — and asking conservation only of the default emission
//! left the other seven writer branches unpriced on the axis this file exists for. The note-bearing
//! seeds are therefore asked again in each shape, against the tree their own emission re-reads, and
//! every shape keeps every note on its node or an ancestor. Priced by making `write_note_line` return
//! early when `sort_keys` is set: the two sorted rows redden
//! (`anchor-tag-comment.seed [sorted]: "" lost "standalone leading"`) while the default row stays
//! green, so the widened charge is what sees the loss.
//!
//! **Not charged here, and why.** The three JSON corpora (`json_roundtrip` 17 seeds,
//! `json_events_replay` 22, `parse_json` 7) share one byte string across `from_json`, `from_jsonc`
//! and `from_json5`, and only the latter two record a note — but strict `to_json_text` has nowhere to
//! put it, so a conservation assertion written against that emission would fail by design. That half
//! of the invariant is not missing, it is charged where the emission can carry the note:
//! `crates/pyrs-json/tests/corpus_notes.rs` walks the committed JSON corpora through both dialects in
//! both shapes and compares placed notes against `to_jsonc_text` / `to_json5_text`, so a note the
//! JSONC writer drops or re-homes is a red run there rather than a gap here. `parse_schema`'s seeds
//! are schema documents, and a schema has no comment vocabulary at all.
//!
//! **What this gate cannot see, stated because a green check invites trust in it.**
//! The measure is the note the *reader recorded*, so a note lost during ingest is
//! invisible: the AST never had it, and nothing is then demanded of the output.
//! Proven by mutation, not by argument — making `attach_inline_comment` report
//! success without attaching anything (the defect that dropped `!x # note`, crash-
//! ce106ccc) reddens `a_note_beside_a_property_only_root_survives_and_settles` and
//! leaves this file **green**. An ingest-side gate would have to count comments in
//! the source text, which cannot be done reliably here: a `#` is a comment only
//! outside quotes, outside a block-scalar body and outside a tag token, and the
//! corpus deliberately contains `!###0 …` — a tag whose suffix is `#` characters.
//! So this file is the writer-side half of the invariant and the fuzz tier plus the
//! pinned per-shape tests are the reader-side half; neither is claimed to cover the
//! other.

use std::path::{Path, PathBuf};

use pyrs_toml::{from_toml, to_toml};
use pyrs_yaml_core::ast::CustomNode;
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::serializer::{SIMPLE_KEY_WINDOW, SerializeOptions, to_yaml_with_options};

/// The path segment a mapping key contributes. The key and its value share it: a
/// note on either names the member, not the slot it was stored in.
fn key_segment(key: &CustomNode) -> String {
    match key {
        CustomNode::Scalar { value, .. } => value.to_string(),
        CustomNode::Null { .. } => "null".to_string(),
        _ => "?".to_string(),
    }
}

/// Every note `node`'s subtree carries, as `(route, text)`.
///
/// Read through `comment()` / `leading_comments()` rather than the raw fields, because a note lives
/// in one of two storage conventions (`decor.leading_comments` or a legacy `comment { standalone:
/// true }`) and the accessors normalise across both, which is why the writers cannot miss a stack.
/// The inline slot is filtered to `!standalone` because `leading_comments()` already answers a legacy
/// standalone note as a one-element decor slice — unfiltered, the same note would be counted twice,
/// and the multisets would be comparing storage rather than notes.
///
/// Text is flattened by the rule both writers document — outer blanks dropped, every line break
/// written as a space — on both sides of the comparison, so a note the writer re-lays over several
/// `# ` lines compares equal to the one-line text the reader stored. Blanks *inside* a line are
/// compared exactly, because neither writer touches them; the rule here used to collapse every
/// whitespace run, which is looser than that contract, and it was retired after a mutation that made
/// each writer's note flattener collapse runs: the collapsing rule stayed green on both corpora
/// while this one reddened on `former-crash-2a5ea394` (TOML) and on the YAML seeds whose notes carry
/// a tab or a double space. A note with no text has nothing to place and is dropped.
fn note_sites(node: &CustomNode, path: &str, into: &mut Vec<(String, String)>) {
    let mut own = Vec::new();
    if let Some(c) = node.comment().filter(|c| !c.standalone) {
        own.push(c.text.to_string());
    }
    for c in node.leading_comments() {
        own.push(c.text.to_string());
    }
    for text in own {
        let flat = text.trim().replace(['\n', '\r'], " ");
        if !flat.is_empty() {
            into.push((path.to_string(), flat));
        }
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs.iter() {
                let child = format!("{path}/{}", key_segment(key));
                note_sites(key, &child, into);
                note_sites(value, &child, into);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for (index, item) in items.iter().enumerate() {
                note_sites(item, &format!("{path}[{index}]"), into);
            }
        }
        _ => {}
    }
}

/// True when `ancestor` is `node` or a strict ancestor of it. The root (`""`) is an
/// ancestor of everything; a segment boundary is required, so `["a/b1"]` is not a
/// descendant of `["a/b"]`.
fn is_ancestor_or_self(ancestor: &str, node: &str) -> bool {
    if ancestor == node || ancestor.is_empty() {
        return true;
    }
    node.strip_prefix(ancestor)
        .is_some_and(|rest| rest.starts_with('/') || rest.starts_with('['))
}

/// Routes a document's own note may come back on: the root, then the last member or item at each
/// level below it. #113 ruled that the note's owner moves to the node that ends the body's block,
/// and which route that is depends on the tail line's shape — a scalar tail's own line names the
/// member, a `]  # note` line names the flow container — so the gate accepts this chain rather
/// than guessing at columns the route walk does not carry.
fn body_tail_routes(node: &CustomNode, path: &str, into: &mut Vec<String>) {
    into.push(path.to_string());
    match node {
        CustomNode::Mapping { pairs, .. } => {
            if let Some((key, value)) = pairs.iter().next_back() {
                body_tail_routes(value, &format!("{path}/{}", key_segment(key)), into);
            }
        }
        CustomNode::Sequence { items, .. } => {
            if let Some((index, item)) = items.iter().enumerate().next_back() {
                body_tail_routes(item, &format!("{path}[{index}]"), into);
            }
        }
        _ => {}
    }
}

/// Match each carried note against the text one emission re-read, and describe every note that did
/// not come back on the node that carried it or on an ancestor of it.
///
/// Deciding this is a bipartite matching, not a per-note scan: a deep note and a shallow note can
/// both claim the same ancestor's copy, and a greedy pass that takes the first candidate reported
/// three notes lost on `former-crash-e1e921af` where the emission had moved none of them — its
/// `/?/?/1X` note came back on `/?`, and a greedy pass had already spent `/?`'s text on `/?/?`.
/// Kuhn's augmenting-path search answers it exactly, and the corpus is far too small to care that
/// a general maximum matching is more than a walk would need.
///
/// `body_tail` is [`body_tail_routes`] of the re-read tree, and it is consulted only for a note the
/// re-read tree holds on the root: that note's owner is the ruled downward move (module doc), and a
/// note the root did *not* carry gains nothing by it.
fn stray_notes(
    carried: &[(String, String)],
    reread: &[(String, String)],
    body_tail: &[String],
) -> Vec<String> {
    fn augment(
        index: usize,
        carried: &[(String, String)],
        reread: &[(String, String)],
        body_tail: &[String],
        owner: &mut [usize],
        seen: &mut [bool],
    ) -> bool {
        for candidate in 0..reread.len() {
            if seen[candidate] {
                continue;
            }
            let (carried_path, carried_text) = &carried[index];
            let (reread_path, reread_text) = &reread[candidate];
            let on_tail =
                carried_path.is_empty() && body_tail.iter().any(|route| route == reread_path);
            if reread_text != carried_text
                || !(is_ancestor_or_self(reread_path, carried_path) || on_tail)
            {
                continue;
            }
            seen[candidate] = true;
            if owner[candidate] == usize::MAX
                || augment(owner[candidate], carried, reread, body_tail, owner, seen)
            {
                owner[candidate] = index;
                return true;
            }
        }
        false
    }

    // Deepest first: the deepest demand has the fewest ancestors, so it should pick first.
    let mut order = (0..carried.len()).collect::<Vec<_>>();
    order.sort_by(|&a, &b| carried[b].0.len().cmp(&carried[a].0.len()));
    let mut owner = vec![usize::MAX; reread.len()];
    for index in order {
        let mut seen = vec![false; reread.len()];
        augment(index, carried, reread, body_tail, &mut owner, &mut seen);
    }

    let mut stray = Vec::new();
    for (index, (path, text)) in carried.iter().enumerate() {
        if owner.contains(&index) {
            continue;
        }
        let seen = reread
            .iter()
            .filter(|(_, t)| t == text)
            .map(|(p, _)| p.as_str())
            .collect::<Vec<_>>();
        stray.push(format!("{path:?} lost {text:?} (re-read on {seen:?})"));
    }
    for (candidate, (path, text)) in reread.iter().enumerate() {
        if owner[candidate] == usize::MAX {
            stray.push(format!("{path:?} gained {text:?}"));
        }
    }
    stray
}

/// A mapping pair whose key cannot take the simple `key: value` spelling. Classified
/// from the AST, never from a seed name: the writer takes the `?`-marker branch for a
/// collection or alias key, and for a scalar/null key whose anchor, tag or length puts
/// its colon past the reader's `SIMPLE_KEY_WINDOW`. Counting the shape rather than the
/// filename means a renamed seed cannot silently leave this census while still reaching
/// the writer branch it exists to cover.
fn explicit_key_pair(key: &CustomNode) -> bool {
    match key {
        CustomNode::Mapping { .. } | CustomNode::Sequence { .. } | CustomNode::Alias { .. } => true,
        CustomNode::Scalar { value, meta, .. } => {
            meta.anchor().is_some()
                || meta.tag().is_some()
                || value.chars().count() > SIMPLE_KEY_WINDOW
        }
        CustomNode::Null { meta, .. } => meta.anchor().is_some() || meta.tag().is_some(),
    }
}

/// Count every explicit-key pair in a node's subtree, keys and values included.
fn count_explicit_keys(node: &CustomNode, into: &mut usize) {
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs {
                if explicit_key_pair(key) {
                    *into += 1;
                }
                count_explicit_keys(key, into);
                count_explicit_keys(value, into);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                count_explicit_keys(item, into);
            }
        }
        _ => {}
    }
}

fn seed_dirs() -> Vec<(&'static str, PathBuf)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds");
    ["yaml_roundtrip", "parse_yaml", "edit_yaml"]
        .iter()
        .map(|name| (*name, root.join(name)))
        .filter(|(_, dir)| dir.is_dir())
        .collect()
}

/// The bytes of one seed that are a *document*.
///
/// `edit_yaml` entries are `document NUL script`: the fuzzer splits them the same way, so charging
/// the whole buffer would ask the YAML parser to read a script as content and reject the input,
/// which the loop above would then skip. Measured 2026-11-18: all 69 seeds hold a NUL, **none** of
/// the 69 parses whole, and cutting at the first NUL yields 69 documents — an unsplit walk would not
/// have failed, it would have found nothing, which is the worse outcome for a coverage gate.
fn document_bytes<'a>(dir: &Path, raw: &'a [u8]) -> &'a [u8] {
    if dir.file_name().is_some_and(|name| name == "edit_yaml") {
        let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
        &raw[..end]
    } else {
        raw
    }
}

/// The writer shapes the settlement charge runs over.
///
/// Each entry is a published option set, not a combination invented here: the indent triplet, the
/// offset, the explicit markers, the folding width (0 = fold nothing) and `sort_keys`. A shape the
/// corpus is never emitted under is a writer branch no fixed-point assertion covers — which is what
/// the first run of this charge demonstrated, four times over (module doc). The names are also the
/// keys of the per-shape floors below, so a shape dropped from this table reads as zero there.
fn settlement_shapes() -> Vec<(&'static str, SerializeOptions)> {
    vec![
        ("default", SerializeOptions::default()),
        (
            "indent4",
            SerializeOptions {
                indent_size: 4,
                indent_mapping: 4,
                indent_sequence: 4,
                ..Default::default()
            },
        ),
        (
            "offset2",
            SerializeOptions {
                indent_offset: 2,
                ..Default::default()
            },
        ),
        (
            "explicit",
            SerializeOptions {
                explicit_start: true,
                explicit_end: true,
                ..Default::default()
            },
        ),
        (
            "width20",
            SerializeOptions {
                width: 20,
                ..Default::default()
            },
        ),
        (
            "width0",
            SerializeOptions {
                width: 0,
                ..Default::default()
            },
        ),
        (
            "sorted",
            SerializeOptions {
                sort_keys: true,
                ..Default::default()
            },
        ),
        (
            "sorted-indent4-width20",
            SerializeOptions {
                sort_keys: true,
                indent_size: 4,
                indent_mapping: 4,
                indent_sequence: 4,
                width: 20,
                ..Default::default()
            },
        ),
    ]
}

#[test]
fn every_note_in_the_seed_corpus_survives_one_emission() {
    let shapes = settlement_shapes();
    let mut settled = vec![0usize; shapes.len()];
    let mut noted = Vec::<(&str, usize)>::new();
    let mut failures = Vec::new();
    let mut explicit_key_seeds = 0usize;
    let mut explicit_key_pairs = 0usize;

    for (name, dir) in seed_dirs() {
        // Every spoke is charged, not only the one whose target asks for it: `edit_yaml`'s
        // deterministic replay (`edit_scripts.rs`) drives the scripts but asks the fixed point of
        // the default shape alone; `parse_yaml`'s target asks only panic-freedom and refusal
        // parity, so its parsed seeds are asked for nothing. Charging all three in every shape
        // keeps a writer regression from sitting in a corpus that `cargo nextest` reads without
        // ever asking the writer anything.
        let mut entries = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", dir.display()))
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "seed"))
            .collect::<Vec<_>>();
        entries.sort();
        let mut noted_here = 0usize;
        for path in entries {
            let raw = std::fs::read(&path).expect("seed must be readable");
            let Ok(src) = std::str::from_utf8(document_bytes(&dir, &raw)) else {
                continue; // rejected before any note exists
            };
            let Ok(node) = parse(src, Schema::Core) else {
                continue; // a typed rejection has no AST to lose notes from
            };
            let mut carried = Vec::new();
            note_sites(&node, "", &mut carried);
            if !carried.is_empty() {
                noted_here += 1;
            }
            // A second, independent census of the corpus: how many documents actually drive
            // the writer's explicit-key branch. This is a count, not a second placement
            // assertion - the settlement loop below still asks every seed the same question.
            let mut pairs_here = 0usize;
            count_explicit_keys(&node, &mut pairs_here);
            if pairs_here > 0 {
                explicit_key_seeds += 1;
            }
            explicit_key_pairs += pairs_here;
            // Settlement is asked of every seed that parsed, not only the note-bearing ones:
            // the assertion the fuzz tier makes by replaying these files is a property of the
            // input, and crash-b3c57280 (no note anywhere) is the shape that escaped when it
            // sat behind the conservation branch. Conservation rides the same loop, because a
            // shape that settles can still drop a note — a dropped note is a fixed point
            // immediately — so asking only the default emission leaves seven writer branches
            // unpriced on the axis this file exists for.
            for (index, (shape, options)) in shapes.iter().enumerate() {
                let emitted = to_yaml_with_options(&node, options)
                    .unwrap_or_else(|e| panic!("{}: {shape} must serialize: {e}", path.display()));
                settled[index] += 1;
                let again_tree = parse(&emitted, Schema::Core).unwrap_or_else(|e| {
                    panic!(
                        "{}: {shape} emitted text must re-parse: {e}",
                        path.display()
                    )
                });
                let again = to_yaml_with_options(&again_tree, options)
                    .unwrap_or_else(|e| panic!("{}: {shape} re-serialize: {e}", path.display()));
                if again != emitted {
                    failures.push(format!(
                        "{} [{shape}]: not a one-round fixed point: {emitted:?} -> {again:?}",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                    ));
                }
                if carried.is_empty() {
                    continue;
                }
                let mut reread = Vec::new();
                note_sites(&again_tree, "", &mut reread);
                let mut body_tail = Vec::new();
                body_tail_routes(&again_tree, "", &mut body_tail);
                let stray = stray_notes(&carried, &reread, &body_tail);
                if !stray.is_empty() {
                    failures.push(format!(
                        "{} [{shape}]: {} from {src:?}, emitted {emitted:?}",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                        stray.join("; "),
                    ));
                }
            }
        }
        noted.push((name, noted_here));
    }

    // Explicit-key coverage. Measured 2026-11-27: 103 seeds / 203 pairs across the three YAML
    // spokes (the two #808 seeds included). The floor is 103 - 10 = 93, slack for retirements;
    // it must stay well above zero so a corpus that stopped carrying explicit keys cannot pass.
    println!("explicit-key seeds: {explicit_key_seeds}");
    println!("explicit-key pairs: {explicit_key_pairs}");
    assert!(
        explicit_key_seeds >= 93,
        "expected explicit-key coverage, got {explicit_key_seeds} seeds / {explicit_key_pairs} pairs"
    );
    assert!(
        failures.is_empty(),
        "note placement broken over the committed corpus:\n{}",
        failures.join("\n")
    );
    // The corpus must actually cover the invariant: a vacuous pass would be worse
    // than no test, because it looks like coverage. Measured on 2026-11-18, the three
    // YAML-spoke corpora carry notes on 120 + 2 + 1 seeds; each floor is that count with
    // slack for inputs being retired, and a reading below it means the corpus lost its
    // comments rather than that the assertion is merely unused.
    for (name, floor) in [("yaml_roundtrip", 116), ("parse_yaml", 2), ("edit_yaml", 1)] {
        let got = noted
            .iter()
            .find(|(spoke, _)| *spoke == name)
            .map_or(0, |(_, count)| *count);
        assert!(
            got >= floor,
            "expected the {name} corpus to carry notes for at least {floor} inputs, got {got}"
        );
    }
    // Measured on 2026-11-18, 234 seeds across the three YAML spokes parse (161 `yaml_roundtrip`
    // + 69 `edit_yaml` document halves + 4 `parse_yaml`), and every shape above settles on all 234.
    // The floor is per shape and keeps the 215 slack for retirements; a name absent from the table
    // reads as zero, so dropping a shape there reddens here instead of thinning the charge.
    for (shape, floor) in [
        ("default", 215),
        ("indent4", 215),
        ("offset2", 215),
        ("explicit", 215),
        ("width20", 215),
        ("width0", 215),
        ("sorted", 215),
        ("sorted-indent4-width20", 215),
    ] {
        let got = shapes
            .iter()
            .position(|(name, _)| *name == shape)
            .map_or(0, |index| settled[index]);
        assert!(
            got >= floor,
            "expected the {shape} shape to settle at least {floor} seeds, got {got}"
        );
    }
}

/// The same invariant on the TOML spoke, asked for exact placement.
///
/// TOML can answer the exact question because the reader re-attributes a note on a member's line to
/// that member, so a writer that leaves it there keeps the route. `crates/pyrs-toml/tests/
/// roundtrip_corpus.rs` already replays these files for the fixed point on every `cargo nextest`,
/// and that is the assertion the TOML fuzz target makes — but a writer that *deletes* a note reaches
/// its fixed point on the first round, so settlement cannot see the loss. `(hy)` measured the shape:
/// `# K` above a section that carries `# T` emitted `# T\n[a]\nx = 1\n` with the key's line gone, and
/// re-parseability, the fixed point and tree equality all passed on a document that had lost
/// something. The property tier catches that class over *generated* trees
/// (`prop_toml_writer_carries_every_note`), which is a draw, not a replay — this charges the corpus,
/// so every crash the fuzzer already surrendered is asked again on every run.
///
/// Exactness is the placement half: the multiset of `(route, text)` the tree carries must equal the
/// multiset the emission re-reads, in both directions. Presence alone would accept the defect this
/// reading found — an element's note written after its separator, so the *next* element read it as
/// its leading note — because the text was still there.
#[test]
fn every_note_in_the_toml_corpus_survives_one_emission() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds");
    let mut noted = Vec::<(&str, usize)>::new();
    let mut failures = Vec::new();
    for name in ["toml_roundtrip", "parse_toml"] {
        let dir = root.join(name);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut paths = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|p| p.extension().is_some_and(|ext| ext == "seed"))
            .collect::<Vec<_>>();
        paths.sort();
        let mut noted_here = 0usize;
        for path in paths {
            let Ok(raw) = std::fs::read(&path) else {
                continue;
            };
            let Ok(src) = std::str::from_utf8(&raw) else {
                continue;
            };
            let Ok(node) = from_toml(src) else {
                continue;
            };
            let mut carried = Vec::new();
            note_sites(&node, "", &mut carried);
            if carried.is_empty() {
                continue;
            }
            noted_here += 1;
            let Ok(emitted) = to_toml(&node) else {
                failures.push(format!(
                    "{}: the writer refused a tree its own reader produced",
                    path.display()
                ));
                continue;
            };
            let Ok(again_tree) = from_toml(&emitted) else {
                failures.push(format!(
                    "{}: emitted text must re-read, got {emitted:?}",
                    path.display()
                ));
                continue;
            };
            let mut reread = Vec::new();
            note_sites(&again_tree, "", &mut reread);
            let mut carried_sorted = carried.clone();
            carried_sorted.sort();
            let mut reread_sorted = reread.clone();
            reread_sorted.sort();
            if carried_sorted != reread_sorted {
                let lost = carried_sorted
                    .iter()
                    .filter(|site| !reread_sorted.contains(site))
                    .collect::<Vec<_>>();
                let gained = reread_sorted
                    .iter()
                    .filter(|site| !carried_sorted.contains(site))
                    .collect::<Vec<_>>();
                failures.push(format!(
                    "{}: lost {lost:?}, gained {gained:?} from {src:?}, emitted {emitted:?}",
                    path.display()
                ));
            }
        }
        noted.push((name, noted_here));
    }
    assert!(
        failures.is_empty(),
        "TOML note placement broken over the committed corpus:\n{}",
        failures.join("\n")
    );
    // Measured on 2026-11-18: 11 + 3 note-bearing seeds across the two TOML corpora, every one
    // placed exactly. The floor sits below that so retiring a seed does not redden the tier, and
    // well above zero so a corpus that stopped carrying notes cannot pass.
    for (name, floor) in [("toml_roundtrip", 10), ("parse_toml", 3)] {
        let got = noted
            .iter()
            .find(|(spoke, _)| *spoke == name)
            .map_or(0, |(_, count)| *count);
        assert!(
            got >= floor,
            "expected the {name} corpus to carry notes for at least {floor} inputs, got {got}"
        );
    }
}
