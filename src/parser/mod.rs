pub mod yaml;

use crate::ast::{Chomping, Comment, CustomNode, ScalarStyle, Tag};
use indexmap::IndexMap;
use saphyr_parser::{Event, Parser as SaphyrParser, ScalarStyle as SaphyrScalarStyle, Span, SpannedEventReceiver};
use yaml::*;

/// Parse a YAML string into a CustomNode AST using saphyr-parser
pub fn parse(yaml: &str) -> Result<CustomNode, String> {
    // Handle empty YAML
    if yaml.trim().is_empty() {
        return Ok(CustomNode::Null {
            comment: None,
            anchor: None,
            tag: None,
        });
    }

    // Extract comments and anchors from raw text before parsing
    let raw_comments = extract_comments(yaml);
    let raw_anchors = extract_anchors(yaml);

    // Parse YAML using saphyr-parser
    let mut receiver = AstReceiver::new(yaml, raw_comments, raw_anchors);
    let mut parser = SaphyrParser::new_from_str(yaml);

    parser
        .load(&mut receiver, true)
        .map_err(|e| format!("YAML parse error: {}", e))?;

    // Get the parsed node (handle empty documents)
    let mut node = receiver.result.unwrap_or(CustomNode::Null {
        comment: None,
        anchor: None,
        tag: None,
    });

    // Resolve merge keys (<<) after parsing
    resolve_merge_keys(&mut node);

    Ok(node)
}

/// Convert saphyr tag to our Tag format
fn convert_tag(tag: &saphyr_parser::Tag) -> Tag {
    // saphyr uses full URIs like "tag:yaml.org,2002:str"
    // We need to convert back to short form like "!!str"
    let handle = &tag.handle;
    let suffix = &tag.suffix;

    if handle == "tag:yaml.org,2002:" {
        // Core schema tag - use !! prefix
        Tag {
            handle: "!!".to_string(),
            suffix: suffix.to_string(),
        }
    } else if handle == "!" {
        // Local tag
        Tag {
            handle: "!".to_string(),
            suffix: suffix.to_string(),
        }
    } else {
        // Other tags
        Tag {
            handle: handle.to_string(),
            suffix: suffix.to_string(),
        }
    }
}

/// Event receiver that builds CustomNode AST
struct AstReceiver<'a> {
    yaml_text: &'a str,
    raw_comments: Vec<RawComment>,
    raw_anchors: Vec<RawAnchor>,
    comment_idx: usize,
    anchor_idx: usize,
    stack: Vec<ParseState>,
    result: Option<CustomNode>,
    /// Current anchor ID to name mapping
    anchors: std::collections::HashMap<usize, String>,
    /// Next anchor name to use (from raw text, in order)
    next_anchor_name: usize,
    /// Pending standalone comment for the next node
    pending_standalone_comment: Option<Comment>,
}

#[derive(Debug)]
enum ParseState {
    /// Building a mapping
    Mapping {
        pairs: IndexMap<CustomNode, CustomNode>,
        current_key: Option<CustomNode>,
        anchor_id: usize,
    },
    /// Building a sequence
    Sequence {
        items: Vec<CustomNode>,
        anchor_id: usize,
    },
}

impl<'a> AstReceiver<'a> {
    fn new(yaml_text: &'a str, raw_comments: Vec<RawComment>, raw_anchors: Vec<RawAnchor>) -> Self {
        Self {
            yaml_text,
            raw_comments,
            raw_anchors,
            comment_idx: 0,
            anchor_idx: 0,
            stack: Vec::new(),
            result: None,
            anchors: std::collections::HashMap::new(),
            next_anchor_name: 0,
            pending_standalone_comment: None,
        }
    }

    /// Find inline comment on a given line after a column
    fn find_inline_comment(&mut self, line: usize, after_col: usize) -> Option<Comment> {
        // Save current position
        let saved_idx = self.comment_idx;

        while self.comment_idx < self.raw_comments.len() {
            let c = &self.raw_comments[self.comment_idx];
            if c.line > line {
                break;
            }
            if c.line < line {
                self.comment_idx += 1;
                continue;
            }
            // Same line - check if it's after the column
            if c.col >= after_col && !c.standalone {
                let comment = Comment {
                    text: c.text.clone(),
                    standalone: false,
                };
                // Don't advance comment_idx - we might need this comment again
                return Some(comment);
            }
            self.comment_idx += 1;
        }

        // Restore position if no comment found
        self.comment_idx = saved_idx;
        None
    }

    /// Find standalone comments before a given line
    fn find_standalone_before_line(&mut self, line: usize) -> Option<Comment> {
        let mut result = None;
        while self.comment_idx < self.raw_comments.len() {
            let c = &self.raw_comments[self.comment_idx];
            if c.line >= line {
                break;
            }
            if c.standalone {
                result = Some(Comment {
                    text: c.text.clone(),
                    standalone: true,
                });
            }
            self.comment_idx += 1;
        }
        result
    }

