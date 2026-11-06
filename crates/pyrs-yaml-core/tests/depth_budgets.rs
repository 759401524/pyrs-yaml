//! Where the library's depth budgets disagree, and what each side answers there.
//!
//! Two numbers describe one pipeline. The YAML writer counts nesting against
//! `SerializeOptions::max_depth`, which defaults to 1000 and is the number the readers advertise
//! too. The YAML *reader* counts against granit's `block_nesting_limit` and `flow_nesting_limit`,
//! both 255 in the `Options::default()` this repository passes to `Parser::new_from_str`, and both
//! counted per container. Between 256 and 999 containers the two therefore disagree, and the
//! disagreement is not symmetric: the writer says nothing is wrong, and the reader refuses text this
//! library produced a moment earlier. Measured here, a 300-level block chain emits 300 lines that
//! `parse` answers `recursion limit exceeded`, while 254 round-trips; the flow shapes answer the
//! same way, at the same container count, which is why `[{a: …}]` crosses it at 128 levels.
//!
//! These assert today's answer, not the wanted one. The remedies and their costs are recorded in
//! `docs/dev/quality-ledger.md` (jq), (jr) and (js) as an open decision; when one of them lands,
//! invert each assertion as its own comment says.

use pyrs_yaml_core::ast::{CustomNode, NodeMap};
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::serializer::{SerializeOptions, to_yaml_with_options};

/// A block mapping nested `depth` levels deep, with a scalar at the bottom.
fn chain(depth: usize) -> CustomNode {
    let mut node = CustomNode::plain_scalar("leaf");
    for _ in 0..depth {
        let mut pairs = NodeMap::default();
        pairs.insert(CustomNode::plain_scalar("k"), node);
        node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
    }
    node
}

fn emit(depth: usize) -> Result<String, String> {
    to_yaml_with_options(&chain(depth), &SerializeOptions::default()).map_err(|e| e.to_string())
}

/// A sequence wrapped in a mapping per level, so each level spends two containers: `[{k: [ … ]}]`.
fn alternating(levels: usize) -> CustomNode {
    let mut node = CustomNode::plain_scalar("leaf");
    for _ in 0..levels {
        let mut pairs = NodeMap::default();
        pairs.insert(CustomNode::plain_scalar("k"), node);
        node = CustomNode::Sequence {
            items: vec![CustomNode::Mapping {
                pairs,
                flow_style: true,
                meta: Default::default(),
            }],
            flow_style: true,
            meta: Default::default(),
        };
    }
    node
}

/// Which unit the writer counts, measured at a cap where the two readings answer differently.
///
/// Under `max_depth = 40` a shape that spends two containers per level refuses at 20 levels - 40
/// containers - where counting levels would still have 20 of 40 to spare, so the writer's budget is
/// the reader's unit and only its number differs.
#[test]
fn the_writer_counts_containers_not_the_lines_that_spell_them() {
    let capped = SerializeOptions {
        max_depth: 40,
        ..Default::default()
    };
    let inside = to_yaml_with_options(&alternating(19), &capped)
        .expect("19 levels spend 38 of the 40 the cap names");
    assert!(!inside.is_empty(), "the emission is text, not a refusal");

    let error = to_yaml_with_options(&alternating(20), &capped)
        .expect_err("20 levels spend 40 containers, which is the cap")
        .to_string();
    assert!(
        error.contains("max depth exceeded"),
        "the writer counts levels rather than containers, so it does not share the reader's rule: \
         {error}"
    );
}

/// The zone where both budgets agree is a working round trip, in one round.
#[test]
fn a_tree_inside_the_readers_nesting_limit_round_trips() {
    let one = emit(254).expect("254 levels is inside both budgets");
    let again = parse(&one, Schema::Core).expect("and the reader accepts what the writer emitted");
    let two = to_yaml_with_options(&again, &SerializeOptions::default()).expect("re-emittable");
    assert_eq!(one, two, "not a fixed point inside the agreed zone");
}

/// The disagreement itself: the writer is silent, the reader refuses its own output.
#[test]
fn the_yaml_writer_emits_text_the_yaml_reader_refuses() {
    let emitted = emit(300).expect("300 levels is inside the writer's 1000");
    let error = parse(&emitted, Schema::Core).expect_err(
        "the reader refuses text this library just emitted (255 is granit's default limit)",
    );
    let message = error.to_string();
    assert!(
        message.contains("recursion limit"),
        "the refusal changed shape, so the boundary below is no longer the interesting one: {message}"
    );
    // If a decision raises granit's two nesting limits to the advertised 1000, this assertion
    // inverts: `parse` accepts, and the round trip holds at 300 too.
}

/// granit counts containers, in both styles: `flow_nesting_limit` answers the same way as
/// `block_nesting_limit`, so a shape that spends two containers per level crosses the limit at half
/// the depth - 255 for one container per level, 127 for two.
#[test]
fn the_reader_stops_at_255_containers_whichever_style_spells_them() {
    for (text, depth, expected) in [
        (flow_mapping(255), 255, true),
        (flow_mapping(256), 256, false),
        (flow_alternating(127), 127, true),
        (flow_alternating(128), 128, false),
    ] {
        let answered = parse(&text, Schema::Core);
        if expected {
            assert!(
                answered.is_ok(),
                "{depth} levels should read inside granit's 255-container limit"
            );
        } else {
            let message = answered
                .expect_err("levels past the container limit are refused")
                .to_string();
            assert!(
                message.contains("recursion limit"),
                "the flow refusal changed shape, so the counting rule above is no longer measured: \
                 {message}"
            );
        }
    }
}

fn flow_mapping(levels: usize) -> String {
    let mut text = String::new();
    for _ in 0..levels {
        text.push_str("{a: ");
    }
    text.push('1');
    for _ in 0..levels {
        text.push('}');
    }
    text
}

fn flow_alternating(levels: usize) -> String {
    let mut text = String::new();
    for _ in 0..levels {
        text.push_str("[{a: ");
    }
    text.push('1');
    for _ in 0..levels {
        text.push_str("}]");
    }
    text
}

/// At a cap the writer answers in type, which is what the Python-facing conversions propagate
/// instead of the `expect` the infallible wrapper still carries.
///
/// The cap is asked through the option rather than at its default of 1000: a chain 999 levels deep
/// was tried here first and overflowed the test thread's stack in a debug build
/// (`STATUS_STACK_OVERFLOW`, 0xc00000fd) - the same shape quality-ledger (ih) records for TOML, where
/// the ruling kept the 1000 budget and accepted that a small-stack thread cannot carry it. A
/// release build on the interpreter's main thread does serve 999 (measured from Python:
/// 1,000,001 bytes out, and `set` refuses to grow past it with a typed `max depth exceeded`).
#[test]
fn the_yaml_writer_refuses_in_type_at_its_cap() {
    let deep = to_yaml_with_options(&chain(300), &SerializeOptions::default())
        .expect("300 levels fits the default cap, even though the reader cannot take it back");
    assert!(deep.ends_with('\n'), "the emission is text, not a panic");

    let optioned = SerializeOptions {
        max_depth: 50,
        ..Default::default()
    };
    let error = to_yaml_with_options(&chain(60), &optioned)
        .expect_err("a tree over the option's cap refuses");
    assert!(
        error.to_string().contains("max depth exceeded"),
        "expected the depth refusal, got {error}"
    );
}
