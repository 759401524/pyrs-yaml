//! AST navigation for edit path resolution.
//!
//! Pure Rust implementation — no PyO3 dependencies.
//! Python-specific segment parsing is in `crate::py::editing`.

use crate::ast::CustomNode;
use crate::ast::NodeMap;
use alloc::borrow::Cow;
use alloc::string::ToString;

/// ```
/// use pyrs_yaml_core::editing::Segment;
/// use std::borrow::Cow;
/// let key = Segment::Key(Cow::Borrowed("a"));
/// let idx = Segment::Index(0);
/// ```
/// A path segment for navigating into a YAML AST.
#[derive(Debug, Clone)]
pub enum Segment<'a> {
    /// Mapping key lookup.
    Key(Cow<'a, str>),
    /// Sequence index lookup.
    Index(i64),
}

/// ```
/// use pyrs_yaml_core::ast::CustomNode;
/// use pyrs_yaml_core::editing::key_eq;
/// assert!(key_eq(&CustomNode::plain_scalar("a"), &CustomNode::plain_scalar("a")));
/// assert!(!key_eq(&CustomNode::plain_scalar("a"), &CustomNode::plain_scalar("b")));
/// ```
pub fn key_eq(a: &CustomNode, b: &CustomNode) -> bool {
    match (a, b) {
        // A leaf key is what the reader resolves it to, not how it is spelled
        // (`CustomNode::key_identity`). Comparing scalar text alone got both directions wrong:
        // `~` and the `Null` node are one key to a reader that folds them, which is why a path
        // editing `$.null` on a document holding `~: 1` missed the entry and appended a second
        // line - text this library then refused as a duplicate key; and two scalars carrying
        // different tags are two keys, which text equality called one.
        (
            CustomNode::Scalar { .. } | CustomNode::Null { .. } | CustomNode::Alias { .. },
            CustomNode::Scalar { .. } | CustomNode::Null { .. } | CustomNode::Alias { .. },
        ) => a.key_identity() == b.key_identity(),
        (CustomNode::Mapping { pairs: ap, .. }, CustomNode::Mapping { pairs: bp, .. }) => {
            ap.len() == bp.len()
                && ap
                    .iter()
                    .all(|(k, v)| bp.iter().any(|(bk, bv)| key_eq(k, bk) && key_eq(v, bv)))
        }
        (CustomNode::Sequence { items: ai, .. }, CustomNode::Sequence { items: bi, .. }) => {
            ai.len() == bi.len() && ai.iter().zip(bi).all(|(x, y)| key_eq(x, y))
        }
        _ => false,
    }
}

pub fn mapping_get<'a>(
    pairs: &'a NodeMap<CustomNode, CustomNode>,
    key: &CustomNode,
) -> Option<&'a CustomNode> {
    key_position(pairs, key).and_then(|idx| pairs.get_index(idx).map(|(_, v)| v))
}

pub fn mapping_get_mut<'a>(
    pairs: &'a mut NodeMap<CustomNode, CustomNode>,
    key: &CustomNode,
) -> Option<&'a mut CustomNode> {
    let idx = key_position(pairs, key)?;
    pairs.get_index_mut(idx).map(|(_, v)| v)
}

pub fn mapping_key_index(
    pairs: &NodeMap<CustomNode, CustomNode>,
    key: &CustomNode,
) -> Option<usize> {
    key_position(pairs, key)
}

/// Where a path names a key, in the order a lookup has to try.
///
/// Two passes, and the order is the fix. Pass one asks what the reader resolves - the null fold and
/// the tags - so `$.null` on a document holding both `~: 1` and `'null': 2` reaches the folded
/// entry, which is the entry a strict re-read considers the same key, instead of the quoted one.
/// Pass two asks the spelling, and only when the fold found nothing: the writer quotes a key exactly
/// when its bare form would fold (`safe_dump({"NULL": 1})` is `'NULL': 1`), so a document whose key
/// is quoted *because* it names a null is still reachable by that name - which is what every caller
/// did before `key_identity` existed, and what a lookup that only asked the fold would lose.
fn key_position(pairs: &NodeMap<CustomNode, CustomNode>, key: &CustomNode) -> Option<usize> {
    pairs
        .iter()
        .position(|(k, _)| k.key_identity() == key.key_identity())
        .or_else(|| pairs.iter().position(|(k, _)| same_spelling(k, key)))
}

/// The leaf comparison this lookup used before the identity model: text for scalars, and any two
/// nulls alike. Used only as the fallback pass, never as the answer when the fold has one.
fn same_spelling(a: &CustomNode, b: &CustomNode) -> bool {
    match (a, b) {
        (CustomNode::Scalar { value: av, .. }, CustomNode::Scalar { value: bv, .. }) => av == bv,
        (CustomNode::Null { .. }, CustomNode::Null { .. }) => true,
        _ => false,
    }
}