    /// Get next anchor name from raw text (in order)
    fn next_anchor_name_from_raw(&mut self) -> Option<String> {
        if self.next_anchor_name < self.raw_anchors.len() {
            let name = self.raw_anchors[self.next_anchor_name].name.clone();
            self.next_anchor_name += 1;
            Some(name)
        } else {
            None
        }
    }

    /// Create a scalar node from value and style
    fn create_scalar(
        &mut self,
        value: &str,
        style: &SaphyrScalarStyle,
        line: usize,
    ) -> CustomNode {
        let scalar_style = match style {
            SaphyrScalarStyle::Plain => ScalarStyle::Plain,
            SaphyrScalarStyle::SingleQuoted => ScalarStyle::SingleQuoted,
            SaphyrScalarStyle::DoubleQuoted => ScalarStyle::DoubleQuoted,
            SaphyrScalarStyle::Literal => ScalarStyle::Literal,
            SaphyrScalarStyle::Folded => ScalarStyle::Folded,
        };

        // Detect chomping for block scalars
        let chomping = if matches!(scalar_style, ScalarStyle::Literal | ScalarStyle::Folded) {
            detect_chomping(self.yaml_text, line)
        } else {
            Chomping::Clip
        };

        // Unescape double-quoted strings
        let scalar_value = if matches!(style, SaphyrScalarStyle::DoubleQuoted) {
            unescape_double_quoted(value)
        } else {
            value.to_string()
        };

        // Find inline comment - look for comment on the same line after the value
        // The value typically starts at column 0 for keys, or after ": " for values
        // We need to find the position after the value ends
        let value_end_col = if line < self.yaml_text.lines().count() {
            let line_text = self.yaml_text.lines().nth(line).unwrap_or("");
            // Find where the value ends by looking for the comment
            if let Some(comment_pos) = line_text.find('#') {
                comment_pos
            } else {
                line_text.len()
            }
        } else {
            value.len()
        };

        let inline = self.find_inline_comment(line, value_end_col);

        CustomNode::Scalar {
            value: scalar_value,
            style: scalar_style,
            comment: inline,
            anchor: None,
            tag: None,
            chomping,
        }
    }

    /// Push a node to the current context
    fn push_node(&mut self, node: CustomNode) {
        match self.stack.last_mut() {
            Some(ParseState::Mapping {
                current_key, ..
            }) => {
                if current_key.is_none() {
                    // This is a key
                    *current_key = Some(node);
                } else {
                    // This is a value - insert the pair
                    let key = current_key.take().unwrap();
                    if let Some(ParseState::Mapping { pairs, .. }) = self.stack.last_mut() {
                        pairs.insert(key, node);
                    }
                }
            }
            Some(ParseState::Sequence { items, .. }) => {
                items.push(node);
            }
            None => {
                // Top level
                self.result = Some(node);
            }
        }
    }
}

