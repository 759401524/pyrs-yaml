use crate::ast::{Chomping, CustomNode, ScalarStyle};
use indexmap::IndexMap;
use std::collections::HashMap;

/// Resolve merge keys (<<) in a YAML AST
/// This replaces <<: *alias entries with the actual values from the referenced mapping
pub fn resolve_merge_keys(node: &mut CustomNode) {
    // First, collect all anchor names and their mappings
    let mut anchors: HashMap<String, IndexMap<CustomNode, CustomNode>> = HashMap::new();
    collect_anchors(node, &mut anchors);

    // Then, resolve merge keys
    resolve_merges_recursive(node, &anchors);
}

/// Recursively resolve merge keys in a node
fn resolve_merges_recursive(
    node: &mut CustomNode,
    anchors: &HashMap<String, IndexMap<CustomNode, CustomNode>>,
) {
    match node {
        CustomNode::Mapping { pairs, .. } => {
            resolve_mapping_merges(pairs, anchors);
        }
        CustomNode::Sequence { items, .. } => {
            for item in items.iter_mut() {
                resolve_merges_recursive(item, anchors);
            }
        }
        _ => {}
    }
}

/// Collect all anchors and their mappings from the AST
fn collect_anchors(
    node: &CustomNode,
    anchors: &mut HashMap<String, IndexMap<CustomNode, CustomNode>>,
) {
    match node {
        CustomNode::Mapping { pairs, anchor, .. } => {
            if let Some(anchor_name) = anchor {
                anchors.insert(anchor_name.clone(), pairs.clone());
            }
            for (_key, value) in pairs {
                collect_anchors(value, anchors);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                collect_anchors(item, anchors);
            }
        }
        _ => {}
    }
}

/// Resolve merge keys in a mapping
fn resolve_mapping_merges(
    pairs: &mut IndexMap<CustomNode, CustomNode>,
    anchors: &HashMap<String, IndexMap<CustomNode, CustomNode>>,
) {
    let merge_key = CustomNode::Scalar {
        value: "<<".to_string(),
        style: ScalarStyle::Plain,
        comment: None,
        anchor: None,
        tag: None,
        chomping: Chomping::Clip,
    };

    // Check if merge key exists and collect merge data
    let merge_data = if let Some(merge_value) = pairs.get(&merge_key) {
        let mut merged_pairs = Vec::new();

        match merge_value {
            CustomNode::Alias { name } => {
                // Single alias: <<: *alias
                if let Some(merged) = anchors.get(name) {
                    for (k, v) in merged {
                        // Only add if not already overridden in current mapping
                        if !pairs.contains_key(k) {
                            merged_pairs.push((k.clone(), v.clone()));
                        }
                    }
                }
            }
            CustomNode::Sequence { items, .. } => {
                // Multiple aliases: <<: [*alias1, *alias2]
                for item in items {
                    if let CustomNode::Alias { name } = item {
                        if let Some(merged) = anchors.get(name) {
                            for (k, v) in merged {
                                if !pairs.contains_key(k) {
                                    merged_pairs.push((k.clone(), v.clone()));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Some(merged_pairs)
    } else {
        None
    };

    // Apply merge data if we have any
    if let Some(merged_pairs) = merge_data {
        // Remove the merge key
        pairs.swap_remove(&merge_key);

        // Add merged pairs at the beginning (to preserve order: merged first, then overrides)
        let mut new_pairs = IndexMap::new();
        for (k, v) in merged_pairs {
            new_pairs.insert(k, v);
        }
        for (k, v) in pairs.iter() {
            new_pairs.insert(k.clone(), v.clone());
        }

        *pairs = new_pairs;
    }

    // Recursively resolve in nested mappings
    for value in pairs.values_mut() {
        resolve_merges_recursive(value, anchors);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn make_scalar(value: &str) -> CustomNode {
        CustomNode::Scalar {
            value: value.to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
        }
    }

    fn get_mapping(node: &CustomNode) -> &IndexMap<CustomNode, CustomNode> {
        match node {
            CustomNode::Mapping { pairs, .. } => pairs,
            _ => panic!("expected Mapping"),
        }
    }

    fn get_scalar_value(node: &CustomNode) -> &str {
        match node {
            CustomNode::Scalar { value, .. } => value,
            _ => panic!("expected Scalar"),
        }
    }

    #[test]
    fn test_single_merge() {
        let yaml = "defaults: &defaults\n  timeout: 30\nprod:\n  <<: *defaults\n  host: x";
        let mut root = parse(yaml).unwrap();
        resolve_merge_keys(&mut root);

        let pairs = get_mapping(&root);
        let prod = pairs.get(&make_scalar("prod")).unwrap();
        let prod_pairs = get_mapping(prod);

        assert_eq!(get_scalar_value(prod_pairs.get(&make_scalar("timeout")).unwrap()), "30");
        assert_eq!(get_scalar_value(prod_pairs.get(&make_scalar("host")).unwrap()), "x");
    }

    #[test]
    fn test_multiple_merge() {
        let yaml = "base1: &b1\n  a: 1\nbase2: &b2\n  b: 2\ncurrent:\n  <<: [*b1, *b2]\n  c: 3";
        let mut root = parse(yaml).unwrap();
        resolve_merge_keys(&mut root);

        let pairs = get_mapping(&root);
        let current = pairs.get(&make_scalar("current")).unwrap();
        let current_pairs = get_mapping(current);

        assert_eq!(get_scalar_value(current_pairs.get(&make_scalar("a")).unwrap()), "1");
        assert_eq!(get_scalar_value(current_pairs.get(&make_scalar("b")).unwrap()), "2");
        assert_eq!(get_scalar_value(current_pairs.get(&make_scalar("c")).unwrap()), "3");
    }

    #[test]
    fn test_merge_override_order() {
        let yaml = "base: &base\n  x: 1\n  y: 2\nderived:\n  <<: *base\n  y: 99";
        let mut root = parse(yaml).unwrap();
        resolve_merge_keys(&mut root);

        let pairs = get_mapping(&root);
        let derived = pairs.get(&make_scalar("derived")).unwrap();
        let derived_pairs = get_mapping(derived);

        // Local key overrides merged key
        assert_eq!(get_scalar_value(derived_pairs.get(&make_scalar("x")).unwrap()), "1");
        assert_eq!(get_scalar_value(derived_pairs.get(&make_scalar("y")).unwrap()), "99");

        // Verify order: merged keys first, then overrides
        let keys: Vec<&CustomNode> = derived_pairs.keys().collect();
        assert_eq!(get_scalar_value(keys[0]), "x");
        assert_eq!(get_scalar_value(keys[1]), "y");
    }
}
