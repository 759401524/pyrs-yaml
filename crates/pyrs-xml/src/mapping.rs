//! `XmlNode` -> shared `CustomNode` (the YAML side of the bridge).
//!
//! This is where the `+` convention lives. Structure that is not an element
//! name is prefixed with `+`, a character outside both the YAML plain-scalar
//! indicators and the XML `Name` production, so a reserved key can never
//! collide with a real element name. A text-only element with no attributes
//! short-circuits to a plain scalar instead of a `{+content: ...}` wrapper.

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use pyrs_ast::ast::{Chomping, CustomNode, NodeMap, NodeMeta, ScalarStyle};

use crate::error::XmlError;
use crate::model::{XmlNode, build};
use crate::scanner::{is_xml_whitespace, scan};

/// How the XML text is read. The two content switches default to `true`; the
/// two caps default to 256 and 1,000,000 and are validated before input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XmlOptions {
    /// Retain comments inside the root as `+c_<n>` parts (default `true`).
    pub comments: bool,
    /// Retain processing instructions inside the root as `+p_<n>` parts
    /// (default `true`).
    pub processing_instructions: bool,
    /// Maximum element nesting depth (default 256, must be `>= 1`).
    pub max_depth: usize,
    /// Maximum number of materialised nodes (default 1,000,000, `>= 1`).
    pub max_nodes: usize,
}

impl Default for XmlOptions {
    fn default() -> Self {
        Self {
            comments: true,
            processing_instructions: true,
            max_depth: 256,
            max_nodes: 1_000_000,
        }
    }
}

impl XmlOptions {
    /// Resolve the explicit options before a single byte of input is read.
    /// A zero cap is a configuration error, never a silent "unlimited".
    pub fn validate(&self) -> Result<(), XmlError> {
        if self.max_depth == 0 {
            return Err(XmlError::InvalidOptions {
                field: "max_depth",
                detail: "must be at least 1; 0 is not unlimited".to_string(),
            });
        }
        if self.max_nodes == 0 {
            return Err(XmlError::InvalidOptions {
                field: "max_nodes",
                detail: "must be at least 1; 0 is not unlimited".to_string(),
            });
        }
        Ok(())
    }
}

/// Read XML text into the shared AST with default options.
pub fn from_xml(text: &str) -> Result<CustomNode, XmlError> {
    from_xml_with_options(text, &XmlOptions::default())
}

/// Read XML text into the shared AST with explicit options.
///
/// Options are validated, then the text is scanned and refused at the `<!`
/// boundary for DTD/entity declarations, then built and mapped. No partial
/// tree is ever returned: the result is a node or a refusal.
pub fn from_xml_with_options(text: &str, options: &XmlOptions) -> Result<CustomNode, XmlError> {
    options.validate()?;
    let tokens = scan(text)?;
    let root = build(tokens, options.max_depth, options.max_nodes)?;
    let mut occurrence = 0usize;
    Ok(map_document(&root, options, &mut occurrence))
}

/// The document is a single mapping with the root element's QName as its one
/// key. There is no `+doc` wrapper — XML has exactly one root element, and a
/// wrapper would make every output less readable for no added information.
fn map_document(root: &XmlNode, options: &XmlOptions, occurrence: &mut usize) -> CustomNode {
    let (name, _, _) = element_parts(root);
    let mut pairs = NodeMap::default();
    pairs.insert(name_key(name), element_value(root, options, occurrence));
    CustomNode::Mapping {
        pairs,
        flow_style: false,
        meta: NodeMeta::default(),
    }
}

fn element_parts(node: &XmlNode) -> (&str, &[(String, String)], &[XmlNode]) {
    match node {
        XmlNode::Element {
            name,
            attrs,
            children,
            ..
        } => (name, attrs, children),
        // `build` guarantees the root and every child element node are
        // `Element`; this arm is unreachable for trees it produced.
        _ => ("", &[], &[]),
    }
}