/// ```
/// use pyrs_yaml_core::editing::normalize_index;
/// assert_eq!(normalize_index(0, 5), Some(0));
/// assert_eq!(normalize_index(-1, 5), Some(4));
/// assert_eq!(normalize_index(5, 5), None);
/// ```
pub fn normalize_index(index: i64, len: usize) -> Option<usize> {
    let normalized = if index < 0 {
        i64::try_from(len).ok()?.checked_add(index)?
    } else {
        index
    };
    usize::try_from(normalized).ok().filter(|&i| i < len)
}

pub use crate::error::NavigateError;

/// ```
/// use pyrs_yaml_core::ast::CustomNode;
/// use pyrs_yaml_core::editing::{navigate, Segment, key_eq};
/// use std::borrow::Cow;
/// use pyrs_yaml_core::ast::NodeMap;
/// let mut pairs = NodeMap::default();
/// pairs.insert(CustomNode::plain_scalar("a"), CustomNode::plain_scalar("1"));
/// let node = CustomNode::Mapping { pairs, flow_style: false, meta: Default::default() };
/// let segs = [Segment::Key(Cow::Borrowed("a"))];
/// let result = navigate(&node, &segs).unwrap();
/// assert!(key_eq(result, &CustomNode::plain_scalar("1")));
/// ```
pub fn navigate<'a>(
    node: &'a CustomNode,
    segments: &[Segment<'_>],
) -> Result<&'a CustomNode, NavigateError> {
    let mut cur = node;
    for seg in segments {
        cur = match (cur, seg) {
            (CustomNode::Mapping { pairs, .. }, Segment::Key(k)) => {
                let key_node = CustomNode::plain_scalar(k.as_ref());
                mapping_get(pairs, &key_node)
                    .ok_or_else(|| NavigateError::Missing(k.to_string()))?
            }
            (CustomNode::Sequence { items, .. }, Segment::Index(i)) => items
                .get(
                    normalize_index(*i, items.len())
                        .ok_or_else(|| NavigateError::Missing(i.to_string()))?,
                )
                .ok_or_else(|| NavigateError::Missing(i.to_string()))?,
            (_, Segment::Key(k)) => return Err(NavigateError::CannotDescend(k.to_string())),
            (_, Segment::Index(i)) => return Err(NavigateError::CannotDescend(i.to_string())),
        };
    }
    Ok(cur)
}

