//! Metadata preservation during edits.
//!
//! Pure Rust implementation — no PyO3 dependencies.

use crate::ast::{CustomNode, NodeMeta, ScalarStyle};

/// ```
/// use pyrs_yaml_core::ast::CustomNode;
/// use pyrs_yaml_core::editing::with_metadata_from;
/// let target = CustomNode::plain_scalar("new_val");
/// let src = CustomNode::quoted_scalar("old_val");
/// let result = with_metadata_from(&target, &src);
/// ```
pub fn with_metadata_from(target: &CustomNode, src: &CustomNode) -> CustomNode {
    match (target, src) {
        (
            CustomNode::Scalar {
                value,
                style,
                block_indent,
                meta: new_meta,
                ..
            },
            CustomNode::Scalar {
                style: src_style,
                meta: src_meta,
                chomping,
                block_indent: src_block_indent,
                ..
            },
        ) => {
            let new_style = if *style == ScalarStyle::Plain
                && *src_style != ScalarStyle::Plain
                && !needs_quoting(value)
            {
                *src_style
            } else {
                *style
            };
            CustomNode::Scalar {
                value: value.clone(),
                style: new_style,
                meta: merged_meta(new_meta, src_meta),
                chomping: *chomping,
                // An explicit block indent only means something next to a
                // `|`/`>` header. Once the edit re-styles the node to a plain
                // or quoted scalar, carrying the old number would be dead
                // state that later re-emits a stray `|2`.
                block_indent: match new_style {
                    ScalarStyle::Literal | ScalarStyle::Folded => {
                        block_indent.or(*src_block_indent)
                    }
                    _ => None,
                },
            }
        }
        (
            CustomNode::Mapping {
                pairs,
                flow_style,
                meta: new_meta,
                ..
            },
            CustomNode::Mapping {
                meta: src_meta,
                flow_style: src_flow_style,
                ..
            },
        ) => CustomNode::Mapping {
            pairs: pairs.clone(),
            meta: merged_meta(new_meta, src_meta),
            flow_style: *flow_style || *src_flow_style,
        },
        (
            CustomNode::Sequence {
                items,
                flow_style,
                meta: new_meta,
                ..
            },
            CustomNode::Sequence {
                meta: src_meta,
                flow_style: src_flow_style,
                ..
            },
        ) => CustomNode::Sequence {
            items: items.clone(),
            meta: merged_meta(new_meta, src_meta),
            flow_style: *flow_style || *src_flow_style,
        },
        (CustomNode::Null { meta: new_meta }, CustomNode::Null { meta: src_meta }) => {
            CustomNode::Null {
                meta: merged_meta(new_meta, src_meta),
            }
        }
        _ => target.clone(),
    }
}

/// The metadata a replacement node ends up carrying, decided per field.
///
/// The direction is not the same for every field, and the reason is the difference between the note on
/// a line and the type of the value on it:
///
/// - `comment` and `anchor` belong to the **document**. A note is the line's, and an anchor is a name
///   other nodes already refer to - replacing a value must not orphan a `*defaults` that still names
///   it. The replaced node wins; the incoming one fills a gap.
/// - `tag` states the type of the value that sits there **now**. The value is exactly what is being
///   replaced, so the incoming tag is the truth about it, and the old tag survives only when the
///   incoming node carries none (`port: !!int 8080` set to `9090` keeps the author's spelling).
///
/// Taking the old tag unconditionally is what made `doc.set("$.d", b"hi")` emit `d: aGk=` - a base64
/// *string* where bytes had been assigned, in the one function whose name promises preservation.
/// `decor` and `source_range` describe the old node's place in the source text, so neither travels;
/// the region writer re-derives both.
fn merged_meta(incoming: &NodeMeta, replaced: &NodeMeta) -> NodeMeta {
    NodeMeta {
        comment: replaced
            .comment
            .clone()
            .or_else(|| incoming.comment.clone()),
        anchor: replaced.anchor.clone().or_else(|| incoming.anchor.clone()),
        tag: incoming.tag.clone().or_else(|| replaced.tag.clone()),
        decor: None,
        source_range: None,
    }
}

