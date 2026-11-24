//! Token stream -> an `XmlNode` tree.
//!
//! This layer owns document well-formedness (matching end tags, one root,
//! no document-level comments/PIs), the depth and node caps, and the namespace
//! prefix scope stack. It does not know anything about YAML: mapping is the
//! next layer's job, so the same tree could in principle be mapped twice with
//! different options.

use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::XmlError;
use crate::scanner::{Token, TokenKind, is_xml_whitespace};

/// The infoset node the scanner's tokens build. Character data and CDATA both
/// become `Text` — the delimiters are one of the six normalised-away lexical
/// facts, not structure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XmlNode {
    /// An element with its attributes and children in document order.
    Element {
        /// QName, prefixes kept verbatim.
        name: String,
        /// Attributes in document order, values already decoded.
        attrs: Vec<(String, String)>,
        /// Content parts in document order.
        children: Vec<XmlNode>,
        /// 1-based line of the start tag.
        line: usize,
        /// 1-based column of the start tag.
        column: usize,
    },
    /// Character data (from text or CDATA).
    Text(String),
    /// A comment body.
    Comment(String),
    /// A processing instruction.
    Pi {
        /// PI target.
        target: String,
        /// PI data, empty string when the PI carried none.
        data: String,
    },
}

struct Frame {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<XmlNode>,
    line: usize,
    column: usize,
}

/// Build the one root element from a token stream.
///
/// `max_depth` and `max_nodes` are enforced on every push and every node
/// creation respectively; a zero cap was already rejected by
/// `XmlOptions::validate()` before the scanner ran.
pub fn build(tokens: Vec<Token>, max_depth: usize, max_nodes: usize) -> Result<XmlNode, XmlError> {
    let mut stack: Vec<Frame> = Vec::new();
    let mut scopes: Vec<BTreeSet<String>> = Vec::new();
    let mut root: Option<XmlNode> = None;
    let mut root_done = false;
    let mut nodes = 0usize;

    for token in tokens {
        let (line, column) = (token.line, token.column);
        match token.kind {
            TokenKind::Start {
                name,
                attrs,
                self_closing,
            } => {
                if root_done {
                    return Err(XmlError::MultipleRoots { line, column });
                }
                if stack.len() + 1 > max_depth {
                    return Err(XmlError::DepthLimitExceeded { limit: max_depth });
                }
                bump(&mut nodes, max_nodes)?;

                let mut scope = BTreeSet::new();
                for (key, _) in &attrs {
                    if let Some(prefix) = key.strip_prefix("xmlns:") {
                        scope.insert(prefix.to_string());
                    }
                }
                check_prefix(&name, &scope, &scopes, line, column)?;
                for (key, _) in &attrs {
                    // `xmlns` and `xmlns:<p>` are declarations, not uses of a
                    // prefix that has to be bound already.
                    if key == "xmlns" || key.starts_with("xmlns:") {
                        continue;
                    }
                    check_prefix(key, &scope, &scopes, line, column)?;
                }

                if self_closing {
                    let node = XmlNode::Element {
                        name,
                        attrs,
                        children: Vec::new(),
                        line,
                        column,
                    };
                    attach(node, &mut stack, &mut root, &mut root_done);
                    continue;
                }
                scopes.push(scope);
                stack.push(Frame {
                    name,
                    attrs,
                    children: Vec::new(),
                    line,
                    column,
                });
            }
            TokenKind::End { name } => {
                let frame = stack.pop().ok_or(XmlError::MalformedMarkup {
                    line,
                    column,
                    detail: alloc::format!("end tag </{name}> without an open element"),
                })?;
                if frame.name != name {
                    return Err(XmlError::MismatchedEndTag {
                        expected: frame.name,
                        found: name,
                        line,
                        column,
                    });
                }
                scopes.pop();
                let node = XmlNode::Element {
                    name: frame.name,
                    attrs: frame.attrs,
                    children: frame.children,
                    line: frame.line,
                    column: frame.column,
                };
                attach(node, &mut stack, &mut root, &mut root_done);
            }
            TokenKind::Text(text) | TokenKind::CData(text) => match stack.last_mut() {
                Some(frame) => {
                    bump(&mut nodes, max_nodes)?;
                    frame.children.push(XmlNode::Text(text));
                }
                None if text.chars().all(is_xml_whitespace) => {
                    // Inter-element whitespace outside the root is markup
                    // layout, not data; dropping it allocates no node.
                }
                None => {
                    return Err(XmlError::MalformedMarkup {
                        line,
                        column,
                        detail: "character data outside the root element".to_string(),
                    });
                }
            },
            TokenKind::Comment(body) => {
                if stack.is_empty() {
                    return Err(XmlError::UnsupportedDocumentNode {
                        kind: "comment".to_string(),
                        line,
                        column,
                    });
                }
                bump(&mut nodes, max_nodes)?;
                stack
                    .last_mut()
                    .expect("checked non-empty above")
                    .children
                    .push(XmlNode::Comment(body));
            }
            TokenKind::ProcessingInstruction { target, data } => {
                if stack.is_empty() {
                    return Err(XmlError::UnsupportedDocumentNode {
                        kind: "processing-instruction".to_string(),
                        line,
                        column,
                    });
                }
                bump(&mut nodes, max_nodes)?;
                stack
                    .last_mut()
                    .expect("checked non-empty above")
                    .children
                    .push(XmlNode::Pi { target, data });
            }
        }
    }

    if let Some(frame) = stack.last() {
        return Err(XmlError::MalformedMarkup {
            line: frame.line,
            column: frame.column,
            detail: alloc::format!("unclosed element <{}>", frame.name),
        });
    }
    root.ok_or(XmlError::MalformedMarkup {
        line: 1,
        column: 1,
        detail: "no root element".to_string(),
    })
}

