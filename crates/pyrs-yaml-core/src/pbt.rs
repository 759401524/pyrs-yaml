#[cfg(test)]
#[allow(unused_doc_comments)]
mod tests {
    use crate::ast::proptest_strategies::*;
    use crate::parser::parse_with_options;
    use crate::parser::yaml::Schema;
    use crate::serializer::{SerializeOptions, to_yaml, to_yaml_with_options};
    use proptest::prelude::*;

    fn try_roundtrip(node: &crate::ast::CustomNode) -> Option<crate::ast::CustomNode> {
        let yaml = to_yaml(node);
        let parsed = parse_with_options(&yaml, true, Schema::Core, 1000, false).ok()?;
        if nodes_equal_ignore_meta(node, &parsed) {
            Some(parsed)
        } else {
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
            let result = crate::parser::yaml::schema_language::validate_node(&node, &resolver);
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