fn needs_quoting(value: &str) -> bool {
    value.is_empty()
        || value
            .chars()
            .any(|c| c.is_whitespace() || ":{}[],&#*!|>".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Chomping;

    #[test]
    fn test_with_metadata_from_copies_anchor() {
        let target = CustomNode::plain_scalar("val");
        let src = CustomNode::Scalar {
            value: "".into(),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            block_indent: None,
            meta: NodeMeta {
                anchor: Some("myanchor".into()),
                ..Default::default()
            },
        };
        let result = with_metadata_from(&target, &src);
        match result {
            CustomNode::Scalar { value, meta, .. } => {
                assert_eq!(value.as_ref(), "val");
                assert_eq!(meta.anchor, Some("myanchor".into()));
            }
            _ => panic!("expected Scalar"),
        }
    }

    #[test]
    fn test_with_metadata_from_copies_tag() {
        let target = CustomNode::plain_scalar("val");
        let src = CustomNode::Scalar {
            value: "".into(),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            block_indent: None,
            meta: NodeMeta {
                tag: Some(crate::ast::Tag::local("custom")),
                ..Default::default()
            },
        };
        let result = with_metadata_from(&target, &src);
        match result {
            CustomNode::Scalar { meta, .. } => {
                assert_eq!(meta.tag, Some(crate::ast::Tag::local("custom")));
            }
            _ => panic!("expected Scalar"),
        }
    }

    #[test]
    fn test_with_metadata_from_preserves_target_value() {
        let target = CustomNode::plain_scalar("newval");
        let src = CustomNode::plain_scalar("oldval");
        let result = with_metadata_from(&target, &src);
        match result {
            CustomNode::Scalar { value, .. } => {
                assert_eq!(value.as_ref(), "newval");
            }
            _ => panic!("expected Scalar"),
        }
    }

    #[test]
    fn test_needs_quoting_empty() {
        assert!(needs_quoting(""));
    }

    #[test]
    fn test_needs_quoting_whitespace() {
        assert!(needs_quoting("hello world"));
    }

    #[test]
    fn test_needs_quoting_special_chars() {
        assert!(needs_quoting("a:b"));
        assert!(needs_quoting("{a}"));
    }

    #[test]
    fn test_needs_quoting_plain() {
        assert!(!needs_quoting("hello"));
        assert!(!needs_quoting("42"));
    }

    /// A tag states the type of the value sitting there now, and the value is exactly what the edit
    /// replaces. Taking the old node's tag unconditionally dropped the incoming one, so assigning bytes
    /// through `YamlDocument.set` emitted `d: aGk=` - the base64 text of a `!!binary` value, written as
    /// a plain string, re-read as a string.
    #[test]
    fn an_incoming_tag_survives_the_replacement() {
        let mut incoming = CustomNode::plain_scalar("aGk=");
        incoming.set_tag(crate::ast::Tag::primary("binary"));
        let replaced = CustomNode::plain_scalar("x");
        let result = with_metadata_from(&incoming, &replaced);
        assert_eq!(
            result.tag().map(|tag| tag.to_string()),
            Some("!!binary".to_string()),
            "the tag the caller assigned has to be the tag that is emitted"
        );
    }

    /// The other direction stays as it was: a document that spells its type out is not re-typed because
    /// the caller handed over an untagged node. `port: !!int 8080` set to `9090` keeps `!!int`.
    #[test]
    fn a_replacement_carrying_no_tag_keeps_the_documents_spelling() {
        let mut replaced = CustomNode::plain_scalar("8080");
        replaced.set_tag(crate::ast::Tag::primary("int"));
        let incoming = CustomNode::plain_scalar("9090");
        let result = with_metadata_from(&incoming, &replaced);
        assert_eq!(
            result.tag().map(|tag| tag.to_string()),
            Some("!!int".to_string()),
            "an incoming value without a tag must not delete the author's"
        );
    }

    /// An anchor is a name the rest of the document already refers to. Dropping it would leave a
    /// `*defaults` pointing at nothing, so the document's name wins over the incoming node's.
    #[test]
    fn the_anchor_the_document_aliases_names_is_kept() {
        let mut incoming = CustomNode::plain_scalar("v");
        incoming.set_anchor("fresh");
        let mut replaced = CustomNode::plain_scalar("old");
        replaced.set_anchor("defaults");
        let result = with_metadata_from(&incoming, &replaced);
        assert_eq!(result.anchor(), Some("defaults"));
    }

    /// A note is the line's, not the value's - the same direction as the anchor, for the same reason:
    /// replacing a value says nothing about the comment beside it.
    #[test]
    fn a_note_stays_with_the_line() {
        let mut incoming = CustomNode::plain_scalar("v");
        incoming.set_comment(crate::ast::Comment {
            text: "incoming".into(),
            standalone: false,
        });
        let mut replaced = CustomNode::plain_scalar("old");
        replaced.set_comment(crate::ast::Comment {
            text: "the line's own note".into(),
            standalone: false,
        });
        let result = with_metadata_from(&incoming, &replaced);
        assert_eq!(
            result.comment().map(|c| c.text.to_string()).as_deref(),
            Some("the line's own note")
        );
    }

    /// Containers inherit the same way, or the rule would only cover half the assignments: a tagged
    /// mapping written into an untagged slot has to arrive tagged.
    #[test]
    fn a_tagged_container_keeps_its_tag_through_the_inheritance() {
        let mut incoming = CustomNode::plain_mapping(Default::default());
        incoming.set_tag(crate::ast::Tag::local("settings"));
        let replaced = CustomNode::plain_mapping(Default::default());
        let result = with_metadata_from(&incoming, &replaced);
        assert_eq!(
            result.tag().map(|tag| tag.to_string()),
            Some("!settings".to_string())
        );
    }
}
