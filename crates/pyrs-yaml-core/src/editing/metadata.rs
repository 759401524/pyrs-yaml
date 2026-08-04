//! Metadata preservation during edits.
//!
//! Pure Rust implementation — no PyO3 dependencies.

use crate::ast::{CustomNode, ScalarStyle};

pub fn with_metadata_from(target: &CustomNode, src: &CustomNode) -> CustomNode {
    match (target, src) {
        (
            CustomNode::Scalar { value, style, .. },
            CustomNode::Scalar {
                style: src_style,
                comment,
                anchor,
                tag,
                chomping,
                ..
            },
        ) => {
            let new_style = if *style == ScalarStyle::Plain
                && *src_style != ScalarStyle::Plain
                && !needs_quoting(value)
            {
                src_style.clone()
            } else {
                style.clone()
            };
            CustomNode::Scalar {
                value: value.clone(),
                style: new_style,
                comment: comment.clone(),
                anchor: anchor.clone(),
                tag: tag.clone(),
                chomping: chomping.clone(),
                source_range: None,
            }
        }
        (
            CustomNode::Mapping {
                pairs, flow_style, ..
            },
            CustomNode::Mapping {
                comment,
                anchor,
                tag,
                flow_style: src_flow_style,
                ..
            },
        ) => CustomNode::Mapping {
            pairs: pairs.clone(),
            comment: comment.clone(),
            anchor: anchor.clone(),
            tag: tag.clone(),
            flow_style: *flow_style || *src_flow_style,
            source_range: None,
        },
        (
            CustomNode::Sequence {
                items, flow_style, ..
            },
            CustomNode::Sequence {
                comment,
                anchor,
                tag,
                flow_style: src_flow_style,
                ..
            },
        ) => CustomNode::Sequence {
            items: items.clone(),
            comment: comment.clone(),
            anchor: anchor.clone(),
            tag: tag.clone(),
            flow_style: *flow_style || *src_flow_style,
            source_range: None,
        },
        (
            CustomNode::Null { .. },
            CustomNode::Null {
                comment,
                anchor,
                tag,
                ..
            },
        ) => CustomNode::Null {
            comment: comment.clone(),
            anchor: anchor.clone(),
            tag: tag.clone(),
            source_range: None,
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
            source_range: None,
            anchor: Some("myanchor".into()),
            tag: None,
            comment: None,
        };
        let result = with_metadata_from(&target, &src);
        match result {
            CustomNode::Scalar { value, anchor, .. } => {
                assert_eq!(value, "val");
                assert_eq!(anchor, Some("myanchor".into()));
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
            source_range: None,
            anchor: None,
            tag: Some(crate::ast::Tag::local("custom")),
            comment: None,
        };
        let result = with_metadata_from(&target, &src);
        match result {
            CustomNode::Scalar { tag, .. } => {
                assert_eq!(tag, Some(crate::ast::Tag::local("custom")));
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
                assert_eq!(value, "newval");
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
