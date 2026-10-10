#[cfg(test)]
#[allow(unused_doc_comments)]
mod tests {
    use crate::ast::proptest_strategies::*;
    use crate::parser::parse_with_options;
    use crate::parser::yaml::Schema;
    use crate::serializer::{SerializeOptions, to_yaml, to_yaml_with_options};
    use proptest::prelude::*;

    /// When a round-trip comparison fails, print a unified diff of the two
    /// YAML serializations so the developer can see exactly what changed.
    fn yaml_diff(a: &str, b: &str) -> String {
        let diff = similar::TextDiff::from_lines(a, b);
        diff.unified_diff()
            .context_radius(3)
            .header("original.yaml", "re-parsed.yaml")
            .to_string()
    }

    fn try_roundtrip(node: &crate::ast::CustomNode) -> Option<crate::ast::CustomNode> {
        let yaml = to_yaml(node);
        let parsed = parse_with_options(&yaml, true, Schema::Core, 1000, false).ok()?;
        if nodes_equal_ignore_meta(node, &parsed) {
            Some(parsed)
        } else {
            let yaml2 = to_yaml(&parsed);
            eprintln!("round-trip mismatch:\n{}", yaml_diff(&yaml, &yaml2));
            None
        }
    }

    /// Key *content* bytes for order-property comparisons, deliberately
    /// ignoring serialized spelling: a flow-unsafe plain token (`0[`) re-reads
    /// as the same string under a quoted style, and `null` normalizes between
    /// the Null variant and a resolved scalar — round-trip-equivalent trees
    /// must not differ in *order* on those style shifts.
    fn key_content(k: &crate::ast::CustomNode) -> Vec<u8> {
        match k {
            crate::ast::CustomNode::Scalar { value, .. } => value.as_bytes().to_vec(),
            crate::ast::CustomNode::Null { .. } => b"null".to_vec(),
            other => to_yaml(other).into_bytes(),
        }
    }

    /// Recursively apply random scalar style / chomping / flow style to every
    /// node in the tree, exercising the style setters on real data.
    fn deep_apply_styles(
        node: &mut crate::ast::CustomNode,
        style: crate::ast::ScalarStyle,
        chomp: crate::ast::Chomping,
        flow: bool,
    ) {
        use crate::ast::CustomNode;
        match node {
            CustomNode::Scalar { .. } => {
                node.set_scalar_style(style);
                node.set_chomping(chomp);
            }
            CustomNode::Mapping { .. } | CustomNode::Sequence { .. } => {
                node.set_flow_style(flow);
            }
            _ => {}
        }
        if let CustomNode::Mapping { pairs, .. } = node {
            for (_, v) in pairs.iter_mut() {
                deep_apply_styles(v, style, chomp, flow);
            }
        } else if let CustomNode::Sequence { items, .. } = node {
            for item in items.iter_mut() {
                deep_apply_styles(item, style, chomp, flow);
            }
        }
    }

    /// Assemble a syntactically-valid YAML document that exercises the
    /// anchor / alias / merge-key structure class — the shape behind the
    /// #163 (repeated alias) and #166 (self-referential merge stack overflow)
    /// crashes. `arb_custom_node()` emits `meta.anchor` but never an `Alias`
    /// node, so those resolve/merge paths are otherwise never fuzzed.
    ///
    /// The builder always produces well-formed source: anchors are defined
    /// before their aliases, and every merge value is one of the YAML 1.1
    /// forms (single alias, alias sequence, sequence with an inline map, bare
    /// inline map) plus the two *invalid* sources (scalar / null) the #166 fix
    /// now rejects instead of leaving a stray `<<`. A self-referential anchor
    /// (`a0: &a0` whose body merges `*a0`) and a forward chain (`a1` merging
    /// `*a0`) are toggled to drive the resolver's path-scoped cycle guard.
    fn arb_merge_alias_source() -> impl Strategy<Value = String> {
        (
            any::<bool>(), // base0 self-merge (name-based cycle)
            any::<bool>(), // base1 merges *a0 (forward chain)
            any::<bool>(), // base2 merges *a1 (chain depth 2)
            0u8..6,        // consumer merge-value shape
            1usize..4,     // repeated-alias count in a plain sequence
        )
            .prop_map(|(self_merge, chain1, chain2, shape, dup)| {
                let mut s = String::new();
                // base0 — optionally a `<<: *a0` cycle, the #166 crasher.
                if self_merge {
                    s.push_str("base0: &a0\n  <<: *a0\n  p0: q0\n");
                } else {
                    s.push_str("base0: &a0\n  p0: q0\n");
                }
                // base1 — merges the already-defined *a0 (acyclic forward ref).
                if chain1 {
                    s.push_str("base1: &a1\n  <<: *a0\n  p1: q1\n");
                } else {
                    s.push_str("base1: &a1\n  p1: q1\n");
                }
                // base2 — merges *a1, giving depth-2 chain expansion.
                if chain2 {
                    s.push_str("base2: &a2\n  <<: *a1\n  p2: q2\n");
                } else {
                    s.push_str("base2: &a2\n  p2: q2\n");
                }
                let consumer_merge = match shape {
                    0 => "<<: *a0",
                    1 => "<<: [*a0, *a1, *a2]",
                    2 => "<<: [*a0, {inline: 1}]",
                    3 => "<<: {inline: 2}",
                    4 => "<<: scalar", // invalid merge source
                    _ => "<<: null",   // invalid merge source
                };
                s.push_str(&format!("consumer:\n  {consumer_merge}\n  r: s\n"));
                // Repeated alias references to the same anchor (#163 class).
                let aliases = vec!["*a0"; dup];
                s.push_str(&format!("dup: [{}]\n", aliases.join(", ")));
                s
            })
    }

