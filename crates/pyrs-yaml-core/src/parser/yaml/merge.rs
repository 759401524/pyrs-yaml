use crate::ast::CustomNode;
use indexmap::IndexMap;
use std::collections::HashMap;

/// Resolve merge keys (<<) in a YAML AST
/// This replaces <<: *alias entries with the actual values from the referenced mapping
pub fn resolve_merge_keys(node: &mut CustomNode) {
    // Fast path: no merge key present anywhere, skip anchor collection entirely
    if !has_merge_key(node) {
        return;
    }
    // First, collect all anchor names and their mappings
    let mut anchors: HashMap<String, IndexMap<CustomNode, CustomNode>> = HashMap::new();
    collect_anchor_mappings(node, &mut anchors);

    // Then, resolve merge keys
    resolve_merges_recursive(node, &anchors);
}

/// Return true if any mapping in the tree has a `<<` merge key.
fn has_merge_key(node: &CustomNode) -> bool {
    match node {
        CustomNode::Mapping { pairs, .. } => {
            let merge_key = CustomNode::plain_scalar("<<");
            if pairs.contains_key(&merge_key) {
                return true;
            }
            pairs.values().any(has_merge_key)
        }
        CustomNode::Sequence { items, .. } => items.iter().any(has_merge_key),
        _ => false,
    }
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

/// Collect all anchor names and their mapping pairs from the AST.
/// Used for resolving merge keys (`<<`).
fn collect_anchor_mappings(
    node: &CustomNode,
    anchors: &mut HashMap<String, IndexMap<CustomNode, CustomNode>>,
) {
    match node {
        CustomNode::Mapping { pairs, meta, .. } => {
            if let Some(anchor_name) = &meta.anchor {
                anchors.insert(anchor_name.clone(), pairs.clone());
            }
            for (_key, value) in pairs {
                collect_anchor_mappings(value, anchors);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                collect_anchor_mappings(item, anchors);
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
    let merge_key = CustomNode::plain_scalar("<<");

    let merge_data = if let Some(merge_value) = pairs.get(&merge_key) {
        let merged_pairs = collect_merge_data(merge_value, pairs, anchors);
        if merged_pairs.is_empty() {
            None
        } else {
            Some(merged_pairs)
        }
    } else {
        None
    };

    if let Some(merged_pairs) = merge_data {
        prepend_merged_pairs(pairs, &merge_key, merged_pairs);
    }

    // Recursively resolve in nested mappings
    for value in pairs.values_mut() {
        resolve_merges_recursive(value, anchors);
    }
}

/// Collect merged pairs from a merge key value (single alias or sequence of aliases).
fn collect_merge_data(
    merge_value: &CustomNode,
    pairs: &IndexMap<CustomNode, CustomNode>,
    anchors: &HashMap<String, IndexMap<CustomNode, CustomNode>>,
) -> Vec<(CustomNode, CustomNode)> {
    let mut merged_pairs = Vec::new();

    match merge_value {
        CustomNode::Alias { name } => {
            collect_merged_pairs_for_anchor(name, pairs, anchors, &mut merged_pairs);
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                if let CustomNode::Alias { name } = item {
                    collect_merged_pairs_for_anchor(name, pairs, anchors, &mut merged_pairs);
                }
            }
        }
        _ => {}
    }

    merged_pairs
}

/// Clear source ranges recursively on a node and its descendants.
/// Merged-in pairs are cloned from an anchor's source location; their byte
/// ranges point into the wrong text, so they must not be treated as layout
/// verifiable or spliceable.
fn clear_source_ranges(node: &mut CustomNode) {
    match node {
        CustomNode::Scalar { meta, .. }
        | CustomNode::Mapping { meta, .. }
        | CustomNode::Sequence { meta, .. }
        | CustomNode::Null { meta, .. } => {
            meta.source_range = None;
        }
        CustomNode::Alias { .. } => {}
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for v in pairs.values_mut() {
                clear_source_ranges(v);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items.iter_mut() {
                clear_source_ranges(item);
            }
        }
        _ => {}
    }
}

/// Collect merged pairs from a single anchor reference.
fn collect_merged_pairs_for_anchor(
    name: &str,
    pairs: &IndexMap<CustomNode, CustomNode>,
    anchors: &HashMap<String, IndexMap<CustomNode, CustomNode>>,
    result: &mut Vec<(CustomNode, CustomNode)>,
) {
    if let Some(merged) = anchors.get(name) {
        for (k, v) in merged {
            if !pairs.contains_key(k) {
                let mut key = k.clone();
                let mut value = v.clone();
                clear_source_ranges(&mut key);
                clear_source_ranges(&mut value);
                result.push((key, value));
            }
        }
    }
}

/// Remove the merge key and prepend merged pairs at the beginning of the mapping.
fn prepend_merged_pairs(
    pairs: &mut IndexMap<CustomNode, CustomNode>,
    merge_key: &CustomNode,
    merged_pairs: Vec<(CustomNode, CustomNode)>,
) {
    pairs.shift_remove(merge_key);

    // Insert merged pairs at the front in order, keeping existing pairs in
    // place. `merged_pairs` is filtered against existing keys by the caller,
    // so `shift_insert` cannot collide.
    for (k, v) in merged_pairs.into_iter().rev() {
        pairs.shift_insert(0, k, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse, yaml::YamlSchema};

    fn make_scalar(value: &str) -> CustomNode {
        CustomNode::plain_scalar(value)
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
        let mut root = parse(yaml, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);

        let pairs = get_mapping(&root);
        let prod = pairs.get(&make_scalar("prod")).unwrap();
        let prod_pairs = get_mapping(prod);

        assert_eq!(
            get_scalar_value(prod_pairs.get(&make_scalar("timeout")).unwrap()),
            "30"
        );
        assert_eq!(
            get_scalar_value(prod_pairs.get(&make_scalar("host")).unwrap()),
            "x"
        );
    }

    #[test]
    fn test_multiple_merge() {
        let yaml = "base1: &b1\n  a: 1\nbase2: &b2\n  b: 2\ncurrent:\n  <<: [*b1, *b2]\n  c: 3";
        let mut root = parse(yaml, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);

        let pairs = get_mapping(&root);
        let current = pairs.get(&make_scalar("current")).unwrap();
        let current_pairs = get_mapping(current);

        assert_eq!(
            get_scalar_value(current_pairs.get(&make_scalar("a")).unwrap()),
            "1"
        );
        assert_eq!(
            get_scalar_value(current_pairs.get(&make_scalar("b")).unwrap()),
            "2"
        );
        assert_eq!(
            get_scalar_value(current_pairs.get(&make_scalar("c")).unwrap()),
            "3"
        );
    }

    #[test]
    fn test_merge_override_order() {
        let yaml = "base: &base\n  x: 1\n  y: 2\nderived:\n  <<: *base\n  y: 99";
        let mut root = parse(yaml, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);

        let pairs = get_mapping(&root);
        let derived = pairs.get(&make_scalar("derived")).unwrap();
        let derived_pairs = get_mapping(derived);

        // Local key overrides merged key
        assert_eq!(
            get_scalar_value(derived_pairs.get(&make_scalar("x")).unwrap()),
            "1"
        );
        assert_eq!(
            get_scalar_value(derived_pairs.get(&make_scalar("y")).unwrap()),
            "99"
        );

        // Verify order: merged keys first, then overrides
        let keys: Vec<&CustomNode> = derived_pairs.keys().collect();
        assert_eq!(get_scalar_value(keys[0]), "x");
        assert_eq!(get_scalar_value(keys[1]), "y");
    }
}