/// ```
/// use pyrs_yaml_core::ast::CustomNode;
/// use pyrs_yaml_core::editing::{navigate_mut, navigate, Segment, key_eq};
/// use std::borrow::Cow;
/// use pyrs_yaml_core::ast::NodeMap;
/// let mut pairs = NodeMap::default();
/// pairs.insert(CustomNode::plain_scalar("a"), CustomNode::plain_scalar("1"));
/// let mut node = CustomNode::Mapping { pairs, flow_style: false, meta: Default::default() };
/// let segs = [Segment::Key(Cow::Borrowed("a"))];
/// *navigate_mut(&mut node, &segs).unwrap() = CustomNode::plain_scalar("9");
/// assert!(key_eq(navigate(&node, &segs).unwrap(), &CustomNode::plain_scalar("9")));
/// ```
pub fn navigate_mut<'a>(
    node: &'a mut CustomNode,
    segments: &[Segment<'_>],
) -> Result<&'a mut CustomNode, NavigateError> {
    let mut cur = node;
    for seg in segments {
        cur = match cur {
            CustomNode::Mapping { pairs, .. } => {
                let k = match seg {
                    Segment::Key(k) => k,
                    Segment::Index(i) => return Err(NavigateError::CannotDescend(i.to_string())),
                };
                let key_node = CustomNode::plain_scalar(k.as_ref());
                mapping_get_mut(pairs, &key_node)
                    .ok_or_else(|| NavigateError::Missing(k.to_string()))?
            }
            CustomNode::Sequence { items, .. } => {
                let i = match seg {
                    Segment::Index(i) => i,
                    Segment::Key(k) => return Err(NavigateError::CannotDescend(k.to_string())),
                };
                let idx = normalize_index(*i, items.len())
                    .ok_or_else(|| NavigateError::Missing(i.to_string()))?;
                items
                    .get_mut(idx)
                    .ok_or_else(|| NavigateError::Missing(i.to_string()))?
            }
            _ => {
                return Err(match seg {
                    Segment::Key(k) => NavigateError::CannotDescend(k.to_string()),
                    Segment::Index(i) => NavigateError::CannotDescend(i.to_string()),
                });
            }
        };
    }
    Ok(cur)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::CustomNode;
    use alloc::string::String;
    use alloc::vec::Vec;

    fn mk_scalar(s: &str) -> CustomNode {
        CustomNode::plain_scalar(s)
    }

    #[test]
    fn a_leaf_key_is_the_key_the_reader_resolves() {
        // Issue #336. `~`, `NULL` and the `Null` node are one key to a reader that folds null
        // spellings, and two scalars carrying different tags are two keys - text-only comparison got
        // both wrong, which is how a path editing `$.null` missed the `~:` entry and appended a line
        // this library then refused as a duplicate key.
        let null_node = CustomNode::Null {
            meta: Default::default(),
        };
        assert!(key_eq(&null_node, &mk_scalar("~")));
        assert!(key_eq(&null_node, &mk_scalar("NULL")));
        assert!(key_eq(&mk_scalar("null"), &mk_scalar("~")));
        assert!(!key_eq(&null_node, &mk_scalar("yes")));
        assert!(!key_eq(&mk_scalar("null"), &mk_scalar("other")));

        let mut tagged_a = mk_scalar("k");
        tagged_a.set_tag(crate::ast::Tag::primary("a"));
        let mut tagged_b = mk_scalar("k");
        tagged_b.set_tag(crate::ast::Tag::primary("b"));
        assert!(!key_eq(&tagged_a, &tagged_b), "a tag is part of the line");
        assert!(key_eq(&tagged_a, &tagged_a));
    }

    fn doc(text: &str) -> CustomNode {
        crate::parser::parse(text, crate::parser::yaml::Schema::Core).expect("the fixture parses")
    }

    fn value_at(node: &CustomNode, key: &str) -> String {
        let segs = [Segment::Key(alloc::borrow::Cow::Borrowed(key))];
        match navigate(node, &segs).expect("the path finds an entry") {
            CustomNode::Scalar { value, .. } => value.as_ref().to_string(),
            other => format!("{other:?}"),
        }
    }

    #[test]
    fn a_quoted_key_that_names_a_null_is_still_reachable_by_that_name() {
        // The writer quotes a key exactly when its bare form would fold, so `safe_dump({"NULL": 1})`
        // is `'NULL': 1`. A lookup that asked only the reader's fold would answer Null for the
        // segment `NULL` and refuse the quoted entry - unreachable by any path, which is a behaviour
        // break every caller relied on (tests/test_property_editing.py caught it on CI).
        let d = doc("'NULL': 1");
        assert_eq!(value_at(&d, "NULL"), "1");
        let plain_text = doc("'a b': 3");
        assert_eq!(value_at(&plain_text, "a b"), "3");
    }

    #[test]
    fn the_fold_answers_first_when_a_document_holds_both_spellings() {
        // `~: 1` beside `'null': 2` is two entries the reader keeps apart, and every folded spelling
        // - `null`, `NULL`, `~` - names the folded one. Pass one has to win, or the fold fix would
        // edit the wrong line; the cost is that the quoted twin is then unreachable by text, which
        // is what the fold says a path segment means.
        let d = doc("~: 1\n'null': 2");
        assert_eq!(value_at(&d, "null"), "1");
        assert_eq!(
            value_at(&d, "NULL"),
            "1",
            "a folded spelling never addresses the quoted twin"
        );
    }

    fn mk_map(pairs: Vec<(&str, &str)>) -> CustomNode {
        CustomNode::Mapping {
            pairs: pairs
                .into_iter()
                .map(|(k, v)| (mk_scalar(k), mk_scalar(v)))
                .collect(),
            flow_style: false,
            meta: Default::default(),
        }
    }

    fn mk_seq(items: Vec<&str>) -> CustomNode {
        CustomNode::Sequence {
            items: items.into_iter().map(mk_scalar).collect(),
            flow_style: false,
            meta: Default::default(),
        }
    }

    #[test]
    fn test_key_eq_same_mapping() {
        let a = mk_map(vec![("x", "1")]);
        let b = mk_map(vec![("x", "1")]);
        assert!(key_eq(&a, &b));
    }

    #[test]
    fn test_key_eq_different_mapping() {
        let a = mk_map(vec![("x", "1")]);
        let b = mk_map(vec![("y", "2")]);
        assert!(!key_eq(&a, &b));
    }

    #[test]
    fn test_mapping_key_index_found() {
        let pairs = vec![
            (mk_scalar("a"), mk_scalar("1")),
            (mk_scalar("b"), mk_scalar("2")),
        ];
        let map: NodeMap<CustomNode, CustomNode> = pairs.into_iter().collect();
        assert_eq!(mapping_key_index(&map, &mk_scalar("a")), Some(0));
        assert_eq!(mapping_key_index(&map, &mk_scalar("b")), Some(1));
    }

    #[test]
    fn test_mapping_key_index_not_found() {
        let pairs = vec![(mk_scalar("a"), mk_scalar("1"))];
        let map: NodeMap<CustomNode, CustomNode> = pairs.into_iter().collect();
        assert_eq!(mapping_key_index(&map, &mk_scalar("x")), None);
    }

    #[test]
    fn test_navigate_missing_key() {
        let node = mk_map(vec![("a", "1")]);
        let segs = [Segment::Key(Cow::Borrowed("x"))];
        assert!(matches!(
            navigate(&node, &segs),
            Err(NavigateError::Missing(_))
        ));
    }

    #[test]
    fn test_navigate_sequence_index() {
        let node = mk_seq(vec!["a", "b", "c"]);
        let segs = [Segment::Index(1)];
        let result = navigate(&node, &segs).unwrap();
        assert!(key_eq(result, &mk_scalar("b")));
    }

    #[test]
    fn test_navigate_cannot_descend_scalar() {
        let node = mk_scalar("hello");
        let segs = [Segment::Key(Cow::Borrowed("x"))];
        assert!(matches!(
            navigate(&node, &segs),
            Err(NavigateError::CannotDescend(_))
        ));
    }
}