    proptest! {
        #[test]
        fn prop_roundtrip(node in arb_custom_node()) {
            if let Some(parsed) = try_roundtrip(&node) {
                prop_assert!(nodes_equal_ignore_meta(&node, &parsed));
            }
        }

        /// Text-level re-parseability: every AST the builder can produce must
        /// serialize to YAML that PARSES AT ALL. The AST-vs-AST round-trip
        /// above cannot catch invalid *text* — when nested block scalars lost
        /// their body indent, `parse(to_yaml(x))` failed outright and the
        /// mismatch was only visible through this gate (serializer base-indent
        /// regression, found via the TOML hot-spot benchmark sample).
        #[test]
        fn prop_output_always_parses(node in arb_custom_node()) {
            let yaml = to_yaml(&node);
            let reparsed = parse_with_options(&yaml, true, Schema::Core, 1000, false);
            prop_assert!(
                reparsed.is_ok(),
                "serialized output failed to re-parse: {yaml:?} -> {:?}",
                reparsed.err()
            );
        }

        #[test]
        fn prop_no_crash_invalid_utf8(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
            if let Ok(s) = std::str::from_utf8(&bytes) {
                let _ = parse_with_options(s, true, Schema::Core, 1000, false);
            }
        }

        #[test]
        fn prop_structure_depth_limit(node in arb_custom_node()) {
            let opts = SerializeOptions {
                max_depth: 3,
                ..Default::default()
            };
            let result = to_yaml_with_options(&node, &opts);
            if let Err(ref e) = result {
                let msg = e.to_string();
                prop_assert!(msg.contains("depth") || msg.contains("max"),
                    "unexpected error: {}", msg);
            }
        }

        // --- 0.14+ new features ---

        /// Applying arbitrary scalar style / chomping / flow style to a tree
        /// never produces invalid YAML (round-trip parse still succeeds and is
        /// structurally equivalent up to the tolerated normalizations).
        #[test]
        fn prop_style_setting_roundtrip(
            (mut node, style, chomp, flow) in (
                arb_custom_node(),
                arb_scalar_style(),
                arb_chomping(),
                any::<bool>(),
            )
        ) {
            deep_apply_styles(&mut node, style, chomp, flow);
            if let Some(parsed) = try_roundtrip(&node) {
                prop_assert!(nodes_equal_ignore_meta(&node, &parsed));
            }
        }

        /// Serializing the result of a round-trip produces an AST-equivalent (not
        /// byte-identical) document: an empty block container normalizes to its
        /// flow spelling on the first re-serialize, and stays stable after.
        #[test]
        fn prop_idempotent(node in arb_custom_node()) {
            if let Some(parsed) = try_roundtrip(&node) {
                let once = to_yaml(&parsed);
                let twice_parsed = parse_with_options(&once, true, Schema::Core, 1000, false)
                    .expect("re-parse of normalized output must succeed");
                prop_assert!(
                    nodes_equal_ignore_meta(&parsed, &twice_parsed),
                    "output not stable after first normalization"
                );
            }
        }

        /// Mapping key order survives the serialize → parse cycle.
        #[test]
        fn prop_mapping_order_preserved(node in arb_custom_node()) {
            if let Some(parsed) = try_roundtrip(&node) {
                fn key_order(n: &crate::ast::CustomNode) -> Vec<Vec<u8>> {
                    match n {
                        crate::ast::CustomNode::Mapping { pairs, .. } => pairs
                            .keys()
                            .map(key_content)
                            .collect(),
                        _ => vec![],
                    }
                }
                let a = key_order(&node);
                let b = key_order(&parsed);
                prop_assert_eq!(a, b, "mapping key order changed");
            }
        }

        /// Flow mappings inside the tree keep their entry order as well.
        /// (Only flow mappings whose entries survive the round-trip are
        /// compared; an empty block mapping normalizes to `{}` and reads back
        /// as a flow mapping, which is covered by the order-shape equality.)
        #[test]
        fn prop_flow_order_preserved(node in arb_custom_node()) {
            if let Some(parsed) = try_roundtrip(&node) {
                fn flow_key_orders(n: &crate::ast::CustomNode) -> Vec<Vec<Vec<u8>>> {
                    let mut out = Vec::new();
                    if let crate::ast::CustomNode::Mapping {
                        pairs,
                        flow_style: true,
                        ..
                    } = n
                        && !pairs.is_empty()
                    {
                        out.push(
                            pairs
                                .keys()
                                .map(key_content)
                                .collect(),
                        );
                    }
                    match n {
                        crate::ast::CustomNode::Mapping { pairs, .. } => {
                            for (k, v) in pairs {
                                out.extend(flow_key_orders(k));
                                out.extend(flow_key_orders(v));
                            }
                        }
                        crate::ast::CustomNode::Sequence { items, .. } => {
                            for i in items {
                                out.extend(flow_key_orders(i));
                            }
                        }
                        _ => {}
                    }
                    out
                }
                let a = flow_key_orders(&node);
                let b = flow_key_orders(&parsed);
                prop_assert_eq!(a, b, "flow mapping order changed");
            }
        }

        /// A comment on a node is emitted by the serializer (the node may be
        /// on a block path that round-trips, in which case the comment text
        /// must survive).
        #[test]
        fn prop_comment_preserved(node in arb_custom_node()) {
            // Only block containers' comments round-trip reliably; flow nodes
            // have comments normalized away by the strategy.
            fn comment_texts(n: &crate::ast::CustomNode) -> Vec<String> {
                match n {
                    crate::ast::CustomNode::Scalar { meta, .. }
                    | crate::ast::CustomNode::Mapping { meta, .. }
                    | crate::ast::CustomNode::Sequence { meta, .. }
                    | crate::ast::CustomNode::Null { meta, .. } => {
                        let mut v = meta
                            .comment
                            .as_ref()
                            .map(|c| vec![c.text.to_string()])
                            .unwrap_or_default();
                        match n {
                            crate::ast::CustomNode::Mapping { pairs, .. } => {
                                for (k, vv) in pairs {
                                    v.extend(comment_texts(k));
                                    v.extend(comment_texts(vv));
                                }
                            }
                            crate::ast::CustomNode::Sequence { items, .. } => {
                                for i in items {
                                    v.extend(comment_texts(i));
                                }
                            }
                            _ => {}
                        }
                        v
                    }
                    _ => vec![],
                }
            }
            if let Some(parsed) = try_roundtrip(&node) {
                let a = comment_texts(&node);
                let b = comment_texts(&parsed);
                for (orig, back) in a.iter().zip(b.iter()) {
                    prop_assert_eq!(orig, back, "comment text changed");
                }
            }
        }

        /// Tag preservation on round-trip.
        #[test]
        fn prop_tag_preserved(node in arb_custom_node()) {
            fn tag_texts(n: &crate::ast::CustomNode) -> Vec<String> {
                match n {
                    crate::ast::CustomNode::Scalar { meta, .. }
                    | crate::ast::CustomNode::Mapping { meta, .. }
                    | crate::ast::CustomNode::Sequence { meta, .. }
                    | crate::ast::CustomNode::Null { meta, .. } => {
                        let mut v = meta
                            .tag
                            .as_ref()
                            .map(|t| vec![t.to_string()])
                            .unwrap_or_default();
                        match n {
                            crate::ast::CustomNode::Mapping { pairs, .. } => {
                                for (k, vv) in pairs {
                                    v.extend(tag_texts(k));
                                    v.extend(tag_texts(vv));
                                }
                            }
                            crate::ast::CustomNode::Sequence { items, .. } => {
                                for i in items {
                                    v.extend(tag_texts(i));
                                }
                            }
                            _ => {}
                        }
                        v
                    }
                    _ => vec![],
                }
            }
            if let Some(parsed) = try_roundtrip(&node) {
                let a = tag_texts(&node);
                let b = tag_texts(&parsed);
                prop_assert_eq!(a, b, "tags changed after round-trip");
            }
        }
    }

