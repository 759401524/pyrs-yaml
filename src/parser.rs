use crate::ast::{Comment, CustomNode, ScalarStyle};
use indexmap::IndexMap;
use yaml_rust2::scanner::{Marker, Scanner, Token, TokenType, TScalarStyle};

/// A comment extracted from the raw YAML text
#[derive(Debug, Clone)]
struct RawComment {
    line: usize,
    col: usize,
    text: String,
    standalone: bool,
}

/// Extract comments from raw YAML text by scanning line by line
fn extract_comments(yaml: &str) -> Vec<RawComment> {
    let mut comments = Vec::new();

    for (line_idx, line) in yaml.lines().enumerate() {
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut escaped = false;

        for (col_idx, ch) in line.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' && (in_single_quote || in_double_quote) {
                escaped = true;
                continue;
            }
            if ch == '\'' && !in_double_quote {
                in_single_quote = !in_single_quote;
                continue;
            }
            if ch == '"' && !in_single_quote {
                in_double_quote = !in_double_quote;
                continue;
            }
            if ch == '#' && !in_single_quote && !in_double_quote {
                let comment_text = line[col_idx + 1..].trim().to_string();
                let is_standalone = line[..col_idx].trim().is_empty();
                comments.push(RawComment {
                    line: line_idx,
                    col: col_idx,
                    text: comment_text,
                    standalone: is_standalone,
                });
                break;
            }
        }
    }

    comments
}

/// Find an inline comment on the same line at a column after `after_col`
fn find_inline_comment(comments: &[RawComment], start_idx: &mut usize, line: usize, after_col: usize) -> Option<Comment> {
    while *start_idx < comments.len() {
        let c = &comments[*start_idx];
        if c.line > line {
            return None;
        }
        if c.line < line {
            *start_idx += 1;
            continue;
        }
        // Same line
        if c.col > after_col && !c.standalone {
            let comment = Comment {
                text: c.text.clone(),
                standalone: false,
            };
            *start_idx += 1;
            return Some(comment);
        }
        *start_idx += 1;
    }
    None
}

/// Find the next standalone comment before a given line
fn find_standalone_comment_before(comments: &[RawComment], start_idx: &mut usize, before_line: usize) -> Option<Comment> {
    let mut result = None;
    while *start_idx < comments.len() {
        let c = &comments[*start_idx];
        if c.line >= before_line {
            break;
        }
        if c.standalone {
            result = Some(Comment {
                text: c.text.clone(),
                standalone: true,
            });
        }
        *start_idx += 1;
    }
    result
}

/// Parse a YAML string into a CustomNode AST
pub fn parse(yaml: &str) -> Result<CustomNode, String> {
    let mut scanner = Scanner::new(yaml.chars());
    let mut tokens_with_pos = Vec::new();

    loop {
        match scanner.next_token() {
            Ok(Some(Token(marker, token_type))) => tokens_with_pos.push((marker, token_type)),
            Ok(None) => break,
            Err(e) => return Err(format!("Scan error: {}", e)),
        }
    }

    let raw_comments = extract_comments(yaml);
    let mut parser = TokenParser::new(tokens_with_pos, raw_comments);
    parser.parse_document()
}

struct TokenParser {
    tokens: Vec<(Marker, TokenType)>,
    pos: usize,
    raw_comments: Vec<RawComment>,
    comment_idx: usize,
}

impl TokenParser {
    fn new(tokens: Vec<(Marker, TokenType)>, raw_comments: Vec<RawComment>) -> Self {
        Self {
            tokens,
            pos: 0,
            raw_comments,
            comment_idx: 0,
        }
    }

    fn peek(&self) -> Option<&TokenType> {
        self.tokens.get(self.pos).map(|(_, t)| t)
    }

    fn peek_marker(&self) -> Option<&Marker> {
        self.tokens.get(self.pos).map(|(m, _)| m)
    }

