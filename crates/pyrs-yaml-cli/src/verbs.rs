//! Query post-processing verbs: structured flags instead of an
//! expression language (design: docs/superpowers/specs/pyq-verbs-design.md).
//!
//! The pipeline order is fixed and documented - `select -> sort -> unique
//! -> slice`, with `join` applied by the caller after the stream is final.
//! Predicate grammar is deliberately micro: `RELPATH OP LITERAL`, no
//! parentheses, no arithmetic, and mismatched comparison kinds are `false`
//! (a documented divergence from jq's total order that keeps scripts
//! unsurprising).

use crate::paths;
use clap::Args;
use pyrs_yaml_core::ast::{CustomNode, ScalarStyle};
use pyrs_yaml_core::parser::yaml::{Schema, YamlType};
use pyrs_yaml_core::{parser, serializer};
use std::borrow::Cow;
use std::cmp::Ordering;

/// Composable post-selection verbs shared by `get` and the `from-*`
/// converters.
#[derive(Args, Debug, Default)]
pub struct Verbs {
    /// Keep nodes matching PRED: `path ==|!=|>|>=|<|<= literal` (path
    /// relative to each node; `.` is the node itself; literal is YAML).
    #[arg(long)]
    pub select: Option<String>,
    /// Stable-sort the stream by a relative path (missing key sorts last).
    #[arg(long)]
    pub sort_by: Option<String>,
    /// Reverse the --sort-by comparison.
    #[arg(long, requires = "sort_by")]
    pub desc: bool,
    /// Deduplicate nodes by serialized form (after sorting, like jq).
    #[arg(long)]
    pub unique: bool,
    /// Keep only the first stream node.
    #[arg(long, conflicts_with = "last")]
    pub first: bool,
    /// Keep only the last stream node.
    #[arg(long)]
    pub last: bool,
    /// Drop the first N stream nodes.
    #[arg(long)]
    pub skip: Option<usize>,
    /// Keep at most N stream nodes.
    #[arg(long)]
    pub take: Option<usize>,
    /// Join an all-scalar stream with SEP into a single string.
    #[arg(long)]
    pub join: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Op {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

/// One parsed `--select` predicate.
struct Predicate {
    rel: paths::Selector,
    op: Op,
    /// Literal operand as a resolved AST node (parsed once).
    lit: CustomNode,
}

/// Split `RELPATH OP LITERAL`: read the relative path (bare dot-path or a
/// quoted bracket key), then the longest matching operator, then the rest
/// as a YAML literal.
fn parse_predicate(input: &str) -> Result<Predicate, String> {
    let bad = || format!("invalid predicate: {input}");
    let bytes = input.as_bytes();
    let lead = input.len() - input.trim_start().len();
    let mut i = lead;
    // Relative path: `.` root, or a run of path-ish chars; a leading
    // quoted key (`"a>b".x`) is accepted by parse_path, so include quoted
    // spans in the scan.
    let start = i;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
    }
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1; // closing quote
                }
                // a quoted key may start a segment list: `."a" [0].b`
                if i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b'[') {
                    continue;
                }
                break;
            }
            b'=' | b'!' | b'<' | b'>' => break,
            c if (c as char).is_whitespace() => {
                // whitespace may precede the operator
                break;
            }
            _ => i += 1,
        }
    }
    let rel_src = &input[start..i];
    let rel = if rel_src.trim().is_empty() || rel_src.trim() == "." {
        paths::parse_path(".")?
    } else {
        let trimmed = rel_src.trim_end();
        // parse_path accepts bare paths; normalize a missing leading dot
        // only when the source starts quoted.
        let normalized = if trimmed.starts_with(['.', '$']) {
            trimmed.to_string()
        } else {
            format!(".{trimmed}")
        };
        paths::parse_path(&normalized)?
    };
    let rest = input[i..].trim_start();
    let (op, rest) = [
        ("==", Op::Eq),
        ("!=", Op::Ne),
        (">=", Op::Ge),
        ("<=", Op::Le),
        (">", Op::Gt),
        ("<", Op::Lt),
    ]
    .iter()
    .find(|(sym, _)| rest.starts_with(sym))
    .map(|(sym, o)| (*o, &rest[sym.len()..]))
    .ok_or_else(bad)?;
    let lit_src = rest.trim();
    if lit_src.is_empty() {
        return Err(bad());
    }
    let lit = parser::parse(lit_src, Schema::Core).map_err(|_| bad())?;
    Ok(Predicate { rel, op, lit })
}

