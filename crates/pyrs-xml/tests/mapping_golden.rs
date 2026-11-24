//! Golden corpus: every fixture maps to the committed YAML text exactly.
//!
//! The emission helper below is deliberately local to this test and tiny: it
//! has no dependency on the YAML writer (this crate must not depend on
//! `pyrs-yaml-core`) and covers only the node shapes `pyrs-xml` produces. Set
//! `PYRS_XML_GENERATE_GOLDEN=1` once to regenerate the committed files; the
//! normal test compares against them and never writes.

use std::fs;
use std::path::{Path, PathBuf};

use pyrs_ast::ast::{CustomNode, ScalarStyle};
use pyrs_xml::from_xml;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn every_golden_fixture_maps_exactly() {
    let dir = fixtures_dir();
    let mut names = fs::read_dir(&dir)
        .expect("fixtures dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .into_string()
                .expect("utf8 name")
        })
        .filter(|name| name.ends_with(".xml"))
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(names.len(), 15, "the golden corpus has 15 fixtures");

    for name in names {
        let source = fs::read(dir.join(&name)).expect("fixture bytes");
        let xml = std::str::from_utf8(&source).expect("UTF-8 input");
        let node = from_xml(xml).unwrap_or_else(|e| panic!("{name}: {e}"));
        let actual = emit(&node);

        let golden = dir.join(name.replace(".xml", ".yaml"));
        if std::env::var_os("PYRS_XML_GENERATE_GOLDEN").is_some() {
            fs::write(&golden, actual.as_bytes()).expect("write golden");
            continue;
        }
        let expected = fs::read_to_string(&golden)
            .unwrap_or_else(|_| panic!("missing golden {}", golden.display()));
        assert_eq!(actual, expected, "golden mismatch for {name}");
    }
}

/// Escape a scalar the way a double-quoted YAML scalar needs.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn inline(node: &CustomNode) -> String {
    match node {
        CustomNode::Scalar {
            value,
            style: ScalarStyle::DoubleQuoted,
            ..
        } => quoted(value),
        // The shipped serializer quotes any plain scalar containing a colon,
        // so the local emitter has to do the same to produce the same bytes.
        CustomNode::Scalar { value, .. } if value.contains(':') => quoted(value),
        CustomNode::Scalar { value, .. } => value.to_string(),
        CustomNode::Null { .. } => "null".to_string(),
        _ => panic!("not an inline node"),
    }
}

fn is_inline(node: &CustomNode) -> bool {
    matches!(node, CustomNode::Scalar { .. } | CustomNode::Null { .. })
}

fn spaces(n: usize) -> String {
    " ".repeat(n)
}

/// Emit one `key: value` pair at `indent`; a nested value starts two spaces in.
fn emit_pair(key: &CustomNode, value: &CustomNode, indent: usize, out: &mut String) {
    out.push_str(&inline(key));
    if is_inline(value) {
        out.push_str(": ");
        out.push_str(&inline(value));
        out.push('\n');
    } else {
        out.push_str(":\n");
        emit_at(value, indent + 2, out);
    }
}

fn emit_at(node: &CustomNode, indent: usize, out: &mut String) {
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs {
                out.push_str(&spaces(indent));
                emit_pair(key, value, indent, out);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                out.push_str(&spaces(indent));
                out.push('-');
                match item {
                    CustomNode::Mapping { pairs, .. }
                        if pairs.iter().next().is_some_and(|(_, v)| !is_inline(v)) =>
                    {
                        // A mapping item whose first value is itself a
                        // collection goes on the next line, indent + 2. The
                        // shipped serializer leaves a space after the dash.
                        out.push_str(" \n");
                        for (key, value) in pairs {
                            out.push_str(&spaces(indent + 2));
                            emit_pair(key, value, indent + 2, out);
                        }
                    }
                    CustomNode::Mapping { pairs, .. } => {
                        out.push(' ');
                        let mut iter = pairs.iter();
                        let (first_key, first_value) = iter.next().expect("mapping has a key");
                        emit_pair(first_key, first_value, indent + 2, out);
                        for (key, value) in iter {
                            out.push_str(&spaces(indent + 2));
                            emit_pair(key, value, indent + 2, out);
                        }
                    }
                    CustomNode::Sequence { .. } => {
                        out.push('\n');
                        emit_at(item, indent + 2, out);
                    }
                    _ => {
                        out.push(' ');
                        out.push_str(&inline(item));
                        out.push('\n');
                    }
                }
            }
        }
        _ => {
            out.push_str(&spaces(indent));
            out.push_str(&inline(node));
            out.push('\n');
        }
    }
}

fn emit(node: &CustomNode) -> String {
    let mut out = String::new();
    emit_at(node, 0, &mut out);
    out
}
