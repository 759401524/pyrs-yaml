use crate::ast::CustomNode;
use indexmap::IndexMap;
use pyo3::prelude::*;
use std::borrow::Cow;

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

#[derive(Debug, Clone)]
pub enum Segment<'a> {
    Key(Cow<'a, str>),
    Index(i64),
}

impl<'a> Segment<'a> {
    pub fn from_py(_py: Python, obj: &Bound<'_, PyAny>) -> PyResult<Segment<'a>> {
        if let Ok(s) = obj.extract::<String>() {
            Ok(Segment::Key(Cow::Owned(s)))
        } else if let Ok(i) = obj.extract::<i64>() {
            Ok(Segment::Index(i))
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(
                "segment must be str or int",
            ))
        }
    }
}

pub fn normalize_index(index: i64, len: usize) -> Option<usize> {
    let normalized = if index < 0 {
        i64::try_from(len).ok()?.checked_add(index)?
    } else {
        index
    };
    usize::try_from(normalized).ok().filter(|&i| i < len)
}

pub fn parse_path_segments(path: &str) -> Result<Vec<Segment<'_>>, String> {
    let rest = path.strip_prefix('$').unwrap_or(path);
    let rest = rest.strip_prefix('.').unwrap_or(rest);
    if rest.starts_with('.') {
        return Err("wildcard-or-deep-scan".to_string());
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
                    return Err(format!("invalid-index:{num}"));
                }
                let idx: i64 = num.parse().map_err(|_| format!("invalid-index:{num}"))?;
                segments.push(Segment::Index(idx));
            }
            '*' => return Err("wildcard-or-deep-scan".to_string()),
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
                    return Err("invalid-path".to_string());
                }
                segments.push(Segment::Key(Cow::Owned(key)));
            }
        }
    }
    Ok(segments)
}

#[derive(Debug)]
pub enum NavigateError {
    Missing(String),
    CannotDescend(String),
    NotContainer,
}

pub fn navigate<'a>(
    node: &'a CustomNode,
    segments: &[Segment<'_>],
) -> Result<&'a CustomNode, NavigateError> {
    let mut cur = node;
    for seg in segments {
        cur = match (cur, seg) {
            (CustomNode::Mapping { pairs, .. }, Segment::Key(k)) => pairs
                .iter()
                .find(|(knode, _)| match knode {
                    CustomNode::Scalar { value, .. } => value == k.as_ref(),
                    _ => false,
                })
                .map(|(_, v)| v)
                .ok_or_else(|| NavigateError::Missing(k.to_string()))?,
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
                })
            }
        };
    }
    Ok(cur)
}