    // Validate that `validate_node` never panics on arbitrary trees + rules,
    // and that every reported error path actually exists in the tree.
    proptest! {
        #[test]
        fn prop_validate_no_panic_and_real_paths(
            node in arb_custom_node(),
        ) {
            let paths = collect_paths(&node);
            prop_assume!(!paths.is_empty());
            let resolver = crate::parser::yaml::schema_language::RuleResolver::with_validate_rules(
                Vec::new(),
                Some(Schema::Core),
                vec![crate::parser::yaml::schema_language::ValidateRule::new(
                    None,
                    crate::parser::yaml::schema_language::ValidateKind::Type(
                        crate::parser::yaml::schema_language::TypeSpec::Scalar(
                            crate::parser::yaml::schema_language::YamlTypeKind::Str,
                        ),
                    ),
                )],
            );
            // Every scalar must be a string — most tree scalars won't satisfy
            // this, so errors (if any) must reference paths that exist.
            let result = crate::parser::yaml::schema_language::validate_node(&node, &resolver, "");
            let all_paths: std::collections::HashSet<String> = paths.into_iter().collect();
            if let Err(errors) = result {
                for e in &errors {
                    prop_assert!(
                        all_paths.contains(&e.path),
                        "validate reported error at non-existent path '{}': {}",
                        e.path, e.message
                    );
                }
            }
        }

        // Exercises `parse_schema_yaml` on random schema YAML — never panics.
        // Use arbitrary string in the "pattern" value position via a compact
        // char class that includes quotes and YAML punctuation.
        #[test]
        fn prop_schema_parse_no_panic(schema_yaml in "[!#$%&()*+,-./:;<=>?@\\[\\]^_`{}~a-zA-Z0-9 \"']{0,200}\\n*") {
            let _ = crate::parser::yaml::schema_language::parse_schema_yaml(&schema_yaml);
        }

        /// Anchors + aliases + merge keys (the #163/#166 structure class):
        /// parsing with merge resolution must never panic or blow the native
        /// stack, and any tree that parses must re-serialize and re-parse
        /// cleanly. Before the #166 path-scoped cycle guard, the self-merge
        /// case overflowed the stack and aborted the process; proptest cannot
        /// catch an abort, so a green run here is the regression proof.
        #[test]
        fn prop_merge_alias_never_panics(src in arb_merge_alias_source()) {
            let parsed = parse_with_options(&src, true, Schema::Core, 1000, false);
            if let Ok(node) = parsed {
                let out = to_yaml(&node);
                let _ = parse_with_options(&out, true, Schema::Core, 1000, false);
            }
        }
    }

