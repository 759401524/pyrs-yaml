//! AST navigation for edit path resolution.
//!
//! Pure Rust implementation — no PyO3 dependencies.
//! Python-specific segment parsing is in `crate::py::editing`.

use crate::ast::CustomNode;
use crate::error::PathError;
use indexmap::IndexMap;
use std::borrow::Cow;

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
        (CustomNode::Scalar { value: av, .. }, CustomNode::Scalar { value: bv, .. }) => av == bv,
        (CustomNode::Null { .. }, CustomNode::Null { .. }) => true,
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

pub fn mapping_get_mut<'a>(
    pairs: &'a mut IndexMap<CustomNode, CustomNode>,
    key: &CustomNode,
) -> Option<&'a mut CustomNode> {
    let idx = pairs.iter().position(|(k, _)| key_eq(k, key))?;
    pairs.get_index_mut(idx).map(|(_, v)| v)
}

pub fn mapping_key_index(
    pairs: &IndexMap<CustomNode, CustomNode>,
    key: &CustomNode,
) -> Option<usize> {
    pairs.iter().position(|(k, _)| key_eq(k, key))
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

/// ```
/// use pyrs_yaml_core::editing::parse_path_segments;
/// let segs = parse_path_segments("$.a.b").unwrap();
/// assert_eq!(segs.len(), 2);
/// ```
pub fn parse_path_segments(path: &str) -> Result<Vec<Segment<'_>>, PathError> {
    let rest = path.strip_prefix('$').unwrap_or(path);
    let rest = rest.strip_prefix('.').unwrap_or(rest);
    if rest.starts_with('.') {
        return Err(PathError::WildcardOrDeepScan);
    }
    let mut segments = Vec::new();
    let mut chars = rest.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            '.' => {
                chars.next();
            }
            '[' => {
                chars.next();
                let mut num = String::new();
                if matches!(chars.peek(), Some('-')) {
                    num.push('-');
                    chars.next();
                }
                while matches!(chars.peek(), Some(d) if d.is_ascii_digit()) {
                    num.push(chars.next().expect("peeked digit"));
                }
                if chars.next() != Some(']') || num.is_empty() || num == "-" {
                    return Err(PathError::InvalidIndex(num));
                }
                let idx: i64 = num
                    .parse()
                    .map_err(|_| PathError::InvalidIndex(num.clone()))?;
                segments.push(Segment::Index(idx));
            }
            '*' => return Err(PathError::WildcardOrDeepScan),
            _ => {
                let mut key = String::new();
                while let Some(&ch) = chars.peek() {
                    if matches!(ch, '.' | '[' | '*' | '$') {
                        break;
                    }
                    key.push(ch);
                    chars.next();
                }
                if key.is_empty() {
                    return Err(PathError::InvalidPath);
                }
                segments.push(Segment::Key(Cow::Owned(key)));
            }
        }
    }
    Ok(segments)
}

pub use crate::error::NavigateError;

/// ```
/// use pyrs_yaml_core::ast::CustomNode;
/// use pyrs_yaml_core::editing::{navigate, Segment, key_eq};
/// use std::borrow::Cow;
/// use indexmap::IndexMap;
/// let mut pairs = IndexMap::new();
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
                pairs
                    .iter()
                    .find(|(knode, _)| key_eq(knode, &key_node))
                    .map(|(_, v)| v)
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
/// use indexmap::IndexMap;
/// let mut pairs = IndexMap::new();
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

    fn mk_scalar(s: &str) -> CustomNode {
        CustomNode::plain_scalar(s)
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
    fn test_key_eq_same_scalar() {
        assert!(key_eq(&mk_scalar("a"), &mk_scalar("a")));
    }

    #[test]
    fn test_key_eq_different_scalar() {
        assert!(!key_eq(&mk_scalar("a"), &mk_scalar("b")));
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
        let map: IndexMap<CustomNode, CustomNode> = pairs.into_iter().collect();
        assert_eq!(mapping_key_index(&map, &mk_scalar("a")), Some(0));
        assert_eq!(mapping_key_index(&map, &mk_scalar("b")), Some(1));
    }

    #[test]
    fn test_mapping_key_index_not_found() {
        let pairs = vec![(mk_scalar("a"), mk_scalar("1"))];
        let map: IndexMap<CustomNode, CustomNode> = pairs.into_iter().collect();
        assert_eq!(mapping_key_index(&map, &mk_scalar("x")), None);
    }

    #[test]
    fn test_navigate_mapping_key() {
        let node = mk_map(vec![("a", "1")]);
        let segs = [Segment::Key(Cow::Borrowed("a"))];
        let result = navigate(&node, &segs).unwrap();
        assert!(key_eq(result, &mk_scalar("1")));
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
