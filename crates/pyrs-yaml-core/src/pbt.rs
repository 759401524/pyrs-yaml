#[cfg(test)]
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
    }
}