    /// The generator's key rule, this crate's writer and this crate's reader have to
    /// describe the same mapping, and nothing but a table like this one proves it.
    /// `keys_collide_in_text` lives in `pyrs-ast` and is a hand-maintained model of
    /// the two components here; every gap between the model and the reality has shown
    /// up first as a random property-tier failure. The gap that wrote this guard
    /// reported `DuplicateKey("null")` for `!E null: A  # 0` beside `!E null: a` — a
    /// pair of `Null` nodes the rule did not look at at all.
    ///
    /// Both directions are pinned against real text. A pair the rule calls
    /// distinguishable must still hold two entries once its serialization is re-read
    /// strictly; a pair it calls identical must be refused or folded by the reader.
    /// A rule degraded to "everything collides" would leave the generator emitting
    /// single-key mappings and the tier green, so the table asserts the count of
    /// classes on each side as well.
    #[test]
    fn the_key_rule_agrees_with_the_writer_and_the_reader() {
        use crate::ast::{Comment, NodeMap, ScalarStyle, Tag};
        use pyrs_ast::ast::proptest_strategies::keys_collide_in_text;

        let null = |tag: Option<(&str, &str)>, anchor: Option<&str>, note: Option<&str>| {
            let mut node = crate::ast::CustomNode::Null {
                meta: Default::default(),
            };
            if let Some((handle, suffix)) = tag {
                node.set_tag(Tag {
                    handle: handle.to_string(),
                    suffix: suffix.to_string(),
                });
            }
            if let Some(name) = anchor {
                node.set_anchor(name);
            }
            if let Some(text) = note {
                node.set_comment(Comment {
                    text: text.into(),
                    standalone: false,
                });
            }
            node
        };
        let scalar = |value: &str, style: ScalarStyle, tag: Option<(&str, &str)>| {
            let mut node = crate::ast::CustomNode::plain_scalar(value);
            node.set_scalar_style(style);
            if let Some((handle, suffix)) = tag {
                node.set_tag(Tag {
                    handle: handle.to_string(),
                    suffix: suffix.to_string(),
                });
            }
            node
        };
        let plain = |value: &str| scalar(value, ScalarStyle::Plain, None);
        let mapping_key = |keys: &[(&str, &str)]| {
            let mut pairs = NodeMap::default();
            for (k, v) in keys {
                pairs.insert(plain(k), plain(v));
            }
            crate::ast::CustomNode::plain_mapping(pairs)
        };
        let sequence_key = |items: &[&str]| {
            crate::ast::CustomNode::plain_sequence(items.iter().map(|item| plain(item)).collect())
        };

        // (label, key a, key b, whether the rule claims the text tells them apart)
        let cases: Vec<(&str, crate::ast::CustomNode, crate::ast::CustomNode, bool)> = vec![
            // --- distinguishable: two entries must survive a strict re-read ---
            (
                "two plain scalars, different text",
                plain("k"),
                plain("v"),
                true,
            ),
            (
                "a null and a string",
                null(None, None, None),
                plain("hello"),
                true,
            ),
            (
                "two nulls under different tags",
                null(Some(("!", "E")), None, None),
                null(Some(("!", "F")), None, None),
                true,
            ),
            (
                "a tagged null and a bare null",
                null(Some(("!", "E")), None, None),
                null(None, None, None),
                true,
            ),
            (
                "two tagged nulls under different anchors",
                null(Some(("!", "E")), Some("a"), None),
                null(Some(("!", "E")), Some("b"), None),
                true,
            ),
            (
                "two nested mappings, different content",
                mapping_key(&[("x", "1")]),
                mapping_key(&[("x", "2")]),
                true,
            ),
            (
                "an empty mapping and an empty sequence",
                mapping_key(&[]),
                sequence_key(&[]),
                true,
            ),
            // The writer quotes an empty plain key (`"": v`) instead of emitting the
            // null spelling the reader would fold, so a tilde key keeps its own entry.
            ("a tilde and an empty key", plain("~"), plain(""), true),
            // --- identical: the reader must refuse or fold, never keep both ---
            (
                "two tagged nulls, one carrying a note",
                null(Some(("!", "E")), None, Some("0")),
                null(Some(("!", "E")), None, None),
                false,
            ),
            (
                "two bare nulls under different anchors",
                null(None, Some("a"), None),
                null(None, Some("b"), None),
                false,
            ),
            (
                "a null and a plain scalar spelled null",
                null(None, None, None),
                plain("null"),
                false,
            ),
            (
                "the NULL and null spellings",
                plain("NULL"),
                plain("null"),
                false,
            ),
            (
                "a null and a tilde",
                null(None, None, None),
                plain("~"),
                false,
            ),
            (
                "the same text, plain and quoted",
                plain("k"),
                scalar("k", ScalarStyle::SingleQuoted, None),
                false,
            ),
            (
                "two empty mappings",
                mapping_key(&[]),
                mapping_key(&[]),
                false,
            ),
            (
                "two empty sequences",
                sequence_key(&[]),
                sequence_key(&[]),
                false,
            ),
        ];
        let distinguishable = cases.iter().filter(|(_, _, _, keepable)| *keepable).count();
        assert_eq!(
            distinguishable, 8,
            "the table must keep a real share of its classes on each side"
        );

        for (label, a, b, keepable) in cases {
            assert_eq!(
                keys_collide_in_text(&a, &b),
                !keepable,
                "{label}: the rule disagrees with the table"
            );
            // Values that differ, so a fold is visible as a lost entry rather than
            // an overwritten one.
            let mut pairs = NodeMap::default();
            pairs.insert(a.clone(), plain("one"));
            // `IndexMap::insert` hands back the value it replaced: nothing replaced
            // means both keys are now in the map.
            let held_both = pairs.insert(b.clone(), plain("two")).is_none();
            if !held_both {
                // The node map itself cannot hold the pair — the same conclusion the
                // rule reaches, and a reason the text oracle below cannot run.
                assert!(!keepable, "{label}: the node map folded a distinct pair");
                continue;
            }
            let yaml = to_yaml(&crate::ast::CustomNode::plain_mapping(pairs));
            match parse_with_options(&yaml, true, Schema::Core, 1000, false) {
                Ok(crate::ast::CustomNode::Mapping { pairs: read, .. }) => {
                    assert_eq!(
                        read.len(),
                        if keepable { 2 } else { 1 },
                        "{label}: {yaml:?} re-read as {read:?}"
                    );
                }
                Ok(other) => panic!("{label}: {yaml:?} re-read as a {other:?}"),
                Err(err) => assert!(
                    !keepable,
                    "{label}: a distinct pair {yaml:?} was refused: {err}"
                ),
            }
        }
    }
}
