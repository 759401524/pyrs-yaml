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
            CustomNode::Scalar { value, style, .. },
            CustomNode::Scalar {
                style: src_style,
                meta: src_meta,
                chomping,
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
                meta: NodeMeta {
                    comment: src_meta.comment.clone(),
                    anchor: src_meta.anchor.clone(),
                    tag: src_meta.tag.clone(),
                    ..Default::default()
                },
                chomping: *chomping,
            }
        }
        (
            CustomNode::Mapping {
                pairs, flow_style, ..
            },
            CustomNode::Mapping {
                meta: src_meta,
                flow_style: src_flow_style,
                ..
            },
        ) => CustomNode::Mapping {
            pairs: pairs.clone(),
            meta: NodeMeta {
                comment: src_meta.comment.clone(),
                anchor: src_meta.anchor.clone(),
                tag: src_meta.tag.clone(),
                ..Default::default()
            },
            flow_style: *flow_style || *src_flow_style,
        },
        (
            CustomNode::Sequence {
                items, flow_style, ..
            },
            CustomNode::Sequence {
                meta: src_meta,
                flow_style: src_flow_style,
                ..
            },
        ) => CustomNode::Sequence {
            items: items.clone(),
            meta: NodeMeta {
                comment: src_meta.comment.clone(),
                anchor: src_meta.anchor.clone(),
                tag: src_meta.tag.clone(),
                ..Default::default()
            },
            flow_style: *flow_style || *src_flow_style,
        },
        (CustomNode::Null { .. }, CustomNode::Null { meta: src_meta, .. }) => CustomNode::Null {
            meta: NodeMeta {
                comment: src_meta.comment.clone(),
                anchor: src_meta.anchor.clone(),
                tag: src_meta.tag.clone(),
                ..Default::default()
            },
        },
        _ => target.clone(),
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
}