fn bump(nodes: &mut usize, limit: usize) -> Result<(), XmlError> {
    *nodes += 1;
    if *nodes > limit {
        return Err(XmlError::NodeLimitExceeded { limit });
    }
    Ok(())
}

fn attach(node: XmlNode, stack: &mut [Frame], root: &mut Option<XmlNode>, root_done: &mut bool) {
    match stack.last_mut() {
        Some(frame) => frame.children.push(node),
        None => {
            *root = Some(node);
            *root_done = true;
        }
    }
}

/// A prefix must be bound in the element's own declarations or an ancestor's.
/// `xml` is implicitly bound by the XML specification; `xmlns` is the
/// declaration namespace itself.
fn check_prefix(
    qname: &str,
    own: &BTreeSet<String>,
    ancestors: &[BTreeSet<String>],
    line: usize,
    column: usize,
) -> Result<(), XmlError> {
    if let Some((prefix, _)) = qname.split_once(':') {
        if prefix == "xml" || prefix == "xmlns" || own.contains(prefix) {
            return Ok(());
        }
        if ancestors.iter().any(|scope| scope.contains(prefix)) {
            return Ok(());
        }
        return Err(XmlError::UndeclaredPrefix {
            prefix: prefix.to_string(),
            line,
            column,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::build;
    use crate::scanner::scan;

    fn tree(text: &str) -> super::XmlNode {
        build(scan(text).unwrap(), 256, 1_000_000).unwrap()
    }

    #[test]
    fn builds_a_single_root_with_children() {
        let node = tree("<root><a>1</a><a>2</a></root>");
        match node {
            super::XmlNode::Element { name, children, .. } => {
                assert_eq!(name, "root");
                assert_eq!(children.len(), 2);
            }
            other => panic!("expected element, got {other:?}"),
        }
    }

    #[test]
    fn mismatched_and_unclosed_tags_are_refused() {
        let e = build(scan("<a></b>").unwrap(), 256, 100).unwrap_err();
        assert!(
            matches!(e, crate::error::XmlError::MismatchedEndTag { ref expected, ref found, .. } if expected == "a" && found == "b")
        );
        let e = build(scan("<a><b></a>").unwrap(), 256, 100).unwrap_err();
        assert_eq!(e.code(), "E-XML-005");
        let e = build(scan("<a>").unwrap(), 256, 100).unwrap_err();
        assert_eq!(e.code(), "E-XML-004");
    }

    #[test]
    fn multiple_roots_and_document_nodes_are_refused() {
        assert_eq!(
            build(scan("<a/><b/>").unwrap(), 256, 100)
                .unwrap_err()
                .code(),
            "E-XML-006"
        );
        let comment = build(scan("<!--x--><a/>").unwrap(), 256, 100).unwrap_err();
        assert!(
            matches!(comment, crate::error::XmlError::UnsupportedDocumentNode { ref kind, .. } if kind == "comment")
        );
        assert_eq!(
            build(scan("<a/><b/>").unwrap(), 256, 100)
                .unwrap_err()
                .code(),
            "E-XML-006"
        );
    }

    #[test]
    fn whitespace_outside_the_root_is_ignored() {
        assert!(build(scan("  \n<a/>  ").unwrap(), 256, 100).is_ok());
    }

    #[test]
    fn depth_and_node_caps_are_enforced() {
        let deep = "<a><a><a/></a></a>";
        assert_eq!(
            build(scan(deep).unwrap(), 2, 100).unwrap_err().code(),
            "E-XML-011"
        );
        assert!(build(scan(deep).unwrap(), 3, 100).is_ok());
        let wide = "<a><x/><x/><x/></a>";
        assert_eq!(
            build(scan(wide).unwrap(), 10, 3).unwrap_err().code(),
            "E-XML-012"
        );
    }

    #[test]
    fn prefix_scope_is_a_stack() {
        assert!(
            build(
                scan("<p:outer xmlns:p='u'><p:inner/></p:outer>").unwrap(),
                256,
                100
            )
            .is_ok()
        );
        assert_eq!(
            build(scan("<p:a/>").unwrap(), 256, 100).unwrap_err().code(),
            "E-XML-013"
        );
        assert_eq!(
            build(scan("<p:a xmlns:p='u'><q:b/></p:a>").unwrap(), 256, 100)
                .unwrap_err()
                .code(),
            "E-XML-013"
        );
    }
}
