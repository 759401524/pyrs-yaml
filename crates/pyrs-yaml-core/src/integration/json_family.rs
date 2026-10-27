//! Engine-level cross-format tests for the JSON family. The `pyrs-json`
//! crate owns the JSON/JSONC/JSON5 grammar itself; scenarios that start from
//! YAML source (so they need this crate's parser) live here instead.

use crate::ast::CustomNode;
use crate::error::SerializeError;
use crate::parser::{parse, yaml::Schema};
use pyrs_json::{to_json_text, to_json_text_pretty, to_json5_text, to_jsonc_text_pretty};

#[test]
fn jsonc_preserves_root_leading_comment() {
    // PR #122: a document-level standalone comment attaches to the
    // root container's `leading_comment` slot. Nested members emit
    // theirs via the pair loop; the outermost node has no preceding
    // key, so `emit_root_leading` is what keeps it from dropping.
    let ast = parse("# header\nport: 8080\n", Schema::Core).unwrap();
    let out = to_jsonc_text_pretty(&ast, 2).unwrap();
    assert!(out.starts_with("// header\n"), "{out}");
    assert!(out.contains("\"port\": 8080"), "{out}");
}

#[test]
fn yaml_nodes_project_like_the_serde_json_path() {
    let node = parse(
        "s: hello\nn: 42\nf: 1.5\nb: true\nq: \"52\"\nnl: ~\nlist: [1, two]\n",
        Schema::Core,
    )
    .unwrap();
    let text = to_json_text_pretty(&node, 2).unwrap();
    assert!(text.contains("\"s\": \"hello\""), "{text}");
    assert!(text.contains("\"n\": 42"), "{text}");
    assert!(text.contains("\"f\": 1.5"), "{text}");
    assert!(text.contains("\"b\": true"), "{text}");
    assert!(text.contains("\"q\": \"52\""), "{text}"); // quoted stays string
    assert!(text.contains("\"nl\": null"), "{text}");
    assert!(text.contains("\"two\""), "{text}");
}

#[test]
fn exotic_scalars_quote_or_normalize() {
    let node = parse("a: 0x1F\nc: !!str 7\n", Schema::Core).unwrap();
    let text = to_json_text(&node).unwrap();
    assert!(text.contains("\"a\":31"), "{text}"); // hex normalizes
    // A `!!str` tag states the type, so the text `7` is the string "7" (YAML 1.2 §6.1). This line
    // used to assert `"c":7`, matching the historical serde_json projection that read the text and
    // ignored the tag - but the loader has resolved this same document to `{'c': '7'}` since #335,
    // so reader and bridge disagreed about one AST. That disagreement was issue #340; the
    // assertion follows the tag, which is what both reference libraries do too.
    assert!(text.contains("\"c\":\"7\""), "{text}");

    // A non-finite float has no JSON literal, so the projection refuses it instead of quoting it into
    // a string: `"b":".inf"` used to come back from `load_json` as text, indistinguishable from a
    // document whose value really was that string. JSON5 owns the token, so the same node still
    // projects losslessly there.
    let infinity = parse("b: .inf\n", Schema::Core).unwrap();
    let err = to_json_text(&infinity).unwrap_err();
    assert!(
        format!("{err:?}").contains("json-cannot-represent-non-finite"),
        "{err:?}"
    );
    let json5 = to_json5_text(&infinity).unwrap();
    assert!(json5.contains("Infinity"), "{json5}");
    assert!(!json5.contains("\"Infinity\""), "{json5} quoted a number");
}

#[test]
fn a_key_pair_that_collapses_without_its_tag_is_refused_not_overwritten() {
    // The reader accepts this document: a tag is part of a key's identity in YAML, so `c` and `!A c`
    // are two keys. A JSON object has no such slot - both spell `"c"` - and the writer used to emit
    // `{"c": 0, "c": "!"}`, which every consumer resolves by dropping one entry: measured,
    // `json.loads` of that text gives `{'c': '!'}`. A silent overwrite is the one outcome the
    // projection must not have, so the pair is refused where it is written.
    let node = parse("c: 0\n!A c: \"!\"\n", Schema::Core).unwrap();
    assert!(matches!(
        to_json_text(&node),
        Err(SerializeError::UnsupportedValue(
            "json-object-keys-collide-without-metadata"
        ))
    ));
    // The same emitter answers for JSONC and JSON5 too, so the refusal has to reach all three.
    assert!(to_jsonc_text_pretty(&node, 2).is_err());
    assert!(to_json5_text(&node).is_err());

    // A tag on a key the object can still name uniquely is not the writer's business: the refusal is
    // about the collision, not about tags.
    let fine = parse("c: 0\n!A d: \"!\"\n", Schema::Core).unwrap();
    assert_eq!(to_json_text(&fine).unwrap(), "{\"c\":0,\"d\":\"!\"}");
}

#[test]
fn alias_and_non_scalar_keys_are_stable_errors() {
    let node = parse("a: &x 1\nb: *x\n", Schema::Core).unwrap();
    let CustomNode::Mapping { pairs, .. } = &node else {
        unreachable!()
    };
    // resolve alias manually is out of scope; the alias node errors
    let (_, b_val) = pairs.get_index(1).unwrap();
    let err = to_json_text(b_val).unwrap_err();
    assert!(format!("{err:?}").contains("json-cannot-represent-alias"));
    // a sequence key cannot be a JSON object key
    let keyed = CustomNode::Mapping {
        pairs: [(
            CustomNode::plain_sequence(vec![CustomNode::plain_scalar("k")]),
            CustomNode::plain_scalar("v"),
        )]
        .into_iter()
        .collect(),
        flow_style: false,
        meta: Default::default(),
    };
    assert!(matches!(
        to_json_text(&keyed),
        Err(SerializeError::UnsupportedValue("json-object-key"))
    ));
}
