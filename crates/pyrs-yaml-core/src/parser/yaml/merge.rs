use crate::ast::{Comment, CustomNode, ScalarStyle};
use indexmap::IndexMap;
use std::collections::HashMap;

/// Recursion budget for merge/alias expansion. The `path` cycle guard bounds
/// re-entry into an *already-expanding* anchor, but a pathological input can
/// still keep producing fresh expansion depth (nested self-anchors feeding the
/// tail walk). This hard cap turns any runaway expansion into a graceful stop —
/// leaving an unresolved `<<` as an ordinary key — instead of a native-stack
/// SIGSEGV, mirroring the parser's container-depth and serializer `max_depth`
/// guards. 256 is orders of magnitude above any real merge nesting.
const MAX_MERGE_DEPTH: usize = 256;

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
    resolve_merges_recursive(node, &anchors, &mut path, 0);
}

/// Return true if any mapping in the tree has a `<<` merge key.
fn has_merge_key(node: &CustomNode) -> bool {
    match node {
        CustomNode::Mapping { pairs, .. } => {
            if pairs.keys().any(is_merge_key) {
                return true;
            }
            pairs.values().any(has_merge_key)
        }
        CustomNode::Sequence { items, .. } => items.iter().any(has_merge_key),
        _ => false,
    }
}

/// Whether an inline merge source names `<<` as one of its own keys - the shape whose
/// contribution the collection step used to discard. Top level only, on purpose: a `<<`
/// nested inside a *value* of the source is resolved by the ordinary recursive walk
/// (every mapping in the tree is visited, and merged values are re-walked at their own
/// level), so looking deeper here would pay a whole-subtree scan for nothing.
fn source_carries_merge_key(merge_value: &CustomNode) -> bool {
    match merge_value {
        CustomNode::Mapping { pairs, .. } => pairs.keys().any(is_merge_key),
        CustomNode::Sequence { items, .. } => items.iter().any(|item| {
            matches!(
                item,
                CustomNode::Mapping { pairs, .. } if pairs.keys().any(is_merge_key)
            )
        }),
        _ => false,
    }
}

/// True when a key IS the merge key: an untagged plain scalar holding `<<`.
///
/// Identity is what YAML resolves, not the whole node — a comment or an anchor is
/// metadata. Comparing full `CustomNode`s (the old `== plain_scalar("<<")`) made a
/// `<<` that carries a note invisible to this pass, so the *same document* meant two
/// different things depending on where its note sat: `<<: #*<LF>  y:` kept a literal
/// `<<` key, while the spelling the writer emits for it (`<<:<LF>  y: ~  # *`) consumed
/// the merge and the pair vanished between rounds (libFuzzer `yaml_roundtrip`
/// crash-69931a77, minimised to 10 bytes). Style and tag still decide identity, exactly
/// as the null-key rule does: a quoted `"<<"` or a tagged `!x <<` is not a merge key.
fn is_merge_key(key: &CustomNode) -> bool {
    matches!(
        key,
        CustomNode::Scalar {
            value,
            style: ScalarStyle::Plain,
            meta,
            ..
        } if meta.tag.is_none() && value.as_ref() == "<<"
    )
}