    fn next(&mut self) -> Option<&TokenType> {
        let token = self.tokens.get(self.pos).map(|(_, t)| t);
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn marker_line_col(&self, offset: isize) -> (usize, usize) {
        let idx = (self.pos as isize + offset).max(0) as usize;
        self.tokens
            .get(idx)
            .map(|(m, _)| (m.line() - 1, m.col()))
            .unwrap_or((0, 0))
    }

    fn parse_document(&mut self) -> Result<CustomNode, String> {
        if let Some(TokenType::StreamStart(_)) = self.peek() {
            self.next();
        }
        if let Some(TokenType::DocumentStart) = self.peek() {
            self.next();
        }

        let node = self.parse_node()?;

        if let Some(TokenType::DocumentEnd) = self.peek() {
            self.next();
        }
        if let Some(TokenType::StreamEnd) = self.peek() {
            self.next();
        }

        Ok(node)
    }

    fn parse_node(&mut self) -> Result<CustomNode, String> {
        // Before parsing this node, collect any standalone comments before its line
        let (line, _col) = self.marker_line_col(0);
        let standalone = find_standalone_comment_before(
            &self.raw_comments,
            &mut self.comment_idx,
            line,
        );

        match self.peek().cloned() {
            Some(TokenType::Scalar(style, value)) => {
                let scalar_value = value.clone();
                let scalar_style = Self::map_style(&style);
                let (line, col) = self.marker_line_col(0);
                self.next();

                // Look for inline comment after this scalar on the same line
                let inline = find_inline_comment(
                    &self.raw_comments,
                    &mut self.comment_idx,
                    line,
                    col + value.len(),
                );

                // Inline comment wins over standalone
                let comment = inline.or(standalone);

                Ok(CustomNode::Scalar {
                    value: scalar_value,
                    style: scalar_style,
                    comment,
                    anchor: None,
                })
            }
            Some(TokenType::BlockMappingStart) => {
                self.next();
                let mut mapping = self.parse_mapping()?;
                // Attach standalone comment to the mapping
                if let Some(s) = standalone {
                    mapping.set_comment(s);
                }
                Ok(mapping)
            }
            Some(TokenType::BlockSequenceStart) => {
                self.next();
                let mut seq = self.parse_sequence()?;
                if let Some(s) = standalone {
                    seq.set_comment(s);
                }
                Ok(seq)
            }
            Some(TokenType::FlowMappingStart) => {
                self.next();
                let mut mapping = self.parse_flow_mapping()?;
                if let Some(s) = standalone {
                    mapping.set_comment(s);
                }
                Ok(mapping)
            }
            Some(TokenType::FlowSequenceStart) => {
                self.next();
                let mut seq = self.parse_flow_sequence()?;
                if let Some(s) = standalone {
                    seq.set_comment(s);
                }
                Ok(seq)
            }
            Some(TokenType::Key) => {
                self.next();
                self.parse_mapping()
            }
            Some(TokenType::Value) => {
                self.next();
                self.parse_node()
            }
            Some(TokenType::BlockEntry) => {
                self.next();
                self.parse_node()
            }
            Some(TokenType::BlockEnd) => {
                self.next();
                if let Some(s) = standalone {
                    Ok(CustomNode::Null {
                        comment: Some(s),
                        anchor: None,
                    })
                } else {
                    Ok(CustomNode::Null {
                        comment: None,
                        anchor: None,
                    })
                }
            }
            Some(TokenType::Alias(name)) => {
                let alias_name = name.clone();
                self.next();
                Ok(CustomNode::Alias { name: alias_name })
            }
            Some(TokenType::Anchor(name)) => {
                let anchor_name = name.clone();
                self.next();
                let mut node = self.parse_node()?;
                match &mut node {
                    CustomNode::Scalar { anchor, .. }
                    | CustomNode::Mapping { anchor, .. }
                    | CustomNode::Sequence { anchor, .. }
                    | CustomNode::Null { anchor, .. } => {
                        *anchor = Some(anchor_name);
                    }
                    _ => {}
                }
                Ok(node)
            }
            None => Ok(CustomNode::Null {
                comment: None,
                anchor: None,
            }),
            _ => {
                self.next();
                if self.peek().is_some() {
                    self.parse_node()
                } else {
                    Ok(CustomNode::Null {
                        comment: None,
                        anchor: None,
                    })
                }
            }
        }
    }

    fn parse_mapping(&mut self) -> Result<CustomNode, String> {
        let mut pairs = IndexMap::new();

        loop {
            match self.peek().cloned() {
                Some(TokenType::BlockEnd) => {
                    self.next();
                    break;
                }
                Some(TokenType::Key) => {
                    self.next();
                    let key = self.parse_key_for_pair()?;
                    let value = self.parse_value_for_pair()?;
                    pairs.insert(key, value);
                }
                Some(TokenType::Scalar(style, key_value)) => {
                    let key_str = key_value.clone();
                    let key_style = Self::map_style(&style);

                    // Collect standalone comments before this key
                    let (line, _col) = self.marker_line_col(0);
                    let standalone = find_standalone_comment_before(
                        &self.raw_comments,
                        &mut self.comment_idx,
                        line,
                    );

                    let (key_line, key_col) = self.marker_line_col(0);
                    self.next();

                    // Look for inline comment after the key text on the same line
                    let key_comment = find_inline_comment(
                        &self.raw_comments,
                        &mut self.comment_idx,
                        key_line,
                        key_col + key_value.len(),
                    );

                    let key = CustomNode::Scalar {
                        value: key_str,
                        style: key_style,
                        comment: key_comment.or(standalone),
                        anchor: None,
                    };

                    if let Some(TokenType::Value) = self.peek() {
                        self.next();
                    }
                    let value = self.parse_node()?;
                    pairs.insert(key, value);
                }
                None => break,
                _ => {
                    self.next();
                }
            }
        }

        Ok(CustomNode::Mapping {
            pairs,
            comment: None,
            anchor: None,
        })
    }

    /// Parse a key in a key-value pair (handles Key token + scalar)
    fn parse_key_for_pair(&mut self) -> Result<CustomNode, String> {
        // Collect standalone comments before this key
        let (line, _col) = self.marker_line_col(0);
        let standalone = find_standalone_comment_before(
            &self.raw_comments,
            &mut self.comment_idx,
            line,
        );

        match self.peek().cloned() {
            Some(TokenType::Scalar(style, key_value)) => {
                let key_str = key_value.clone();
                let key_style = Self::map_style(&style);
                self.next();

                // Don't look for inline comments on the key - they belong to the value
                Ok(CustomNode::Scalar {
                    value: key_str,
                    style: key_style,
                    comment: standalone,
                    anchor: None,
                })
            }
            _ => {
                // Complex key - parse as node
                let mut node = self.parse_node()?;
                if let Some(s) = standalone {
                    node.set_comment(s);
                }
                Ok(node)
            }
        }
    }

    /// Parse a value in a key-value pair (handles Value token + scalar)
    fn parse_value_for_pair(&mut self) -> Result<CustomNode, String> {
        if let Some(TokenType::Value) = self.peek() {
            // Get position of Value token
            let (_val_line, val_col) = self.marker_line_col(0);
            self.next();

            // Get position of the value scalar
            let (val_scalar_line, val_scalar_col) = self.marker_line_col(0);

            // Parse the value node
            let mut node = self.parse_node()?;

            // If the value is a scalar, look for inline comment after it
            if let CustomNode::Scalar { value, comment, .. } = &mut node {
                if comment.is_none() {
                    let inline = find_inline_comment(
                        &self.raw_comments,
                        &mut self.comment_idx,
                        val_scalar_line,
                        val_scalar_col + value.len(),
                    );
                    *comment = inline;
                }
            }

            Ok(node)
        } else {
            self.parse_node()
        }
    }

    fn parse_flow_mapping(&mut self) -> Result<CustomNode, String> {
        let mut pairs = IndexMap::new();

        loop {
            match self.peek().cloned() {
                Some(TokenType::FlowMappingEnd) => {
                    self.next();
                    break;
                }
                Some(TokenType::Key) => {
                    self.next();
                    let key = self.parse_key_for_pair()?;
                    let value = self.parse_value_for_pair()?;
                    pairs.insert(key, value);
                }
                Some(TokenType::Scalar(style, key_value)) => {
                    let key_str = key_value.clone();
                    let key_style = Self::map_style(&style);
                    self.next();
                    if let Some(TokenType::Value) = self.peek() {
                        self.next();
                    }
                    let value = self.parse_node()?;
                    let key = CustomNode::Scalar {
                        value: key_str,
                        style: key_style,
                        comment: None,
                        anchor: None,
                    };
                    pairs.insert(key, value);
                }
                Some(TokenType::FlowEntry) => {
                    self.next();
                }
                None => break,
                _ => {
                    self.next();
                }
            }
        }

        Ok(CustomNode::Mapping {
            pairs,
            comment: None,
            anchor: None,
        })
    }

    fn parse_sequence(&mut self) -> Result<CustomNode, String> {
        let mut items = Vec::new();

        loop {
            match self.peek().cloned() {
                Some(TokenType::BlockEnd) => {
                    self.next();
                    break;
                }
                Some(TokenType::BlockEntry) => {
                    self.next();
                    let item = self.parse_node()?;
                    items.push(item);
                }
                Some(TokenType::Value) => {
                    self.next();
                    let item = self.parse_node()?;
                    items.push(item);
                }
                None => break,
                _ => {
                    let item = self.parse_node()?;
                    items.push(item);
                }
            }
        }

        Ok(CustomNode::Sequence {
            items,
            comment: None,
            anchor: None,
        })
    }

    fn parse_flow_sequence(&mut self) -> Result<CustomNode, String> {
        let mut items = Vec::new();

        loop {
            match self.peek().cloned() {
                Some(TokenType::FlowSequenceEnd) => {
                    self.next();
                    break;
                }
                Some(TokenType::FlowEntry) => {
                    self.next();
                    let item = self.parse_node()?;
                    items.push(item);
                }
                None => break,
                _ => {
                    let item = self.parse_node()?;
                    items.push(item);
                }
            }
        }

        Ok(CustomNode::Sequence {
            items,
            comment: None,
            anchor: None,
        })
    }

    fn map_style(style: &TScalarStyle) -> ScalarStyle {
        match style {
            TScalarStyle::Plain => ScalarStyle::Plain,
            TScalarStyle::SingleQuoted => ScalarStyle::SingleQuoted,
            TScalarStyle::DoubleQuoted => ScalarStyle::DoubleQuoted,
            TScalarStyle::Literal => ScalarStyle::Literal,
            TScalarStyle::Folded => ScalarStyle::Folded,
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
}