/// A content part after §4.1 preprocessing (adjacent text merged, whitespace
/// dropped when there is an element child, comments/PIs optionally dropped).
enum Part<'a> {
    Text(String),
    Element(&'a XmlNode),
    /// The `usize` is the global occurrence index among preserved comments.
    Comment(String, usize),
    /// The `usize` is the global occurrence index among preserved PIs.
    Pi(String, String, usize),
}

fn build_parts<'a>(
    children: &'a [XmlNode],
    options: &XmlOptions,
    occurrence: &mut usize,
) -> Vec<Part<'a>> {
    let has_element_child = children
        .iter()
        .any(|child| matches!(child, XmlNode::Element { .. }));
    let mut parts: Vec<Part<'a>> = Vec::with_capacity(children.len());

    for child in children {
        match child {
            XmlNode::Text(text) => match parts.last_mut() {
                Some(Part::Text(last)) => last.push_str(text),
                _ => parts.push(Part::Text(text.clone())),
            },
            XmlNode::Element { .. } => parts.push(Part::Element(child)),
            XmlNode::Comment(body) => {
                if options.comments {
                    let index = *occurrence;
                    *occurrence += 1;
                    parts.push(Part::Comment(body.clone(), index));
                }
            }
            XmlNode::Pi { target, data } => {
                if options.processing_instructions {
                    let index = *occurrence;
                    *occurrence += 1;
                    parts.push(Part::Pi(target.clone(), data.clone(), index));
                }
            }
        }
    }

    if has_element_child {
        parts.retain(|part| match part {
            Part::Text(text) => !text.chars().all(is_xml_whitespace),
            _ => true,
        });
    }
    parts
}

/// The YAML value of an element, per the five shape rules in the spec.
fn element_value(node: &XmlNode, options: &XmlOptions, occurrence: &mut usize) -> CustomNode {
    let (_, attrs, children) = element_parts(node);
    let parts = build_parts(children, options, occurrence);

    let mut pairs = NodeMap::default();
    for (name, value) in attrs {
        pairs.insert(attr_key(name), xml_scalar(value));
    }

    if parts.is_empty() {
        return if pairs.is_empty() {
            null_node()
        } else {
            mapping(pairs)
        };
    }

    let only_elements = parts.iter().all(|part| matches!(part, Part::Element(_)));

    if parts.len() == 1 && matches!(parts[0], Part::Text(_)) {
        let Part::Text(text) = &parts[0] else {
            unreachable!("guarded above")
        };
        let value = xml_scalar(text);
        if pairs.is_empty() {
            return value;
        }
        pairs.insert(reserved_key("+content"), value);
        return mapping(pairs);
    }

    if only_elements {
        for part in parts {
            if let Part::Element(child) = part {
                insert_child(&mut pairs, child, options, occurrence);
            }
        }
        return mapping(pairs);
    }

    let items = parts
        .into_iter()
        .map(|part| part_node(part, options, occurrence))
        .collect::<Vec<_>>();
    pairs.insert(
        reserved_key("+content"),
        CustomNode::Sequence {
            items,
            flow_style: false,
            meta: NodeMeta::default(),
        },
    );
    mapping(pairs)
}

/// A child element under its QName; repeated siblings gather into a sequence
/// under that one key, in document order.
fn insert_child(
    pairs: &mut NodeMap<CustomNode, CustomNode>,
    child: &XmlNode,
    options: &XmlOptions,
    occurrence: &mut usize,
) {
    let (name, _, _) = element_parts(child);
    let key = name_key(name);
    let value = element_value(child, options, occurrence);
    match pairs.get_mut(&key) {
        Some(CustomNode::Sequence { items, .. }) => items.push(value),
        Some(existing) => {
            let first = core::mem::replace(existing, null_node());
            *existing = CustomNode::Sequence {
                items: vec![first, value],
                flow_style: false,
                meta: NodeMeta::default(),
            };
        }
        None => {
            pairs.insert(key, value);
        }
    }
}

