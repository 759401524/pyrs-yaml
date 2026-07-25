use crate::ast::{Comment, CustomNode, ScalarStyle};
use indexmap::IndexMap;
use yaml_rust2::scanner::{Scanner, Token, TokenType, TScalarStyle};

/// Parse a YAML string into a CustomNode AST
pub fn parse(yaml: &str) -> Result<CustomNode, String> {
    let mut scanner = Scanner::new(yaml.chars());
    let mut tokens = Vec::new();

    // Collect all tokens
    loop {
        match scanner.next_token() {
            Ok(Some(Token(_marker, token_type))) => tokens.push(token_type),
            Ok(None) => break,
            Err(e) => return Err(format!("Scan error: {}", e)),
        }
    }

    // Parse tokens into AST
    let mut parser = TokenParser::new(tokens);
    parser.parse_document()
}

struct TokenParser {
    tokens: Vec<TokenType>,
    pos: usize,
    /// Pending comment to attach to next node
    pending_comment: Option<Comment>,
}

impl TokenParser {
    fn new(tokens: Vec<TokenType>) -> Self {
        Self {
            tokens,
            pos: 0,
            pending_comment: None,
        }
    }

    fn peek(&self) -> Option<&TokenType> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<&TokenType> {
        let token = self.tokens.get(self.pos);
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn parse_document(&mut self) -> Result<CustomNode, String> {
        // Skip stream start if present
        if let Some(TokenType::StreamStart(_)) = self.peek() {
            self.next();
        }

        // Handle document start
        if let Some(TokenType::DocumentStart) = self.peek() {
            self.next();
        }

        let node = self.parse_node()?;

        // Handle document end
        if let Some(TokenType::DocumentEnd) = self.peek() {
            self.next();
        }

        // Skip stream end
        if let Some(TokenType::StreamEnd) = self.peek() {
            self.next();
        }

        Ok(node)
    }

    fn parse_node(&mut self) -> Result<CustomNode, String> {
        match self.peek() {
            Some(TokenType::Scalar(style, value)) => {
                let scalar_value = value.clone();
                let scalar_style = match style {
                    TScalarStyle::Plain => ScalarStyle::Plain,
                    TScalarStyle::SingleQuoted => ScalarStyle::SingleQuoted,
                    TScalarStyle::DoubleQuoted => ScalarStyle::DoubleQuoted,
                    TScalarStyle::Literal => ScalarStyle::Literal,
                    TScalarStyle::Folded => ScalarStyle::Folded,
                };
                self.next();

                // Check for line-end comment
                let comment = self.extract_pending_comment();

                Ok(CustomNode::Scalar {
                    value: scalar_value,
                    style: scalar_style,
                    comment,
                    anchor: None,
                })
            }
            Some(TokenType::BlockMappingStart) => {
                self.next();
                self.parse_mapping()
            }
            Some(TokenType::BlockSequenceStart) => {
                self.next();
                self.parse_sequence()
            }
            Some(TokenType::FlowMappingStart) => {
                self.next();
                self.parse_flow_mapping()
            }
            Some(TokenType::FlowSequenceStart) => {
                self.next();
                self.parse_flow_sequence()
            }
            Some(TokenType::Key) => {
                // Mapping without explicit start token
                self.next();
                self.parse_mapping()
            }
            Some(TokenType::Value) => {
                // Sequence item
                self.next();
                self.parse_node()
            }
            Some(TokenType::BlockEntry) => {
                // Sequence entry
                self.next();
                self.parse_node()
            }
            Some(TokenType::BlockEnd) => {
                self.next();
                // Return pending comment as standalone if any
                if let Some(comment) = self.pending_comment.take() {
                    Ok(CustomNode::Null {
                        comment: Some(comment),
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
                Ok(CustomNode::Alias {
                    name: alias_name,
                })
            }
            Some(TokenType::Anchor(name)) => {
                let anchor_name = name.clone();
                self.next();
                // Parse the anchored node
                let mut node = self.parse_node()?;
                // Set anchor on the node
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
                // Skip unknown tokens and try again
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
            match self.peek() {
                Some(TokenType::BlockEnd) => {
                    self.next();
                    break;
                }
                Some(TokenType::Key) => {
                    self.next();
                    let key = self.parse_node()?;
                    if let Some(TokenType::Value) = self.peek() {
                        self.next();
                    }
                    let value = self.parse_node()?;
                    pairs.insert(key, value);
                }
                Some(TokenType::Scalar(style, key_value)) => {
                    // Implicit key in flow mapping
                    let key_str = key_value.clone();
                    let key_style = match style {
                        TScalarStyle::Plain => ScalarStyle::Plain,
                        TScalarStyle::SingleQuoted => ScalarStyle::SingleQuoted,
                        TScalarStyle::DoubleQuoted => ScalarStyle::DoubleQuoted,
                        TScalarStyle::Literal => ScalarStyle::Literal,
                        TScalarStyle::Folded => ScalarStyle::Folded,
                    };
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
                None => break,
                _ => {
                    self.next();
                }
            }
        }

        let comment = self.extract_pending_comment();

        Ok(CustomNode::Mapping {
            pairs,
            comment,
            anchor: None,
        })
    }

    fn parse_flow_mapping(&mut self) -> Result<CustomNode, String> {
        let mut pairs = IndexMap::new();

        loop {
            match self.peek() {
                Some(TokenType::FlowMappingEnd) => {
                    self.next();
                    break;
                }
                Some(TokenType::Key) => {
                    self.next();
                    let key = self.parse_node()?;
                    if let Some(TokenType::Value) = self.peek() {
                        self.next();
                    }
                    let value = self.parse_node()?;
                    pairs.insert(key, value);
                }
                Some(TokenType::Scalar(style, key_value)) => {
                    let key_str = key_value.clone();
                    let key_style = match style {
                        TScalarStyle::Plain => ScalarStyle::Plain,
                        TScalarStyle::SingleQuoted => ScalarStyle::SingleQuoted,
                        TScalarStyle::DoubleQuoted => ScalarStyle::DoubleQuoted,
                        TScalarStyle::Literal => ScalarStyle::Literal,
                        TScalarStyle::Folded => ScalarStyle::Folded,
                    };
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

        let comment = self.extract_pending_comment();

        Ok(CustomNode::Mapping {
            pairs,
            comment,
            anchor: None,
        })
    }

    fn parse_sequence(&mut self) -> Result<CustomNode, String> {
        let mut items = Vec::new();

        loop {
            match self.peek() {
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

        let comment = self.extract_pending_comment();

        Ok(CustomNode::Sequence {
            items,
            comment,
            anchor: None,
        })
    }

    fn parse_flow_sequence(&mut self) -> Result<CustomNode, String> {
        let mut items = Vec::new();

        loop {
            match self.peek() {
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

        let comment = self.extract_pending_comment();

        Ok(CustomNode::Sequence {
            items,
            comment,
            anchor: None,
        })
    }

    fn extract_pending_comment(&mut self) -> Option<Comment> {
        self.pending_comment.take()
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
            let key = CustomNode::Scalar {
                value: "key".to_string(),
                style: ScalarStyle::Plain,
                comment: None,
                anchor: None,
            };
            assert!(pairs.contains_key(&key));
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