impl<'a> SpannedEventReceiver<'a> for AstReceiver<'a> {
    fn on_event(&mut self, event: Event<'a>, span: Span) {
        match event {
            Event::StreamStart | Event::StreamEnd | Event::DocumentStart(_) | Event::DocumentEnd => {
                // Ignore these events
            }
            Event::Scalar(value, style, anchor_id, tag) => {
                let line = span.start.line() - 1; // Convert to 0-indexed

                // Find standalone comments before this line
                let standalone = self.find_standalone_before_line(line);

                let mut node = self.create_scalar(&value, &style, line);

                // Attach standalone comment if found
                if let Some(comment) = standalone {
                    match &mut node {
                        CustomNode::Scalar { comment: c, .. } => {
                            // If there's already an inline comment, keep it
                            // Otherwise, use the standalone comment
                            if c.is_none() {
                                *c = Some(comment);
                            }
                        }
                        _ => {}
                    }
                }

                // Handle anchor - use next raw anchor name
                if anchor_id != 0 {
                    if let Some(name) = self.next_anchor_name_from_raw() {
                        self.anchors.insert(anchor_id, name.clone());
                        match &mut node {
                            CustomNode::Scalar { anchor, .. } => {
                                *anchor = Some(name);
                            }
                            _ => {}
                        }
                    }
                }

                // Handle tag
                if let Some(tag) = tag {
                    match &mut node {
                        CustomNode::Scalar { tag: t, .. } => {
                            *t = Some(convert_tag(&tag));
                        }
                        _ => {}
                    }
                }

                self.push_node(node);
            }
            Event::MappingStart(anchor_id, tag) => {
                let line = span.start.line() - 1;

                // Find standalone comments before this line
                let standalone = self.find_standalone_before_line(line);

                // Handle anchor - use next raw anchor name
                let anchor_name = if anchor_id != 0 {
                    if let Some(name) = self.next_anchor_name_from_raw() {
                        self.anchors.insert(anchor_id, name.clone());
                        Some(name)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Handle tag
                let tag_obj = tag.map(|t| convert_tag(&t));

                self.stack.push(ParseState::Mapping {
                    pairs: IndexMap::new(),
                    current_key: None,
                    anchor_id,
                });

                // Store standalone comment for when we pop
                if let Some(comment) = standalone {
                    // We'll attach this to the mapping when we pop
                    // For now, store it temporarily
                    self.pending_standalone_comment = Some(comment);
                }
            }
            Event::MappingEnd => {
                if let Some(ParseState::Mapping { pairs, anchor_id, .. }) = self.stack.pop() {
                    // Get anchor name from self.anchors
                    let anchor = self.anchors.get(&anchor_id).cloned();

                    // Get pending standalone comment
                    let comment = self.pending_standalone_comment.take();

                    let mapping = CustomNode::Mapping {
                        pairs,
                        comment,
                        anchor,
                        tag: None,
                    };
                    self.push_node(mapping);
                }
            }
            Event::SequenceStart(anchor_id, tag) => {
                let line = span.start.line() - 1;

                // Find standalone comments before this line
                let standalone = self.find_standalone_before_line(line);

                // Handle anchor - use next raw anchor name
                if anchor_id != 0 {
                    if let Some(name) = self.next_anchor_name_from_raw() {
                        self.anchors.insert(anchor_id, name.clone());
                    }
                }

                // Handle tag
                let _tag_obj = tag.map(|t| convert_tag(&t));

                self.stack.push(ParseState::Sequence {
                    items: Vec::new(),
                    anchor_id,
                });

                // Store standalone comment for when we pop
                if let Some(comment) = standalone {
                    self.pending_standalone_comment = Some(comment);
                }
            }
            Event::SequenceEnd => {
                if let Some(ParseState::Sequence { items, anchor_id, .. }) = self.stack.pop() {
                    // Get anchor name from self.anchors
                    let anchor = self.anchors.get(&anchor_id).cloned();

                    // Get pending standalone comment
                    let comment = self.pending_standalone_comment.take();

                    let seq = CustomNode::Sequence {
                        items,
                        comment,
                        anchor,
                        tag: None,
                    };
                    self.push_node(seq);
                }
            }
            Event::Alias(anchor_id) => {
                let alias_name = self
                    .anchors
                    .get(&anchor_id)
                    .cloned()
                    .unwrap_or_else(|| format!("alias_{}", anchor_id));

                let node = CustomNode::Alias {
                    name: alias_name,
                };
                self.push_node(node);
            }
            Event::Nothing => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_scalar() {
        let result = parse("hello");
        assert!(result.is_ok());
        if let Ok(CustomNode::Scalar { value, style, .. }) = result {
            assert_eq!(value, "hello");
            assert_eq!(style, ScalarStyle::Plain);
        }
    }

    #[test]
    fn test_parse_mapping() {
        let yaml = "key: value";
        let result = parse(yaml);
        assert!(result.is_ok());
        if let Ok(CustomNode::Mapping { pairs, .. }) = result {
            assert_eq!(pairs.len(), 1);
        }
    }

    #[test]
    fn test_parse_sequence() {
        let yaml = "- item1\n- item2";
        let result = parse(yaml);
        assert!(result.is_ok());
        if let Ok(CustomNode::Sequence { items, .. }) = result {
            assert_eq!(items.len(), 2);
        }
    }

    #[test]
    fn test_parse_tag() {
        let yaml = "name: !!str John";
        let result = parse(yaml);
        assert!(result.is_ok());
        if let Ok(CustomNode::Mapping { pairs, .. }) = result {
            for (k, v) in pairs {
                if let CustomNode::Scalar { value, .. } = k {
                    if value == "name" {
                        if let CustomNode::Scalar { tag, .. } = v {
                            assert!(tag.is_some());
                            assert_eq!(tag.unwrap().suffix, "str");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_parse_complex_key() {
        let yaml = "? [key1, key2]\n: value";
        let result = parse(yaml);
        assert!(result.is_ok());
        if let Ok(CustomNode::Mapping { pairs, .. }) = result {
            assert_eq!(pairs.len(), 1);
        }
    }

    #[test]
    fn test_parse_empty() {
        let result = parse("");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), CustomNode::Null { .. }));
    }

    #[test]
    fn test_parse_anchor() {
        let yaml = "defaults: &defaults\n  timeout: 30";
        let result = parse(yaml);
        assert!(result.is_ok());
        if let Ok(CustomNode::Mapping { pairs, .. }) = result {
            for (k, v) in pairs {
                if let CustomNode::Scalar { value, .. } = k {
                    if value == "defaults" {
                        if let CustomNode::Mapping { anchor, .. } = v {
                            assert!(anchor.is_some());
                            assert_eq!(anchor.unwrap(), "defaults");
                        }
                    }
                }
            }
        }
    }
}