/// One part of a `+content` sequence.
fn part_node(part: Part<'_>, options: &XmlOptions, occurrence: &mut usize) -> CustomNode {
    match part {
        Part::Text(text) => xml_scalar(&text),
        Part::Element(child) => {
            let (name, _, _) = element_parts(child);
            let value = element_value(child, options, occurrence);
            let mut pair = NodeMap::default();
            pair.insert(name_key(name), value);
            mapping(pair)
        }
        Part::Comment(body, index) => {
            let mut pair = NodeMap::default();
            pair.insert(
                reserved_key(&alloc::format!("+c_{index}")),
                xml_scalar(&body),
            );
            mapping(pair)
        }
        Part::Pi(target, data, index) => {
            let mut pi = NodeMap::default();
            pi.insert(reserved_key("target"), xml_scalar(&target));
            pi.insert(reserved_key("data"), xml_scalar(&data));
            let mut pair = NodeMap::default();
            pair.insert(reserved_key(&alloc::format!("+p_{index}")), mapping(pi));
            mapping(pair)
        }
    }
}

fn mapping(pairs: NodeMap<CustomNode, CustomNode>) -> CustomNode {
    CustomNode::Mapping {
        pairs,
        flow_style: false,
        meta: NodeMeta::default(),
    }
}

fn null_node() -> CustomNode {
    CustomNode::Null {
        meta: NodeMeta::default(),
    }
}

/// A scalar node with an explicit style.
fn scalar(value: &str, quoted: bool) -> CustomNode {
    CustomNode::Scalar {
        value: Arc::from(value),
        style: if quoted {
            ScalarStyle::DoubleQuoted
        } else {
            ScalarStyle::Plain
        },
        chomping: Chomping::Clip,
        block_indent: None,
        meta: NodeMeta::default(),
    }
}

/// The first character of a YAML plain scalar may not be one of these. `+`
/// deliberately is not among them: that is why the reserved keys work.
fn is_yaml_indicator(c: char) -> bool {
    matches!(
        c,
        '-' | '?'
            | ':'
            | ','
            | '['
            | ']'
            | '{'
            | '}'
            | '#'
            | '&'
            | '*'
            | '!'
            | '|'
            | '>'
            | '\''
            | '"'
            | '%'
            | '@'
            | '`'
            | '\u{feff}'
    )
}

/// Whether a value must be quoted so YAML reads it back as the same string.
fn needs_quotes(s: &str) -> bool {
    s.is_empty()
        || pyrs_schema::schema::plain_text_is_typed(s)
        || s.chars().next().is_some_and(is_yaml_indicator)
        || s.chars().next().is_some_and(is_xml_whitespace)
        || s.chars().last().is_some_and(is_xml_whitespace)
        || s.contains(['\n', '\r', '\t'])
}

/// A character-data / attribute value scalar. Typed-looking text (`007`,
/// `true`, `null`, `+7`, `yes`) and indicator-leading text stay strings.
pub(crate) fn xml_scalar(text: &str) -> CustomNode {
    scalar(text, needs_quotes(text))
}

/// Reserved keys are plain-safe by construction: `+` is not a YAML indicator,
/// and none of the reserved spellings is a core/1.1 typed lexeme.
fn reserved_key(name: &str) -> CustomNode {
    scalar(name, false)
}

fn attr_key(name: &str) -> CustomNode {
    reserved_key(&alloc::format!("+@{name}"))
}

/// An element name is a real name and may need quoting: a colon inside a
/// QName, a leading indicator, or a typed-looking spelling would all change
/// meaning if emitted plain.
fn name_key(name: &str) -> CustomNode {
    let quoted = name.is_empty()
        || name.contains(':')
        || is_yaml_indicator(name.chars().next().expect("names are non-empty"))
        || pyrs_schema::schema::plain_text_is_typed(name);
    scalar(name, quoted)
}

#[cfg(test)]
mod tests {
    use super::{XmlOptions, from_xml, from_xml_with_options};
    use crate::error::XmlError;
    use alloc::vec::Vec;
    use pyrs_ast::ast::{CustomNode, ScalarStyle};

    fn root(node: &CustomNode) -> &CustomNode {
        match node {
            CustomNode::Mapping { pairs, .. } => pairs.values().next().expect("one root"),
            _ => panic!("document is a mapping"),
        }
    }