/// Textual identity used by `==`, `!=` and `--unique`.
fn node_text(node: &CustomNode) -> String {
    serializer::to_yaml(node).trim_end().to_string()
}

/// Typed view of a scalar for ordering; non-scalars and mixed kinds are
/// "incomparable" (documented `false`).
enum Typed<'a> {
    Num(f64),
    Str(Cow<'a, str>),
    Other,
}

fn typed(node: &CustomNode) -> Typed<'_> {
    if let CustomNode::Scalar {
        value,
        style: ScalarStyle::Plain,
        meta,
        ..
    } = node
    {
        // A standard tag states the type here as it does in the loader and the format
        // bridges: ordering or filtering `!!str 1.20` as the number 1.2 answers a different
        // question than the document asked. A tag the text cannot satisfy has no typed view,
        // which this function already reports as incomparable - the honest answer for a
        // comparison, rather than a guess at one side of it.
        let resolved = match meta.tag.as_ref().and_then(|t| {
            pyrs_yaml_core::parser::yaml::schema::standard_tag_kind(&t.handle, &t.suffix)
        }) {
            Some(kind) => match kind.resolve(value) {
                Some(typed) => typed,
                None => return Typed::Other,
            },
            None => Schema::Core.resolve(value),
        };
        return match resolved {
            YamlType::Int(i) => Typed::Num(i as f64),
            YamlType::Float(f) if f.is_finite() => Typed::Num(f),
            YamlType::Str(s) => Typed::Str(s),
            _ => Typed::Other,
        };
    }
    Typed::Other
}

fn compare(found: &CustomNode, op: Op, lit: &CustomNode) -> bool {
    match (typed(found), typed(lit)) {
        (Typed::Num(a), Typed::Num(b)) => match op {
            Op::Eq => a == b,
            Op::Ne => a != b,
            Op::Gt => a > b,
            Op::Ge => a >= b,
            Op::Lt => a < b,
            Op::Le => a <= b,
        },
        (Typed::Str(ref a), Typed::Str(ref b)) => match op {
            Op::Eq => a == b,
            Op::Ne => a != b,
            Op::Gt => a > b,
            Op::Ge => a >= b,
            Op::Lt => a < b,
            Op::Le => a <= b,
        },
        // Equality has a structural meaning for any pair; ordering does
        // not (mismatched kinds are false by contract).
        (Typed::Other, _) | (_, Typed::Other) if matches!(op, Op::Eq | Op::Ne) => {
            let eq = node_text(found) == node_text(lit);
            if op == Op::Eq { eq } else { !eq }
        }
        _ => false,
    }
}