/// Recursively resolve merge keys in a node
fn resolve_merges_recursive(
    node: &mut CustomNode,
    anchors: &HashMap<String, IndexMap<CustomNode, CustomNode>>,
    path: &mut Vec<String>,
    depth: usize,
) {
    if depth >= MAX_MERGE_DEPTH {
        return;
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            resolve_mapping_merges(pairs, anchors, path, depth);
        }
        CustomNode::Sequence { items, .. } => {
            for item in items.iter_mut() {
                resolve_merges_recursive(item, anchors, path, depth + 1);
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

/// True if `node`'s subtree contains an `Alias`. A merge value that references
/// an anchor is unsafe to keep as a literal key: object conversion would follow
/// the alias and could re-enter the #166 self-referential expansion even when
/// the cycle guard produced an empty merge here.
fn references_alias(node: &CustomNode) -> bool {
    match node {
        CustomNode::Alias { .. } => true,
        CustomNode::Sequence { items, .. } => items.iter().any(references_alias),
        CustomNode::Mapping { pairs, .. } => pairs
            .iter()
            .any(|(k, v)| references_alias(k) || references_alias(v)),
        CustomNode::Null { .. } | CustomNode::Scalar { .. } => false,
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
    depth: usize,
) {
    // The merge entry is addressed by position, never by value: `IndexMap` compares
    // whole nodes, and a note or an anchor on the key is metadata, not identity. See
    // [`is_merge_key`].
    let merge_index = pairs.keys().position(is_merge_key);

    // Snapshot the mapping's own children (minus the merge entry) before the merge
    // expansion inserts cloned anchor pairs into `pairs`. The tail walk below must
    // re-walk only these original children: the freshly merged-in values are
    // clones of an anchor body that was already resolved under the path guard in
    // the expansion loop. Re-walking them here (with the guard context gone)
    // re-expands nested self-anchors and grows the walk without bound — the
    // libFuzzer `parse_yaml` stack-overflow (path-identical frames cycling
    // resolve_mapping_merges -> resolve_merges_recursive forever).
    //
    // Positions, not key clones, because `insert_merged_pairs` shift-inserts into the
    // middle of the map and would invalidate every index after it — which is why this
    // walk now runs *before* the insertion. (Looking the children up by key after the
    // insertion, as this used to, could also hand back a merged-in clone whenever that
    // clone compared equal to an own key and sat earlier in the map.)
    let own_indices: Vec<usize> = (0..pairs.len())
        .filter(|index| Some(*index) != merge_index)
        .collect();

    // Decide whether `<<` is consumed as a merge or kept as an ordinary key.
    // The safety-critical line is any ALIAS in the value's subtree, not just a
    // top-level alias: a surviving `<<: *a` or `<<: [*a, *a]` node (even when
    // the cycle guard resolves it to nothing) re-expands during object
    // conversion and re-triggers the #166 recursion. So an `<<` is kept as a
    // literal key ONLY when it is alias-free AND yields nothing to merge
    // (Null / scalar / `<<: []` / `<<: [1, 2]` / `<<: {}`); then `{'<<': []}`
    // etc. round-trip instead of being silently dropped. Anything alias-bearing
    // or producing real merged pairs is consumed.
    let merge_data = merge_index
        .and_then(|index| pairs.get_index(index))
        .and_then(|(_, merge_value)| match merge_value {
            CustomNode::Null { .. } | CustomNode::Scalar { .. } => None,
            CustomNode::Alias { .. } => {
                Some(collect_merge_data(merge_value, pairs, anchors, path, depth))
            }
            CustomNode::Mapping { .. } | CustomNode::Sequence { .. } => {
                // Resolve the source before collecting from it. A `<<` inside a merge
                // source is a merge key *of that source*, and the pairs it brings are
                // content the document holds; collecting first handed that key to the
                // ownership test below, whose `contains_key` compares whole nodes
                // against a map that still contains this mapping's own `<<` entry - so
                // the nested `<<` matched, was skipped, and the source contributed
                // nothing. `<<: {<<: {x: 1}}` then kept two literal `<<` levels while
                // PyYAML and ruamel both resolve it to `x: 1`, and
                // `<<: {<<: {x: 1, y: 1}, y: 2}` dropped `x` outright: not a different
                // spelling of the same data, but data missing from the object view.
                // Resolving in place also gives the precedence the reference libraries
                // agree on: the source's own keys override what its nested merge brings
                // (`<<: {<<: {x: 1}, x: 9}` is `x: 9`), because that override happens
                // inside the source, one level at a time, before its pairs travel up.
                // The clone is paid only where a nested `<<` actually exists, and only
                // one level needs looking at: a `<<` deeper inside a value is already
                // reached by the recursive walk below (`resolve_merges_recursive` visits
                // every mapping in the tree, and the merged values are re-walked at
                // their own level), so it cannot be lost the way a top-level one can.
                let mut resolved;
                let source = if source_carries_merge_key(merge_value) {
                    resolved = merge_value.clone();
                    resolve_merges_recursive(&mut resolved, anchors, path, depth + 1);
                    &resolved
                } else {
                    merge_value
                };
                let merged = collect_merge_data(source, pairs, anchors, path, depth);
                if !merged.is_empty() || references_alias(merge_value) {
                    Some(merged)
                } else {
                    None
                }
            }
        });

    let mut expanded = None;
    // Comments carried by the merge entry — on its key, or on the value node the
    // merge consumes — are content of this mapping either way, so they re-home onto
    // the entry the merge contributes. Both halves were silent data loss behind a
    // stable text, and both were caught by the fuzz tier's note-survival oracle
    // (libFuzzer `yaml_roundtrip` crash-953bf87a, crash-f453c4e5).
    let merge_orphans: Vec<Comment> = merge_index
        .and_then(|index| pairs.get_index(index))
        .map(|(key, value)| {
            let mut notes = Vec::new();
            for carried in [key, value] {
                notes.extend(carried.leading_comments().iter().cloned());
                notes.extend(carried.comment().filter(|c| !c.standalone).cloned());
            }
            notes
        })
        .unwrap_or_default();
    // Notes of merged pairs that a mapping's own key overrides, collected while the
    // pairs are dropped below. They are content of this mapping too, so they join
    // the same re-homing as the merge entry's own notes.
    let mut overridden_notes: Vec<Comment> = Vec::new();
    if let Some(merged_pairs) = merge_data {
        // Expansions resolve against the *original* mapping, so an anchor that
        // transitively merges another still sees the latter's own merge keys.
        // `<<` survives in the expansion and is stripped on the way in.
        //
        // The first anchor to supply a key wins, both for a key repeated inside
        // one anchor and for a collision across the merge value: YAML 1.1 gives
        // each anchor its own precedence among the merge sources and lets the
        // mapping's own keys override all of them.
        let mut pending: IndexMap<CustomNode, (String, CustomNode)> =
            IndexMap::with_capacity(pairs.len());
        for (source, key, value) in merged_pairs {
            pending.entry(key).or_insert((source, value));
        }

        // The anchor name stays on the path while its expansion is walked, so a
        // merge key inside the expansion that points back at its own ancestor is
        // recognised and left alone. An inline mapping (empty source) pulls in no
        // anchor, so there is nothing to guard against.
        for value in pending.values_mut() {
            if value.0.is_empty() {
                resolve_merges_recursive(&mut value.1, anchors, path, depth + 1);
            } else {
                path.push(value.0.clone());
                resolve_merges_recursive(&mut value.1, anchors, path, depth + 1);
                path.pop();
            }
        }

        expanded = Some(
            pending
                .into_iter()
                // A mapping's own key always wins over a merged one (YAML 1.1), and
                // keeping both was not merely wrong precedence: `IndexMap` keys whole
                // nodes, so a merged `y` and an own `y # :` sat side by side and the
                // writer emitted the same key twice at one level — text our own
                // parser then rejected as a duplicate key, breaking the contract that
                // `to_yaml` never emits unparseable output (libFuzzer `yaml_roundtrip`
                // crash-3495cc86, minimised to 19 bytes: `:` LF `# &` LF `y: #:` LF
                // `<<:` LF `  y:`). Identity here is the same one `push_node`'s
                // duplicate check uses — the emitted value of an untagged scalar key —
                // so the two rules cannot drift apart. A dropped pair's notes are
                // content, so they re-home instead of disappearing with it.
                .filter(|(key, value): &(_, (String, CustomNode))| {
                    if owns_owning_key(pairs, merge_index, key) {
                        for carried in [key, &value.1] {
                            overridden_notes.extend(carried.leading_comments().iter().cloned());
                            overridden_notes
                                .extend(carried.comment().filter(|c| !c.standalone).cloned());
                        }
                        false
                    } else {
                        true
                    }
                })
                .map(|(key, (_source, value))| (key, value))
                .collect::<IndexMap<CustomNode, CustomNode>>(),
        );
    }

    // Recursively resolve in the mapping's own nested children (never the
    // merged-in clones, already resolved above under the guard) while their
    // snapshot positions still address them.
    for index in own_indices {
        if let Some((_, value)) = pairs.get_index_mut(index) {
            resolve_merges_recursive(value, anchors, path, depth + 1);
        }
    }

    if let Some(expanded) = expanded {
        let mut orphans = merge_orphans;
        orphans.extend(overridden_notes);
        insert_merged_pairs(pairs, merge_index, expanded, orphans);
    }
}

/// Whether `pairs` already holds `key` as one of its own entries, ignoring the
/// merge entry itself (which is about to be removed).
///
/// Identity is what the serializer prints, not the whole node: two untagged scalar
/// keys with the same text collide whatever their style or carried note, because
/// they emit identically — the same rule `push_node` uses to reject a duplicate
/// key, so the merge filter and the duplicate detector can never disagree about
/// which pairs may coexist.
fn owns_owning_key(
    pairs: &IndexMap<CustomNode, CustomNode>,
    merge_index: Option<usize>,
    key: &CustomNode,
) -> bool {
    pairs
        .iter()
        .enumerate()
        .any(|(index, (own, _))| Some(index) != merge_index && same_emitted_key(own, key))
}

fn same_emitted_key(a: &CustomNode, b: &CustomNode) -> bool {
    match (a, b) {
        (
            CustomNode::Scalar {
                value: va,
                meta: ma,
                ..
            },
            CustomNode::Scalar {
                value: vb,
                meta: mb,
                ..
            },
        ) => ma.tag.is_none() && mb.tag.is_none() && va == vb,
        _ => a == b,
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
    depth: usize,
) -> AnchoredPairs {
    let mut merged_pairs = AnchoredPairs::new();

    match merge_value {
        CustomNode::Alias { name } => {
            collect_merged_pairs_for_anchor(name, pairs, anchors, path, depth, &mut merged_pairs);
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                match item {
                    CustomNode::Alias { name } => collect_merged_pairs_for_anchor(
                        name,
                        pairs,
                        anchors,
                        path,
                        depth,
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
    depth: usize,
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
    // The snapshot is taken before any resolution runs, so an anchor body that
    // merges for itself is one step stale here - and a `<<` read from that stale
    // copy hits the same whole-node ownership test below and is skipped, which is
    // the alias-site twin of the inline-source defect: a template chain
    // (`use: {<<: *m}` over `mid: &m {<<: *b, y: 2}`) silently loses the keys it
    // inherits, and the emission is stable, so no round-trip assertion sees it.
    // Resolving the body here puts the nested merge inside the source, where the
    // source's own keys override it, exactly as the inline path now does. The
    // anchor's name joins the path for that walk: an expansion that reaches back
    // into its own body must terminate, and `depth` keeps the hard budget honest.
    let mut resolved;
    let body = if depth < MAX_MERGE_DEPTH && merged.keys().any(is_merge_key) {
        resolved = merged.clone();
        let mut nested = path.to_vec();
        nested.push(name.to_string());
        resolve_mapping_merges(&mut resolved, anchors, &mut nested, depth + 1);
        &resolved
    } else {
        merged
    };
    for (k, v) in body {
        if !pairs.contains_key(k) {
            let mut key = k.clone();
            let mut value = v.clone();
            clear_source_ranges(&mut key);
            clear_source_ranges(&mut value);
            result.push((name.to_string(), key, value));
        }
    }
}

/// Remove the merge key, insert the merged pairs in the slot the merge key occupied,
/// and re-home the comments `orphans` carries onto the entry the merge contributed.
fn insert_merged_pairs(
    pairs: &mut IndexMap<CustomNode, CustomNode>,
    merge_index: Option<usize>,
    merged_pairs: IndexMap<CustomNode, CustomNode>,
    orphans: Vec<Comment>,
) {
    // Merged pairs land where the `<<:` stood, not at the front of the map. Removing
    // the merge key shifts every later pair down by one, so reusing that index puts
    // them back at the position the author wrote. Prepending (`shift_insert(0, ..)`)
    // was the one reorder that could invert an anchor against its alias: a source
    // that defines `&b` on an earlier own key and uses `*b` inside the merged map is
    // valid YAML, and prepending moved the use above the definition, so `to_yaml`
    // emitted text its own parser rejected (`crash-9b77aea4`). A mapping whose merge
    // key is already first is unaffected — slot 0 is what prepend already did.
    let slot = merge_index.unwrap_or(0);
    if let Some(index) = merge_index {
        pairs.shift_remove_index(index);
    }

    // `merged_pairs` is filtered against the mapping's own keys by
    // [`owns_owning_key`] in the caller, so `shift_insert` cannot collide — without
    // that filter the emission repeated a key and our own reader rejected it. Each
    // insert grows the map by one, so `slot + offset` stays within range.
    for (offset, (k, v)) in merged_pairs.into_iter().enumerate() {
        pairs.shift_insert(slot + offset, k, v);
    }

    if orphans.is_empty() {
        return;
    }
    // `IndexMap` never hands out `&mut K`, so take the entry out, add the notes to
    // its key, and put it back on top.
    if let Some((mut key, value)) = pairs.shift_remove_index(slot) {
        for note in orphans {
            key.push_leading_comment(note);
        }
        pairs.shift_insert(slot, key, value);
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
    fn merged_pairs_take_the_merge_keys_own_slot() {
        // `crash-9b77aea4` (78 bytes, minimised to 15). An *own* key defines the
        // anchor that a later merge contributes, and the expansion inserted the
        // merged pair at the front of the map -- so the emission used `*b` before
        // `&b` was defined and our own reader rejected its own output
        // ("found unknown anchor"), breaking the contract that `to_yaml` never
        // emits unparseable text. The merged pair belongs in the slot the `<<:`
        // occupied: that is where the source put it, and it keeps every anchor
        // definition ahead of the aliases that came after it.
        let yaml = "a: &b 1\n<<:\n <: *b\n";
        let mut root = parse(yaml, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);

        let text = crate::serializer::to_yaml(&root);
        assert_eq!(text, "a: &b 1\n<: *b\n");
        crate::parser::parse(&text, YamlSchema::Core)
            .unwrap_or_else(|e| panic!("emission failed to re-parse: {e}\ntext={text:?}"));
    }

    #[test]
    fn merge_expansion_never_emits_an_alias_before_its_anchor() {
        // The general invariant, over both the readable shape and the exact bytes
        // the fuzzer minimised. A valid source defines every anchor before any use
        // of it, so preserving authored order is enough to keep that true after
        // expansion -- prepending was the one operation that could invert it.
        for yaml in [
            "a: &b 1\n<<:\n <: *b\n",
            ": &b\n<<:\n <: *b\n",
            "k: &b 2\nn: &c 3\n<<:\n p: *b\n q: *c\n",
        ] {
            let mut root = parse(yaml, YamlSchema::Core).unwrap();
            resolve_merge_keys(&mut root);
            let text = crate::serializer::to_yaml(&root);
            crate::parser::parse(&text, YamlSchema::Core).unwrap_or_else(|e| {
                panic!("source {yaml:?} expanded to unparseable {text:?}: {e}")
            });
            let again =
                crate::serializer::to_yaml(&crate::parser::parse(&text, YamlSchema::Core).unwrap());
            assert_eq!(again, text, "expansion is not a fixed point for {yaml:?}");
        }
    }

    #[test]
    fn test_literal_merge_key_with_non_merge_value_is_kept() {
        // A `<<` that is not an effective merge stays an ordinary key so it
        // round-trips: Null / plain-scalar values, and inline Mapping/Sequence
        // values that yield nothing to merge (`<<: []`, `<<: [1, 2]`). Surfaced
        // by the round-trip property fuzz (`{'<<': None}` and `{'<<': []}` were
        // silently dropped). Alias-valued `<<` (even resolving to nothing) is
        // still consumed -- the #166 guard tests cover that.
        for yaml in [
            "<<: null\n",
            "<<: 1\n",
            "k: 1\n<<: 2\n",
            "<<: []\n",
            "a: 1\n<<: [1, 2]\n",
            "<<: {}\n",
        ] {
            let mut root = parse(yaml, YamlSchema::Core).unwrap();
            resolve_merge_keys(&mut root);
            let pairs = get_mapping(&root);
            assert!(
                pairs.contains_key(&make_scalar("<<")),
                "`<<` with a non-merge value must remain a key in {yaml:?}"
            );
        }
    }

    /// A note riding the merge key is metadata, not identity. Whole-node comparison
    /// made `<<` invisible to the merge pass whenever a comment sat on the key, so the
    /// same document meant two different things depending on where the note was:
    /// `<<: #*<LF>  y:` kept a literal `<<`, while the spelling the writer emits for it
    /// (`<<:<LF>  y: ~  # *`) consumed the merge instead — and the pair then vanished
    /// between rounds (libFuzzer `yaml_roundtrip` crash-69931a77, minimised to 10
    /// bytes; crash-0a6fe677, crash-2d3dab18 and crash-f88c2382 replay CRASH→CLEAN with
    /// it, and all four redden together when identity goes back to whole-node
    /// equality — that is the attribution, not the shared assertion).
    #[test]
    fn a_merge_key_carrying_a_note_is_still_a_merge_key() {
        let src = "<<: #*\n  y:";
        let mut root = parse(src, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);
        let pairs = get_mapping(&root);
        assert!(
            !pairs.keys().any(is_merge_key),
            "the merge entry is consumed, note and all: {root:?}"
        );
        assert_eq!(pairs.len(), 1, "only the merged pair survives: {root:?}");
        assert_eq!(get_scalar_value(pairs.iter().next().unwrap().0), "y");

        // And one emission step is already the fixed point.
        let one = crate::serializer::to_yaml(&root);
        let again = parse(&one, YamlSchema::Core).unwrap();
        assert_eq!(
            crate::serializer::to_yaml(&again),
            one,
            "{src:?} must settle in one step: {one:?}"
        );
    }

    /// The other half of the identity rule: style and tag still decide. A quoted
    /// `"<<"` and a tagged `!x <<` are not merge keys, so they keep their nested
    /// mapping as an ordinary value — pinned so the metadata fix cannot widen into
    /// re-merging what YAML does not resolve as a merge.
    #[test]
    fn a_quoted_or_tagged_merge_lookalike_stays_an_ordinary_key() {
        for src in ["\"<<\": #*\n  y: 1\n", "!x <<:\n  y: 1\n"] {
            let mut root = parse(src, YamlSchema::Core).unwrap();
            resolve_merge_keys(&mut root);
            let pairs = get_mapping(&root);
            assert_eq!(pairs.len(), 1, "{src:?} keeps its single entry: {root:?}");
            assert!(
                matches!(pairs.iter().next().unwrap().1, CustomNode::Mapping { .. }),
                "{src:?} keeps the nested mapping as its value: {root:?}"
            );
            let one = crate::serializer::to_yaml(&root);
            let again = parse(&one, YamlSchema::Core).unwrap();
            assert_eq!(
                crate::serializer::to_yaml(&again),
                one,
                "{src:?} must settle in one step: {one:?}"
            );
        }
    }

    /// Every note text attached anywhere in `node`.
    fn gathered_notes(node: &CustomNode, out: &mut Vec<String>) {
        if let Some(c) = node.comment() {
            out.push(c.text.to_string());
        }
        for c in node.leading_comments() {
            out.push(c.text.to_string());
        }
        match node {
            CustomNode::Mapping { pairs, .. } => {
                for (k, v) in pairs.iter() {
                    gathered_notes(k, out);
                    gathered_notes(v, out);
                }
            }
            CustomNode::Sequence { items, .. } => {
                for item in items {
                    gathered_notes(item, out);
                }
            }
            _ => {}
        }
    }

    /// The 19-byte minimisation of libFuzzer `yaml_roundtrip` crash-3495cc86: an
    /// untagged key carrying a note, a `<<` entry, and a merged key with the same
    /// emitted text. `IndexMap` compares whole nodes, so the merged pair sat beside
    /// the own pair instead of being overridden, and `to_yaml` printed the key twice
    /// at one level — text our own parser then refused as a duplicate key, breaking
    /// the engine's "we never emit unparseable output" contract. The mapping's own
    /// key wins (YAML 1.1), the merged pair is dropped, and its notes are re-homed
    /// rather than lost with it.
    #[test]
    fn a_merge_never_repeats_a_key_the_mapping_owns() {
        let src = std::str::from_utf8(include_bytes!(
            "../../../../../fuzz/seeds/yaml_roundtrip/former-crash-3495cc86.seed"
        ))
        .expect("seed is utf-8");
        let parsed = parse(src, YamlSchema::Core).expect("input parses");

        let mut wanted = Vec::new();
        gathered_notes(&parsed, &mut wanted);

        let one = crate::serializer::to_yaml(&parsed);
        assert_eq!(
            one.matches("\ny:").count(),
            1,
            "the key `y` may appear once at this level: {one:?}"
        );
        let again = parse(&one, YamlSchema::Core)
            .unwrap_or_else(|e| panic!("our own emission must re-parse: {e}\n---\n{one}\n---"));
        let twice = crate::serializer::to_yaml(&again);
        assert_eq!(twice, one, "one emission is the fixed point: {one:?}");

        let mut kept = Vec::new();
        gathered_notes(&again, &mut kept);
        for text in &wanted {
            assert!(
                one.contains(text.as_str()),
                "note {text:?} was lost; wanted {wanted:?}, got {kept:?} in {one:?}"
            );
        }
    }

    /// A comment hung on a `<<` line outlives the merge that consumes the key: it
    /// moves onto the entry the merge contributes. Losing it is silent data loss in
    /// a shape whose text stays stable, so nothing but the fuzz tier's
    /// note-survival oracle could see it — which is exactly how it surfaced
    /// (libFuzzer `yaml_roundtrip` crash-953bf87a), one step behind the merge-key
    /// identity fix that made the key visible to the pass at all.
    #[test]
    fn a_consumed_merge_key_re_homes_its_comments() {
        let src = "chi:\n  #&ld:\n  <<:\n    y: 2\n";
        let mut root = parse(src, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);
        let one = crate::serializer::to_yaml(&root);
        assert!(
            one.contains("&ld:"),
            "the merge key's note survives: {src:?} -> {one:?}"
        );
        let again = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).unwrap());
        assert_eq!(again, one, "{one:?} must be a fixed point: {again:?}");

        // The other half: notes riding the lines of the merge VALUE node, which the
        // merge consumes along with that node (crash-f453c4e5).
        let src = "<<: #*&&&:\n#:\n#note-two\n y:\n";
        let mut root = parse(src, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);
        let one = crate::serializer::to_yaml(&root);
        assert!(
            one.contains("*&&&:") && one.contains("note-two"),
            "the merge value's notes survive: {src:?} -> {one:?}"
        );
        let again = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).unwrap());
        assert_eq!(again, one, "{one:?} must be a fixed point: {again:?}");
    }

    /// A fold that removes an entry also removes the anchor that entry *defined*, and
    /// every alias still pointing at it then names a node the document no longer
    /// contains. `<<: &b` — a literal, null-valued `<<`, which per our own rule is an
    /// ordinary key — folded against the real merge `<<:` below it, so the emission
    /// used `*b` without ever defining `&b`: text our own parser rejects
    /// (`found unknown anchor`), breaking the "we never emit unparseable output"
    /// contract (libFuzzer `yaml_roundtrip`, crash-43eca7a3, 18 bytes).
    #[test]
    fn a_folded_merge_entrys_anchor_survives_the_fold() {
        for src in [
            "\n<<: &b\n<<:\n <: *b",
            "<<: &b\n<<: {k: *b}",
            "i: &c 1\n<<: &b\n<<: {j: *b}\nk: *c",
        ] {
            let mut root = parse(src, YamlSchema::Core).unwrap();
            resolve_merge_keys(&mut root);
            let one = crate::serializer::to_yaml(&root);
            let re = parse(&one, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("{src:?} expanded to unparseable {one:?}: {e}"));
            let again = crate::serializer::to_yaml(&re);
            assert_eq!(again, one, "{one:?} must be a fixed point: {again:?}");
        }
    }

    /// A mapping can spell its merge key twice — once plain, once carrying a note —
    /// and `IndexMap` kept both while the merge pass addresses a single entry, so one
    /// pair vanished every round. Folding merge keys exactly like null keys is what
    /// re-reading the emitted text actually yields. Read from the committed seed so
    /// the test cannot drift from the bytes that crashed (libFuzzer `yaml_roundtrip`
    /// crash-973bd522).
    #[test]
    fn duplicate_merge_keys_fold_to_a_fixed_point() {
        let artifact =
            include_bytes!("../../../../../fuzz/seeds/yaml_roundtrip/former-crash-973bd522.seed");
        let src = String::from_utf8(artifact.to_vec()).expect("seed is valid utf-8");
        let mut root = parse(&src, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);
        let one = crate::serializer::to_yaml(&root);
        let again = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).unwrap());
        assert_eq!(
            again, one,
            "{src:?} must settle in one step: {one:?} -> {again:?}"
        );
    }

    /// A merge source that itself carries a `<<` key has a merge key *of that source*,
    /// and the pairs it brings are content the document holds. The collectors skip a
    /// source key the target already owns by comparing whole nodes - and the target's own
    /// `<<` entry is still in the map at that moment, so a nested `<<` was dropped rather
    /// than applied: `<<: {<<: {x: 1}}` resolved to two literal `<<` levels, and
    /// `<<: {<<: {x: 1, y: 1}, y: 2}` lost `x` outright. Both PyYAML and ruamel resolve
    /// the nesting to `x: 1` and to `x: 1` + `y: 2` (measured, not assumed), so this was
    /// not a different spelling of the same data - the pair was missing from the object
    /// view. `parse` alone, with no explicit second pass: the claim is that ONE parse
    /// reaches the answer, which is what a caller and the fuzz tier get.
    #[test]
    fn a_merge_source_that_itself_merges_contributes_its_nested_pairs() {
        for (src, want) in [
            ("<<: {<<: {x: 1}}", "x: 1\n"),
            ("<<:\n <<:\n   x: 1\n", "x: 1\n"),
            ("<<:\n <<:\n   <<:\n     x: 1\n", "x: 1\n"),
            // The source's own key still overrides what its nested merge contributes.
            ("<<: {<<: {x: 1}, x: 9}\n", "x: 9\n"),
            // A `<<` nested inside a VALUE travels with it: the guard looks one level
            // down because that is the only place a pair can be lost - the recursive
            // walk already visits every mapping deeper in the tree (measured against
            // PyYAML, which reads all four of these the same way).
            ("<<: {a: {<<: {x: 1}}}", "a: {x: 1}\n"),
            ("<<: {<<: {a: {<<: {x: 1}}}}", "a: {x: 1}\n"),
        ] {
            let root = parse(src, YamlSchema::Core).unwrap();
            let one = crate::serializer::to_yaml(&root);
            assert_eq!(one, want, "{src:?} must resolve the nesting in one parse");
            let again = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).unwrap());
            assert_eq!(again, one, "{one:?} must be a fixed point: {again:?}");
        }
    }

    /// Precedence inside the nesting, read off both reference libraries: a nested merge
    /// contributes the keys the source does not name itself, and the source's own `y`
    /// overrides the nested `y` - so the pair set is `x: 1` plus `y: 2`, and the order is
    /// the nested pair first because it arrives in the slot the `<<` occupied.
    #[test]
    fn a_nested_merge_adds_the_keys_the_source_does_not_name() {
        let src = "<<: {<<: {x: 1, y: 1}, y: 2}\n";
        let root = parse(src, YamlSchema::Core).unwrap();
        let one = crate::serializer::to_yaml(&root);
        assert_eq!(one, "x: 1\ny: 2\n", "{src:?} resolves to both keys");
        let again = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).unwrap());
        assert_eq!(again, one, "{one:?} must be a fixed point: {again:?}");
    }

    /// The fuzz backlog's shape C (`<<:` / ` <<: #b` / `  :`, minimised from
    /// crash-d0745105): the note rode the *nested* merge key, and because the skip test
    /// compared whole nodes, the note decided whether a level was consumed at all - the
    /// first emission kept two levels, the re-read of its own output collapsed to one.
    /// Same document, two meanings, depending on where a comment sat: exactly the class
    /// [`is_merge_key`] was rewritten for (crash-69931a77), at its sibling site.
    #[test]
    fn a_nested_merge_key_carrying_a_note_means_the_same_thing() {
        let src = "<<:\n <<: #b\n  :";
        let root = parse(src, YamlSchema::Core).unwrap();
        let one = crate::serializer::to_yaml(&root);
        assert_eq!(one, "# b\n~: ~\n", "{src:?} collapses both levels at once");
        let again = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).unwrap());
        assert_eq!(again, one, "{one:?} must settle in one round: {again:?}");
        // The spelling without the note means the same thing, which is the property the
        // whole fix is about: metadata must not decide how a document resolves.
        let plain = crate::serializer::to_yaml(&parse("<<:\n <<:\n  :", YamlSchema::Core).unwrap());
        assert_eq!(
            plain, "~: ~\n",
            "the note-free spelling resolves the same way"
        );
    }

    /// The same discard at the other collection site, and the one users hit: a template
    /// chain. `collect_anchor_mappings` snapshots every anchor body *before* any merge
    /// resolution runs, and the collector reads that snapshot, so a `<<` inside the body
    /// is handed to the same whole-node ownership test and skipped. Measured against
    /// PyYAML: `use` lost the `x: 1` it inherits through `mid`, and a three-level chain
    /// lost two keys. The text is stable either way, so the round-trip tier is blind to
    /// this - only the object view shows it.
    #[test]
    fn an_anchor_that_itself_merges_contributes_the_inherited_keys() {
        let src = "base: &b {x: 1}\nmid: &m {<<: *b, y: 2}\nuse:\n  <<: *m\n  z: 3\n";
        let root = parse(src, YamlSchema::Core).unwrap();
        let use_pairs = get_mapping(
            get_mapping(&root)
                .get(&make_scalar("use"))
                .expect("`use` is in the document"),
        );
        for (key, value) in [("x", "1"), ("y", "2"), ("z", "3")] {
            let found = use_pairs.get(&make_scalar(key)).unwrap_or_else(|| {
                panic!("{src:?} inherits `{key}` through the chain: {use_pairs:?}")
            });
            assert_eq!(get_scalar_value(found), value, "{key} came from the chain");
        }
        assert!(
            !use_pairs.keys().any(is_merge_key),
            "no stray merge key left behind: {use_pairs:?}"
        );
        let one = crate::serializer::to_yaml(&root);
        let again = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).unwrap());
        assert_eq!(again, one, "{one:?} must be a fixed point: {again:?}");
    }

    /// Three levels deep, which is where the staleness compounds: each body's snapshot
    /// is one resolution behind, so `c` lost `p` and the document's own merge lost both
    /// `p` and `q`. PyYAML resolves the whole chain (`p`, `q`, `r` at the top level).
    #[test]
    fn a_three_level_anchor_chain_resolves_to_its_deepest_keys() {
        let src = "a: &A {p: 1}\nb: &B {<<: *A, q: 2}\nc: &C {<<: *B, r: 3}\n<<: *C\n";
        let root = parse(src, YamlSchema::Core).unwrap();
        let top = get_mapping(&root);
        for key in ["p", "q", "r"] {
            assert!(
                top.contains_key(&make_scalar(key)),
                "{src:?} must carry {key} at the top level: {top:?}"
            );
        }
        assert!(
            !top.keys().any(is_merge_key),
            "the merge entry is consumed, not left literal: {top:?}"
        );
        let c = get_mapping(top.get(&make_scalar("c")).unwrap());
        assert!(
            c.contains_key(&make_scalar("p")),
            "`c` inherits `p` through two levels: {c:?}"
        );
        let one = crate::serializer::to_yaml(&root);
        let again = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).unwrap());
        assert_eq!(again, one, "{one:?} must be a fixed point: {again:?}");
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

    /// libFuzzer `parse_yaml` stack-overflow (58 bytes): nested self-anchors
    /// (`bas: &b` whose body re-`use`s `&b` / `<<: *b`) fed the tail recursion
    /// with the path guard already popped, so each walk re-expanded a fresh
    /// clone and the descent never bottomed out — ASAN reported a stack-overflow
    /// on `resolve_mapping_merges -> resolve_merges_recursive` frames. The tail
    /// walk now skips merged-in clones and a depth budget bounds any residual
    /// runaway, so this hostile input must resolve (or degrade) without panicking
    /// or overflowing the native stack.
    #[test]
    fn nested_self_anchor_merge_terminates() {
        let yaml = "bas: &b\n l)bas: &b\n l)##d:\n  <<: *b\n #y:##d:\n  <<: *b\n #y:";
        let mut root = parse(yaml, YamlSchema::Core).unwrap();
        resolve_merge_keys(&mut root);
        // Re-parse the resolved tree: serialization output must survive, proving
        // the guard produced a well-formed (if degraded) AST rather than a crash.
        let out = crate::serializer::to_yaml(&root);
        let _ = parse(&out, YamlSchema::Core);
    }
}
