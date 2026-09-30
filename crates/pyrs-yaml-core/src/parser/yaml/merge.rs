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

    // Then, resolve merge keys. `path` carries the anchor names whose expansion
    // is currently being walked, so a merge that resolves back to an ancestor on
    // this path is recognised as a cycle and left alone instead of recursing
    // until the stack is exhausted.
    let mut path: Vec<String> = Vec::new();
    resolve_merges_recursive(node, &anchors, &mut path);
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
    path: &mut Vec<String>,
) {
    match node {
        CustomNode::Mapping { pairs, .. } => {
            resolve_mapping_merges(pairs, anchors, path);
        }
        CustomNode::Sequence { items, .. } => {
            for item in items.iter_mut() {
                resolve_merges_recursive(item, anchors, path);
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

/// Anchor name carried alongside each merged-in key so the expansion it came
/// from can be told apart from an independent merge elsewhere in the document.
/// An empty name marks an inline mapping used directly as a merge value.
type AnchoredPairs = Vec<(String, CustomNode, CustomNode)>;

/// Resolve merge keys in a mapping
fn resolve_mapping_merges(
    pairs: &mut IndexMap<CustomNode, CustomNode>,
    anchors: &HashMap<String, IndexMap<CustomNode, CustomNode>>,
    path: &mut Vec<String>,
) {
    let merge_key = CustomNode::plain_scalar("<<");

    // A `<<` whose value resolves to nothing is still a merge key: YAML says it
    // merges a mapping or a sequence of mappings, so a scalar, a sequence, or a
    // null source is a hard error rather than an ordinary key that happens to
    // be spelled `<<`. Reading it as a plain key is how a self-referential
    // anchor used to survive into the output (`a: &a\n  b:\n    <<: *a`).
    let merge_data = pairs
        .get(&merge_key)
        .map(|merge_value| collect_merge_data(merge_value, pairs, anchors, path));

    if let Some(merged_pairs) = merge_data {
        // Expansions resolve against the *original* mapping, so an anchor that
        // transitively merges another still sees the latter's own merge keys.
        // `<<` survives in the expansion and is stripped on the way in.
        //
        // The first anchor to supply a key wins, both for a key repeated inside
        // one anchor and for a collision across the merge value: YAML 1.1 gives
        // each anchor its own precedence among the merge sources and lets the
        // mapping's own keys override all of them.
        let mut expanded: IndexMap<CustomNode, (String, CustomNode)> =
            IndexMap::with_capacity(pairs.len());
        for (source, key, value) in merged_pairs {
            expanded.entry(key).or_insert((source, value));
        }

        // The anchor name stays on the path while its expansion is walked, so a
        // merge key inside the expansion that points back at its own ancestor is
        // recognised and left alone. An inline mapping (empty source) pulls in no
        // anchor, so there is nothing to guard against.
        for value in expanded.values_mut() {
            if value.0.is_empty() {
                resolve_merges_recursive(&mut value.1, anchors, path);
            } else {
                path.push(value.0.clone());
                resolve_merges_recursive(&mut value.1, anchors, path);
                path.pop();
            }
        }

        let expanded: IndexMap<CustomNode, CustomNode> = expanded
            .into_iter()
            .map(|(key, (_source, value))| (key, value))
            .collect();
        prepend_merged_pairs(pairs, &merge_key, expanded);
    }

    // Recursively resolve in nested mappings
    for value in pairs.values_mut() {
        resolve_merges_recursive(value, anchors, path);
    }
}

/// Collect merged pairs from a merge key value.
///
/// YAML 1.1 merge types: an alias to a mapping, a sequence of such aliases,
/// or an inline mapping. Nothing else is a merge, so anything else is rejected
/// rather than silently dropped (which used to leave a stray `<<` behind).
///
/// `path` carries the anchors currently being expanded; it is forwarded to the
/// anchor collector, which is what lets a merge inside an expansion detect that
/// it is pulling in its own ancestor.
fn collect_merge_data(
    merge_value: &CustomNode,
    pairs: &IndexMap<CustomNode, CustomNode>,
    anchors: &HashMap<String, IndexMap<CustomNode, CustomNode>>,
    path: &[String],
) -> AnchoredPairs {
    let mut merged_pairs = AnchoredPairs::new();

    match merge_value {
        CustomNode::Alias { name } => {
            collect_merged_pairs_for_anchor(name, pairs, anchors, path, &mut merged_pairs);
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                match item {
                    CustomNode::Alias { name } => collect_merged_pairs_for_anchor(
                        name,
                        pairs,
                        anchors,
                        path,
                        &mut merged_pairs,
                    ),
                    CustomNode::Mapping { pairs: sub, .. } => {
                        collect_inline_pairs(sub, pairs, &mut merged_pairs);
                    }
                    _ => {
                        // Any other element type invalidates the whole merge
                        // value: a partially applied sequence is not a merge.
                        return AnchoredPairs::new();
                    }
                }
            }
        }
        CustomNode::Mapping { pairs: sub, .. } => {
            collect_inline_pairs(sub, pairs, &mut merged_pairs);
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

/// Collect merged pairs from an inline mapping used directly as a merge value.
fn collect_inline_pairs(
    source: &IndexMap<CustomNode, CustomNode>,
    pairs: &IndexMap<CustomNode, CustomNode>,
    result: &mut AnchoredPairs,
) {
    for (key, value) in source {
        if pairs.contains_key(key) {
            continue;
        }
        let mut key = key.clone();
        let mut value = value.clone();
        clear_source_ranges(&mut key);
        clear_source_ranges(&mut value);
        result.push((String::new(), key, value));
    }
}

/// Collect merged pairs from a single anchor reference.
fn collect_merged_pairs_for_anchor(
    name: &str,
    pairs: &IndexMap<CustomNode, CustomNode>,
    anchors: &HashMap<String, IndexMap<CustomNode, CustomNode>>,
    path: &[String],
    result: &mut AnchoredPairs,
) {
    let Some(merged) = anchors.get(name) else {
        return;
    };
    // Re-entering an anchor whose expansion is already being walked further up
    // this path is a cycle, not a merge. Skipping the expansion leaves the `<<`
    // key resolving to nothing, which is the only terminating answer: the
    // alternative is feeding the expansion back into itself until the stack is
    // exhausted. Because the guard is path-scoped, two sibling references to the
    // same anchor (each starting from an empty path) still expand fully.
    if path.iter().any(|open| open == name) {
        return;
    }
    for (k, v) in merged {
        if !pairs.contains_key(k) {
            let mut key = k.clone();
            let mut value = v.clone();
            clear_source_ranges(&mut key);
            clear_source_ranges(&mut value);
            result.push((name.to_string(), key, value));
        }
    }
}

/// Remove the merge key and prepend merged pairs at the beginning of the mapping.
fn prepend_merged_pairs(
    pairs: &mut IndexMap<CustomNode, CustomNode>,
    merge_key: &CustomNode,
    merged_pairs: IndexMap<CustomNode, CustomNode>,
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

    // Regression: issue #166. A self-referential merge (`<<: *a` inside anchor
    // `&a`) used to expand forever and blow the native stack (exit 0xC00000FD),
    // taking the whole interpreter process down with it. The path-scoped anchor
    // guard must terminate instead. An acyclic AST cannot carry PyYAML's cyclic
    // dict, so the expansion bottoms out at an empty mapping -- the assertion is
    // "it terminates and resolves the known prefix", not an exact structural tie.
    #[test]
    fn test_self_referential_merge_terminates() {
        let yaml = "a: &a\n  b:\n    <<: *a\n";
        let mut root = parse(yaml, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);

        let outer = get_mapping(&root).get(&make_scalar("a")).unwrap();
        let b = get_mapping(outer).get(&make_scalar("b")).unwrap();
        let b_pairs = get_mapping(b);
        // `<<` is gone (it resolved to the cycle guard), the known key `b` is
        // expanded once, and its nested `b` terminates at an empty mapping.
        assert!(
            !b_pairs.contains_key(&make_scalar("<<")),
            "stray merge key survived"
        );
        let inner = b_pairs.get(&make_scalar("b")).unwrap();
        assert!(
            get_mapping(inner).is_empty(),
            "cycle should bottom out empty"
        );
    }

    #[test]
    fn test_self_referential_merge_with_siblings_terminates() {
        let yaml = "a: &a\n  b: 1\n  c:\n    <<: *a\n";
        let mut root = parse(yaml, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);

        let a = get_mapping(&root).get(&make_scalar("a")).unwrap();
        let a_pairs = get_mapping(a);
        assert_eq!(
            get_scalar_value(a_pairs.get(&make_scalar("b")).unwrap()),
            "1"
        );
        let c = get_mapping(a_pairs.get(&make_scalar("c")).unwrap());
        // c merged a's non-conflicting keys (b) but its own `c` cycle is guarded.
        assert_eq!(get_scalar_value(c.get(&make_scalar("b")).unwrap()), "1");
        assert!(
            !c.contains_key(&make_scalar("<<")),
            "stray merge key survived"
        );
    }

    #[test]
    fn test_nested_merge_in_anchor_terminates() {
        let yaml = "a: &a\n  x: 1\n  sub:\n    <<: *a\nb:\n  <<: *a\n";
        let mut root = parse(yaml, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);

        let pairs = get_mapping(&root);
        for key in ["a", "b"] {
            let m = get_mapping(pairs.get(&make_scalar(key)).unwrap());
            assert_eq!(get_scalar_value(m.get(&make_scalar("x")).unwrap()), "1");
            assert!(
                !m.contains_key(&make_scalar("<<")),
                "stray merge key in {key}"
            );
        }
    }

    // Regression: a merge key whose source is not a mapping (null alias, scalar,
    // or a block sequence) used to survive as a literal `<<` key in the output.
    #[test]
    fn test_null_merge_source_leaves_no_residual() {
        let yaml = "a: &a\nb:\n  <<: *a\n  c: 1\n";
        let mut root = parse(yaml, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);

        let b = get_mapping(get_mapping(&root).get(&make_scalar("b")).unwrap());
        assert!(
            !b.contains_key(&make_scalar("<<")),
            "stray merge key survived"
        );
        assert_eq!(get_scalar_value(b.get(&make_scalar("c")).unwrap()), "1");
    }

    #[test]
    fn test_inline_map_merge_value() {
        let yaml = "b:\n  <<: {x: 1}\n  y: 2\n";
        let mut root = parse(yaml, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);

        let b = get_mapping(get_mapping(&root).get(&make_scalar("b")).unwrap());
        assert!(
            !b.contains_key(&make_scalar("<<")),
            "stray merge key survived"
        );
        assert_eq!(get_scalar_value(b.get(&make_scalar("x")).unwrap()), "1");
        assert_eq!(get_scalar_value(b.get(&make_scalar("y")).unwrap()), "2");
    }

    #[test]
    fn test_merge_sequence_of_nonalias_still_merges_inline_map() {
        let yaml = "a: &a\n  x: 1\nb:\n  <<: [*a, {y: 2}]\n  z: 3\n";
        let mut root = parse(yaml, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);

        let b = get_mapping(get_mapping(&root).get(&make_scalar("b")).unwrap());
        assert!(
            !b.contains_key(&make_scalar("<<")),
            "stray merge key survived"
        );
        for key in ["x", "y", "z"] {
            assert!(b.contains_key(&make_scalar(key)), "merge lost key {key}");
        }
    }
}
