//! JSONPath-lite selectors: `$`, dot keys, bracket keys/indices, wildcards.
//! Segment model mirrors the `Node.find_first` semantics of the Python API.

use pyrs_yaml_core::ast::CustomNode;

#[derive(Debug, Clone, PartialEq)]
enum Seg {
    Key(String),
    Index(i64),
    Wildcard,
}

/// A parsed selector, e.g. `$.servers[*].host`.
#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    segs: Vec<Seg>,
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    let bytes = t.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\'')
            || (bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"'))
    {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

/// Parse `.a.b[0]`, `$.a.b`, `[*]`, `a."x.y"` (quoted brackets allow dots).
pub fn parse_path(input: &str) -> Result<Selector, String> {
    let s = input.trim();
    let mut segs = Vec::new();
    let mut rest = s.strip_prefix('$').unwrap_or(s);
    if rest.is_empty() {
        return Ok(Selector { segs });
    }
    while !rest.is_empty() {
        if let Some(tail) = rest.strip_prefix('.') {
            rest = tail;
            if rest.is_empty() {
                return Err(format!("trailing '.' in path: {input}"));
            }
            // `..` deep-scan is intentionally not in v1.
            let token: String = rest
                .chars()
                .take_while(|c| *c != '.' && *c != '[')
                .collect();
            if token.is_empty() {
                return Err(format!("empty key segment in path: {input}"));
            }
            let consumed = token.len();
            match token.as_str() {
                "*" => segs.push(Seg::Wildcard),
                _ => segs.push(Seg::Key(token)),
            }
            rest = &rest[consumed..];
        } else if let Some(tail) = rest.strip_prefix('[') {
            let close = tail
                .find(']')
                .ok_or_else(|| format!("unclosed '[' in path: {input}"))?;
            let inner = &tail[..close];
            if inner == "*" {
                segs.push(Seg::Wildcard);
            } else if let Ok(i) = inner.parse::<i64>() {
                segs.push(Seg::Index(i));
            } else {
                segs.push(Seg::Key(unquote(inner)));
            }
            rest = &tail[close + 1..];
        } else {
            return Err(format!("unexpected character in path: {input}"));
        }
    }
    Ok(Selector { segs })
}

impl Selector {
    /// First match, resolved through plain mappings/sequences only (aliases
    /// and typed collections are out of the v1 surface).
    pub fn select<'a>(&self, root: &'a CustomNode) -> Result<Option<&'a CustomNode>, String> {
        let mut current = root;
        for seg in &self.segs {
            current = match (seg, current) {
                (Seg::Key(k), CustomNode::Mapping { pairs, .. }) => pairs
                    .iter()
                    .find_map(|(key, value)| matches_key(key, k).then_some(value))
                    .ok_or_else(|| format!("no key {k}"))?,
                (Seg::Index(i), CustomNode::Sequence { items, .. }) => {
                    let idx = if *i < 0 {
                        (items.len() as i64 + i) as usize
                    } else {
                        *i as usize
                    };
                    items
                        .get(idx)
                        .ok_or_else(|| format!("index {i} out of range"))?
                }
                (Seg::Wildcard, CustomNode::Mapping { pairs, .. }) => pairs
                    .first()
                    .map(|(_, v)| v)
                    .ok_or_else(|| "wildcard on empty mapping".to_string())?,
                (Seg::Wildcard, CustomNode::Sequence { items, .. }) => items
                    .first()
                    .ok_or_else(|| "wildcard on empty sequence".to_string())?,
                _ => return Err(format!("cannot descend with {seg:?}")),
            };
        }
        Ok(Some(current))
    }
}

fn matches_key(key: &CustomNode, want: &str) -> bool {
    matches!(key, CustomNode::Scalar { value, .. } if value.as_ref() == want)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyrs_yaml_core::parser::{parse, yaml::Schema};

    fn doc() -> CustomNode {
        parse(
            "servers:\n  - host: a\n    port: 1\n  - host: b\nempty: {}\n",
            Schema::Core,
        )
        .unwrap()
    }

    #[test]
    fn dot_and_bracket_paths() {
        let d = doc();
        assert_eq!(
            parse_path("$.servers[1].host")
                .unwrap()
                .select(&d)
                .unwrap()
                .map(node_text),
            Some("b".to_string())
        );
        assert_eq!(
            parse_path(".servers[-2].port")
                .unwrap()
                .select(&d)
                .unwrap()
                .map(node_text),
            Some("1".to_string())
        );
        assert_eq!(
            parse_path("$['servers'][0]['host']")
                .unwrap()
                .select(&d)
                .unwrap()
                .map(node_text),
            Some("a".to_string())
        );
    }

    #[test]
    fn root_and_misses() {
        let d = doc();
        assert!(parse_path("$").unwrap().select(&d).unwrap().is_some());
        assert!(parse_path("$.nope").unwrap().select(&d).is_err());
        assert!(parse_path("$.empty.x").unwrap().select(&d).is_err());
    }

    fn node_text(n: &CustomNode) -> String {
        match n {
            CustomNode::Scalar { value, .. } => value.to_string(),
            _ => "<node>".to_string(),
        }
    }
}