    fn scalar_text(node: &CustomNode) -> &str {
        match node {
            CustomNode::Scalar { value, .. } => value,
            _ => panic!("expected scalar, got {node:?}"),
        }
    }

    #[test]
    fn a_text_only_element_is_a_plain_scalar() {
        let doc = from_xml("<name>Ada</name>").unwrap();
        assert!(matches!(
            root(&doc),
            CustomNode::Scalar {
                style: ScalarStyle::Plain,
                ..
            }
        ));
        assert_eq!(scalar_text(root(&doc)), "Ada");
    }

    #[test]
    fn typed_looking_text_stays_a_string() {
        for text in ["007", "true", "null", "+7", "yes", "~", "1e3"] {
            let doc = from_xml(&alloc::format!("<n>{text}</n>")).unwrap();
            assert!(
                matches!(
                    root(&doc),
                    CustomNode::Scalar {
                        style: ScalarStyle::DoubleQuoted,
                        ..
                    }
                ),
                "{text} was emitted plain"
            );
        }
        assert_eq!(scalar_text(root(&from_xml("<n>007</n>").unwrap())), "007");
    }

    #[test]
    fn empty_elements_are_null_and_attrs_are_reserved_keys() {
        let doc = from_xml("<a id='7'/>").unwrap();
        let CustomNode::Mapping { pairs, .. } = root(&doc) else {
            panic!("expected mapping")
        };
        assert_eq!(pairs.len(), 1);
        let key = pairs.keys().next().unwrap();
        assert_eq!(scalar_text(key), "+@id");
        assert_eq!(scalar_text(pairs.values().next().unwrap()), "7");
        assert!(
            matches!(from_xml("<a/>").unwrap(), CustomNode::Mapping { ref pairs, .. } if matches!(pairs.values().next(), Some(CustomNode::Null { .. })))
        );
    }

    #[test]
    fn attrs_plus_text_uses_content_reserved_key() {
        let doc = from_xml("<a id='7'>hi</a>").unwrap();
        let CustomNode::Mapping { pairs, .. } = root(&doc) else {
            panic!("expected mapping")
        };
        let content = pairs
            .iter()
            .find(|(k, _)| scalar_text(k) == "+content")
            .map(|(_, v)| v)
            .expect("+content present");
        assert_eq!(scalar_text(content), "hi");
    }

    #[test]
    fn repeated_siblings_become_one_sequence_key() {
        let doc = from_xml("<r><item>a</item><item>b</item></r>").unwrap();
        let CustomNode::Mapping { pairs, .. } = root(&doc) else {
            panic!("expected mapping")
        };
        assert_eq!(pairs.len(), 1);
        assert_eq!(scalar_text(pairs.keys().next().unwrap()), "item");
        assert!(
            matches!(pairs.values().next(), Some(CustomNode::Sequence { items, .. }) if items.len() == 2)
        );
    }

