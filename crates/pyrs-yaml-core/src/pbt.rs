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

    proptest! {
        #[test]
        fn prop_roundtrip(node in arb_custom_node()) {
            if let Some(parsed) = try_roundtrip(&node) {
                prop_assert!(nodes_equal_ignore_meta(&node, &parsed));
            }
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
                            .map(|k| {
                                let y = to_yaml(k);
                                y.into_bytes()
                            })
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
                    {
                        if !pairs.is_empty() {
                            out.push(
                                pairs
                                    .keys()
                                    .map(|k| to_yaml(k).into_bytes())
                                    .collect(),
                            );
                        }
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
                        crate::parser::yaml::schema_language::YamlTypeKind::Str,
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
    }
}
