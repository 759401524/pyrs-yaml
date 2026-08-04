use indexmap::IndexMap;
use proptest::prelude::*;
use proptest::strategy::Union;
use std::sync::Arc;

use crate::ast::CustomNode;
use crate::parser::parse_with_options;
use crate::parser::yaml::YamlSchema;
use crate::py::editing::{self, Segment};
use crate::serializer::{to_yaml_with_options, SerializeOptions};
use crate::splice::SpliceState;

fn key_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9_]{0,7}").unwrap()
}

fn value_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z0-9_]{1,8}").unwrap()
}

fn leaf_strategy() -> impl Strategy<Value = CustomNode> {
    value_strategy().prop_map(CustomNode::plain_scalar)
}

fn mapping_strategy(depth: u32) -> impl Strategy<Value = CustomNode> {
    prop::collection::vec((key_strategy(), node_strategy(depth)), 1..10).prop_map(|pairs| {
        let mut map = IndexMap::new();
        for (k, v) in pairs {
            map.insert(CustomNode::plain_scalar(k), v);
        }
        CustomNode::plain_mapping(map)
    })
}

fn sequence_strategy(depth: u32) -> impl Strategy<Value = CustomNode> {
    prop::collection::vec(node_strategy(depth), 0..8).prop_map(CustomNode::plain_sequence)
}

fn node_strategy(depth: u32) -> BoxedStrategy<CustomNode> {
    if depth >= 4 {
        leaf_strategy().boxed()
    } else {
        Union::new_weighted(vec![
            (3, leaf_strategy().boxed()),
            (1, mapping_strategy(depth + 1).boxed()),
            (1, sequence_strategy(depth + 1).boxed()),
        ])
        .boxed()
    }
}

fn doc_strategy() -> impl Strategy<Value = String> {
    mapping_strategy(0).prop_map(|node| {
        let yaml = to_yaml_with_options(&node, &SerializeOptions::default())
            .expect("serialization of generated AST must succeed");
        if yaml.ends_with('\n') {
            yaml
        } else {
            yaml + "\n"
        }
    })
}

fn collect_keys(node: &CustomNode, prefix: &str, keys: &mut Vec<String>) {
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs {
                if let CustomNode::Scalar { value: k, .. } = key {
                    let path = if prefix.is_empty() {
                        format!("$.{k}")
                    } else {
                        format!("{prefix}.{k}")
                    };
                    keys.push(path.clone());
                    collect_keys(value, &path, keys);
                }
            }
        }
        CustomNode::Sequence { items, .. } => {
            for (i, item) in items.iter().enumerate() {
                collect_keys(item, &format!("{prefix}[{i}]"), keys);
            }
        }
        _ => {}
    }
}

proptest! {
    #[test]
    fn untouched_bytes_survive_edit(ref doc_str in doc_strategy()) {
        let ast = parse_with_options(doc_str, true, YamlSchema::Core, 1000, false)
            .expect("generated doc must parse");
        let source: Arc<str> = Arc::from(doc_str.as_str());

        if !crate::parser::check_default_layout(&ast, doc_str) {
            return Ok(());
        }

        let mut keys = Vec::new();
        collect_keys(&ast, "", &mut keys);
        if keys.is_empty() {
            return Ok(());
        }

        let path = keys[0].clone();
        let path_dbg = path.clone();
        let segs = editing::parse_path_segments(&path)
            .unwrap_or_else(|_| panic!("invalid path: {path_dbg}"));
        let segs: Vec<Segment<'_>> = segs.into_iter().collect();

        let mut edited_ast = ast.clone();
        let new_value = CustomNode::plain_scalar("zzz_prop");
        let unit = editing::set_path(&mut edited_ast, &segs, new_value, true, doc_str, None, false)
            .expect("set_path must succeed");

        let mut state = SpliceState::new(source);

        if unit.eligible && state.apply(&unit).is_ok() {
            if let Some(spliced) = state.materialize() {
                match &unit.kind {
                    editing::DirtyKind::Region { range, text, .. } => {
                        let expected = format!(
                            "{}{}{}",
                            &doc_str[..range.start],
                            text,
                            &doc_str[range.end..]
                        );
                        prop_assert_eq!(spliced, expected,
                            "splice must preserve untouched bytes outside the dirty region");
                    }
                    editing::DirtyKind::Insert { at, text } => {
                        let expected = format!(
                            "{}{}{}",
                            &doc_str[..*at],
                            text,
                            &doc_str[*at..]
                        );
                        prop_assert_eq!(spliced, expected,
                            "splice must preserve untouched bytes outside the insert point");
                    }
                    editing::DirtyKind::Delete { range } => {
                        let expected = format!(
                            "{}{}",
                            &doc_str[..range.start],
                            &doc_str[range.end..]
                        );
                        prop_assert_eq!(spliced, expected,
                            "splice must preserve untouched bytes outside the delete range");
                    }
                }
            }
        }
    }
}