impl Verbs {
    /// Run select -> sort -> unique -> slice over the matched stream.
    pub fn apply<'a>(
        &self,
        stream: Vec<&'a CustomNode>,
        path: &str,
    ) -> Result<Vec<&'a CustomNode>, String> {
        // Parse the predicate first: a syntax error must surface even for
        // empty or filtered-to-empty streams (fail fast for scripts).
        let predicate = self
            .select
            .as_ref()
            .map(|src| parse_predicate(src))
            .transpose()?;
        if stream.is_empty() {
            // No verbs involved yet: this is the plain selector miss.
            return Err(format!("path not found: {path}"));
        }
        let mut out = stream;
        if let Some(pred) = &predicate {
            let mut kept = Vec::new();
            for node in out {
                if let Some(found) = pred.rel.select(node).ok().flatten()
                    && compare(found, pred.op, &pred.lit)
                {
                    kept.push(node);
                }
            }
            out = kept;
        }
        if let Some(key_src) = &self.sort_by {
            let key = paths::parse_path(key_src)?;
            out.sort_by(|a, b| sort_cmp(&sort_key(a, &key), &sort_key(b, &key)));
            if self.desc {
                // stable sort + reverse keeps equal elements' relative
                // order mirrored, matching `sort_by(...) | reverse`
                out.reverse();
            }
        }
        if self.unique {
            let mut seen = Vec::<String>::with_capacity(out.len());
            out.retain(|node| {
                let text = node_text(node);
                if seen.contains(&text) {
                    false
                } else {
                    seen.push(text);
                    true
                }
            });
        }
        if let Some(n) = self.skip {
            out = out.into_iter().skip(n).collect();
        }
        if let Some(n) = self.take {
            out = out.into_iter().take(n).collect();
        }
        if out.is_empty() {
            return Err(format!("no matches after filters: {path}"));
        }
        if self.first {
            out.truncate(1);
        }
        if self.last {
            out = out.into_iter().rev().take(1).collect();
            out.reverse();
        }
        Ok(out)
    }

    /// Final `--join` stage: requires an all-scalar stream.
    pub fn render_join(&self, stream: &[&CustomNode]) -> Result<Option<String>, String> {
        let Some(sep) = &self.join else {
            return Ok(None);
        };
        let mut parts = Vec::with_capacity(stream.len());
        for node in stream {
            match node {
                CustomNode::Scalar { value, .. } => parts.push(value.to_string()),
                _ => return Err("--join needs an all-scalar stream".to_string()),
            }
        }
        Ok(Some(parts.join(sep)))
    }
}

/// Sort key: typed scalar first (numbers before strings never happens -
/// incomparable pairs keep stream order via Equal), missing keys last.
fn sort_key(node: &CustomNode, key: &paths::Selector) -> (u8, f64, String) {
    match key.select(node).ok().flatten() {
        Some(found) => match typed(found) {
            Typed::Num(n) => (0, n, String::new()),
            Typed::Str(s) => (1, 0.0, s.into_owned()),
            Typed::Other => (2, 0.0, node_text(found)),
        },
        None => (3, 0.0, String::new()),
    }
}

