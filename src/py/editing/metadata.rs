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