    #[test]
    fn pretty_print_whitespace_does_not_force_mixed_content() {
        let doc = from_xml("<r>\n  <a>1</a>\n  <b>2</b>\n</r>").unwrap();
        let CustomNode::Mapping { pairs, .. } = root(&doc) else {
            panic!("expected mapping")
        };
        assert!(!pairs.keys().any(|k| scalar_text(k) == "+content"));
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn significant_whitespace_without_element_children_is_kept() {
        let doc = from_xml("<a> </a>").unwrap();
        assert_eq!(scalar_text(root(&doc)), " ");
        assert!(matches!(
            root(&doc),
            CustomNode::Scalar {
                style: ScalarStyle::DoubleQuoted,
                ..
            }
        ));
    }

    #[test]
    fn comments_and_pis_share_one_occurrence_counter() {
        let doc = from_xml("<r><!--c--><?p d?></r>").unwrap();
        let CustomNode::Mapping { pairs, .. } = root(&doc) else {
            panic!("expected mapping")
        };
        let content = pairs
            .iter()
            .find(|(k, _)| scalar_text(k) == "+content")
            .map(|(_, v)| v)
            .unwrap();
        let CustomNode::Sequence { items, .. } = content else {
            panic!("expected sequence")
        };
        let keys = items
            .iter()
            .map(|item| match item {
                CustomNode::Mapping { pairs, .. } => scalar_text(pairs.keys().next().unwrap()),
                _ => "?",
            })
            .collect::<Vec<_>>();
        assert_eq!(keys, ["+c_0", "+p_1"]);
    }

    #[test]
    fn dropping_comments_renumbers_the_pi() {
        let options = XmlOptions {
            comments: false,
            ..XmlOptions::default()
        };
        let doc = from_xml_with_options("<r><!--c--><?p d?></r>", &options).unwrap();
        let CustomNode::Mapping { pairs, .. } = root(&doc) else {
            panic!("expected mapping")
        };
        let content = pairs
            .iter()
            .find(|(k, _)| scalar_text(k) == "+content")
            .map(|(_, v)| v)
            .unwrap();
        let CustomNode::Sequence { items, .. } = content else {
            panic!("expected sequence")
        };
        let key = match &items[0] {
            CustomNode::Mapping { pairs, .. } => scalar_text(pairs.keys().next().unwrap()),
            _ => "?",
        };
        assert_eq!(key, "+p_0");
    }

    #[test]
    fn the_golden_example_has_seven_content_parts() {
        let doc = from_xml(
            "<root id=\"7\"><name>Ada</name><item>a</item><item>b</item><!--note--><?pi go?><p>Hello <b>x</b>!</p><empty/></root>",
        )
        .unwrap();
        let CustomNode::Mapping { pairs, .. } = root(&doc) else {
            panic!("expected mapping")
        };
        let content = pairs
            .iter()
            .find(|(k, _)| scalar_text(k) == "+content")
            .map(|(_, v)| v)
            .unwrap();
        assert!(matches!(content, CustomNode::Sequence { items, .. } if items.len() == 7));
    }

    #[test]
    fn options_are_validated_before_input() {
        let bad = XmlOptions {
            max_depth: 0,
            ..XmlOptions::default()
        };
        assert!(matches!(
            from_xml_with_options("<a/>", &bad),
            Err(XmlError::InvalidOptions {
                field: "max_depth",
                ..
            })
        ));
        let bad = XmlOptions {
            max_nodes: 0,
            ..XmlOptions::default()
        };
        assert_eq!(
            from_xml_with_options("<a/>", &bad).unwrap_err().code(),
            "E-XML-014"
        );
    }

    #[test]
    fn every_input_family_maps_to_its_code() {
        let cases: [(&str, &str); 13] = [
            ("<!DOCTYPE r [<!ENTITY x 'y'>]><r/>", "E-XML-001"),
            ("<!ENTITY x 'y'>", "E-XML-002"),
            ("<r>&x;</r>", "E-XML-003"),
            ("<r", "E-XML-004"),
            ("<r></s>", "E-XML-005"),
            ("<a/><b/>", "E-XML-006"),
            ("<!--x--><a/>", "E-XML-007"),
            ("<?xml version='1.1'?><a/>", "E-XML-008"),
            ("<a x='1' x='2'/>", "E-XML-009"),
            ("<a>&#0;</a>", "E-XML-010"),
            ("<p:a/>", "E-XML-013"),
            ("<a><b/></a>", "E-XML-011"),
            ("<a><x/><x/><x/></a>", "E-XML-012"),
        ];
        for (input, code) in cases {
            let actual = match code {
                "E-XML-011" => from_xml_with_options(
                    input,
                    &XmlOptions {
                        max_depth: 1,
                        ..XmlOptions::default()
                    },
                )
                .unwrap_err()
                .code(),
                "E-XML-012" => from_xml_with_options(
                    input,
                    &XmlOptions {
                        max_nodes: 3,
                        ..XmlOptions::default()
                    },
                )
                .unwrap_err()
                .code(),
                _ => from_xml(input).unwrap_err().code(),
            };
            assert_eq!(actual, code, "input {input:?}");
        }
    }
}