/// f64 is only PartialOrd, so the tuple ordering is spelled out: buckets
/// decide first, the bucket-internal measure second (NaN-safe Equal).
fn sort_cmp(a: &(u8, f64, String), b: &(u8, f64, String)) -> Ordering {
    a.0.cmp(&b.0).then_with(|| match (a.0, b.0) {
        (0, 0) => a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal),
        (1, 1) | (2, 2) => a.2.cmp(&b.2),
        _ => Ordering::Equal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes(src: &str) -> Vec<CustomNode> {
        let doc = parser::parse(src, Schema::Core).unwrap();
        let CustomNode::Mapping { pairs, .. } = &doc else {
            panic!("fixture root must be a mapping")
        };
        let CustomNode::Sequence { items, .. } = pairs.values().next().expect("seq value") else {
            panic!("fixture s must be a sequence")
        };
        items.clone()
    }

    fn refs(v: &[CustomNode]) -> Vec<&CustomNode> {
        v.iter().collect()
    }

    fn texts(stream: &[&CustomNode]) -> Vec<String> {
        stream.iter().map(|n| node_text(n)).collect()
    }

    const FIXTURE: &str = "s:\n  - {name: b, port: 900}\n  - {name: a, port: 1500}\n  - {name: c}\n  - {name: a, port: 1500}\n";

    #[test]
    fn predicate_ops_and_longest_match() {
        let p = parse_predicate("port >= 1000").unwrap();
        assert_eq!(p.op, Op::Ge);
        let doc = parser::parse("port: 1500", Schema::Core).unwrap();
        let CustomNode::Mapping { pairs, .. } = &doc else {
            unreachable!()
        };
        assert!(compare(pairs.values().next().unwrap(), p.op, &p.lit));
        assert!(parse_predicate("port >").is_err());
        assert!(parse_predicate("port => 1").is_err());
    }

    #[test]
    fn select_filters_by_typed_comparison() {
        let items = nodes(FIXTURE);
        let v = Verbs {
            select: Some("port > 1000".into()),
            ..Default::default()
        };
        let out = v.apply(refs(&items), "$").unwrap();
        assert_eq!(out.len(), 2, "both a/1500 items pass; missing port = false");
    }

    #[test]
    fn equality_works_across_kinds_ordering_does_not() {
        let items = nodes("s:\n  - {v: 1}\n  - {v: text}\n  - {v: true}\n");
        let v = Verbs {
            select: Some("v == 1".into()),
            ..Default::default()
        };
        assert_eq!(v.apply(refs(&items), "$").unwrap().len(), 1);
        let v = Verbs {
            select: Some("v > 1".into()),
            ..Default::default()
        };
        assert!(
            v.apply(refs(&items), "$").is_err(),
            "all kinds false -> empty"
        );
    }

    #[test]
    fn sort_by_places_missing_last_and_desc_reverses() {
        let items = nodes(FIXTURE);
        let v = Verbs {
            sort_by: Some(".port".into()),
            ..Default::default()
        };
        let out = texts(&v.apply(refs(&items), "$").unwrap());
        assert!(out[0].contains("900"), "{out:?}");
        assert!(out.last().unwrap().contains("name: c"), "{out:?}");
        let v = Verbs {
            sort_by: Some(".port".into()),
            desc: true,
            ..Default::default()
        };
        let out = texts(&v.apply(refs(&items), "$").unwrap());
        assert!(
            out[0].contains("name: c") || out[0].contains("1500"),
            "{out:?}"
        );
    }

    #[test]
    fn unique_dedups_after_sort() {
        let items = nodes(FIXTURE);
        let v = Verbs {
            sort_by: Some(".name".into()),
            unique: true,
            ..Default::default()
        };
        assert_eq!(v.apply(refs(&items), "$").unwrap().len(), 3);
    }

    #[test]
    fn slice_first_last() {
        let items = nodes(FIXTURE);
        let v = Verbs {
            skip: Some(1),
            take: Some(2),
            ..Default::default()
        };
        assert_eq!(v.apply(refs(&items), "$").unwrap().len(), 2);
        let v = Verbs {
            first: true,
            ..Default::default()
        };
        assert!(node_text(v.apply(refs(&items), "$").unwrap()[0]).contains("name: b"));
        let v = Verbs {
            last: true,
            ..Default::default()
        };
        assert!(node_text(v.apply(refs(&items), "$").unwrap()[0]).contains("a, port: 1500"));
    }

    #[test]
    fn join_requires_scalars() {
        let items = nodes("s:\n  - one\n  - two\n");
        let v = Verbs {
            join: Some(",".into()),
            ..Default::default()
        };
        assert_eq!(
            v.render_join(&refs(&items)).unwrap(),
            Some("one,two".into())
        );
        let items = nodes(FIXTURE);
        assert!(v.render_join(&refs(&items)).is_err());
    }

    #[test]
    fn root_dot_predicate_on_scalar_stream() {
        let items = nodes("s:\n  - 1\n  - 5\n  - 9\n");
        let v = Verbs {
            select: Some(". > 4".into()),
            sort_by: Some(".".into()),
            ..Default::default()
        };
        let out = v.apply(refs(&items), "$").unwrap();
        assert_eq!(texts(&out), vec!["5", "9"]);
    }
}
