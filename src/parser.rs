use crate::ast::{Chomping, Comment, CustomNode, ScalarStyle, Tag};
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

/// Detect chomping indicator from raw YAML text
/// The chomping indicator appears on the line before the block scalar content
/// Looks for |- or |+ or >- or >+ patterns
fn detect_chomping(yaml: &str, content_line: usize) -> Chomping {
    let lines: Vec<&str> = yaml.lines().collect();

    // Look at the line before the content for the block scalar indicator
    // The indicator could be on the same line as the key or on a previous line
    for check_line in (0..=content_line).rev() {
        if check_line >= lines.len() {
            continue;
        }
        let line_text = lines[check_line];

        // Look for | or > followed by - or +
        for (i, ch) in line_text.char_indices() {
            if ch == '|' || ch == '>' {
                let remaining = &line_text[i + 1..];
                if remaining.starts_with('-') {
                    return Chomping::Strip;
                } else if remaining.starts_with('+') {
                    return Chomping::Keep;
                }
                // Found the indicator without chomping, stop looking
                if remaining.is_empty() || remaining.starts_with(|c: char| c.is_whitespace()) {
                    return Chomping::Clip;
                }
            }
        }
    }

    Chomping::Clip
}

/// Unescape a double-quoted YAML string
fn unescape_double_quoted(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('/') => result.push('/'),
                Some('0') => result.push('\0'),
                Some('a') => result.push('\x07'),
                Some('b') => result.push('\x08'),
                Some('f') => result.push('\x0C'),
                Some('e') => result.push('\x1B'),
                Some(' ') => result.push(' '),
                Some('\n') => {
                    // Line continuation - skip whitespace
                    while let Some(&next) = chars.clone().peekable().peek() {
                        if next.is_whitespace() {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
                Some('u') => {
                    // Unicode escape: \uXXXX
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(code_point) = u32::from_str_radix(&hex, 16) {
                        if let Some(c) = char::from_u32(code_point) {
                            result.push(c);
                        }
                    }
                }
                Some('U') => {
                    // Unicode escape: \UXXXXXXXX
                    let hex: String = chars.by_ref().take(8).collect();
                    if let Ok(code_point) = u32::from_str_radix(&hex, 16) {
                        if let Some(c) = char::from_u32(code_point) {
                            result.push(c);
                        }
                    }
                }
                Some('x') => {
                    // Hex escape: \xXX
                    let hex: String = chars.by_ref().take(2).collect();
                    if let Ok(code_point) = u8::from_str_radix(&hex, 16) {
                        result.push(code_point as char);
                    }
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// YAML 1.2 type resolution for plain scalars
#[derive(Debug, Clone, PartialEq)]
pub enum YamlType {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value (decimal, octal, hex)
    Int(i64),
    /// Float value (including infinity and NaN)
    Float(f64),
    /// String value (default)
    Str(String),
}

/// Resolve a plain scalar to its YAML 1.2 type
pub fn resolve_yaml_type(value: &str) -> YamlType {
    let trimmed = value.trim();

    // Null values (YAML 1.2)
    if trimmed.is_empty() || trimmed == "null" || trimmed == "Null" || trimmed == "NULL" || trimmed == "~" {
        return YamlType::Null;
    }

    // Boolean values (YAML 1.2 - only true/false are valid)
    if trimmed == "true" || trimmed == "True" || trimmed == "TRUE" {
        return YamlType::Bool(true);
    }
    if trimmed == "false" || trimmed == "False" || trimmed == "FALSE" {
        return YamlType::Bool(false);
    }

    // Infinity
    if trimmed == ".inf" || trimmed == ".Inf" || trimmed == ".INF" || trimmed == "inf" || trimmed == "Inf" || trimmed == "INF" {
        return YamlType::Float(f64::INFINITY);
    }
    if trimmed == "-.inf" || trimmed == "-.Inf" || trimmed == "-.INF" || trimmed == "-inf" || trimmed == "-Inf" || trimmed == "-INF" {
        return YamlType::Float(f64::NEG_INFINITY);
    }

    // NaN
    if trimmed == ".nan" || trimmed == ".NaN" || trimmed == ".NAN" || trimmed == "nan" || trimmed == "NaN" || trimmed == "NAN" {
        return YamlType::Float(f64::NAN);
    }

    // Octal integer (0o prefix)
    if trimmed.starts_with("0o") || trimmed.starts_with("0O") {
        let octal_str = &trimmed[2..];
        if let Ok(val) = i64::from_str_radix(octal_str, 8) {
            return YamlType::Int(val);
        }
    }

    // Hexadecimal integer (0x prefix)
    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        let hex_str = &trimmed[2..];
        if let Ok(val) = i64::from_str_radix(hex_str, 16) {
            return YamlType::Int(val);
        }
    }

    // Float with decimal point or exponent
    if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
        if let Ok(val) = trimmed.parse::<f64>() {
            return YamlType::Float(val);
        }
    }

    // Decimal integer
    if let Ok(val) = trimmed.parse::<i64>() {
        return YamlType::Int(val);
    }

    // Default: string
    YamlType::Str(value.to_string())
}

/// Format a YAML 1.2 type back to string for serialization
fn format_yaml_type(ty: &YamlType) -> String {
    match ty {
        YamlType::Null => "null".to_string(),
        YamlType::Bool(true) => "true".to_string(),
        YamlType::Bool(false) => "false".to_string(),
        YamlType::Int(val) => val.to_string(),
        YamlType::Float(val) => {
            if val.is_infinite() {
                if *val > 0.0 { ".inf".to_string() } else { "-.inf".to_string() }
            } else if val.is_nan() {
                ".nan".to_string()
            } else if *val == val.floor() && val.abs() < 1e15 {
                // Format as integer if it's a whole number
                format!("{}", *val as i64)
            } else {
                val.to_string()
            }
        }
        YamlType::Str(s) => s.clone(),
    }
}

/// Find an inline comment on the same line at a column after `after_col`
fn find_inline_comment(
    comments: &[RawComment],
    start_idx: &mut usize,
    line: usize,
    after_col: usize,
) -> Option<Comment> {
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
fn find_standalone_comment_before(
    comments: &[RawComment],
    start_idx: &mut usize,
    before_line: usize,
) -> Option<Comment> {
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
    let mut parser = TokenParser::new(tokens_with_pos, raw_comments, yaml.to_string());
    parser.parse_document()
}

struct TokenParser {
    tokens: Vec<(Marker, TokenType)>,
    pos: usize,
    raw_comments: Vec<RawComment>,
    comment_idx: usize,
    yaml_text: String,
}

impl TokenParser {
    fn new(tokens: Vec<(Marker, TokenType)>, raw_comments: Vec<RawComment>, yaml_text: String) -> Self {
        Self {
            tokens,
            pos: 0,
            raw_comments,
            comment_idx: 0,
            yaml_text,
        }
    }

    fn peek(&self) -> Option<&TokenType> {
        self.tokens.get(self.pos).map(|(_, t)| t)
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

        // Check for tag and anchor in any order
        let mut pending_tag = None;
        let mut pending_anchor = None;

        // YAML allows anchor and tag in any order before the node
        loop {
            match self.peek().cloned() {
                Some(TokenType::Tag(handle, suffix)) => {
                    self.next();
                    pending_tag = Some(Tag { handle, suffix });
                }
                Some(TokenType::Anchor(name)) => {
                    self.next();
                    pending_anchor = Some(name);
                }
                _ => break,
            }
        }

        match self.peek().cloned() {
            Some(TokenType::Scalar(style, value)) => {
                let scalar_style = Self::map_style(&style);

                // Unescape double-quoted strings
                let scalar_value = if matches!(style, TScalarStyle::DoubleQuoted) {
                    unescape_double_quoted(&value)
                } else {
                    value.clone()
                };

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

                // Detect chomping for block scalars (Literal or Folded)
                let chomping = if matches!(scalar_style, ScalarStyle::Literal | ScalarStyle::Folded) {
                    detect_chomping(&self.yaml_text, line)
                } else {
                    Chomping::Clip
                };

                Ok(CustomNode::Scalar {
                    value: scalar_value,
                    style: scalar_style,
                    comment,
                    anchor: pending_anchor,
                    tag: pending_tag,
                    chomping,
                })
            }
            Some(TokenType::BlockMappingStart) => {
                self.next();
                let mut mapping = self.parse_mapping()?;
                if let Some(s) = standalone {
                    mapping.set_comment(s);
                }
                if let Some(t) = pending_tag {
                    mapping.set_tag(t);
                }
                if let Some(a) = pending_anchor {
                    if let CustomNode::Mapping { anchor, .. } = &mut mapping {
                        *anchor = Some(a);
                    }
                }
                Ok(mapping)
            }
            Some(TokenType::BlockSequenceStart) => {
                self.next();
                let mut seq = self.parse_sequence()?;
                if let Some(s) = standalone {
                    seq.set_comment(s);
                }
                if let Some(t) = pending_tag {
                    seq.set_tag(t);
                }
                if let Some(a) = pending_anchor {
                    if let CustomNode::Sequence { anchor, .. } = &mut seq {
                        *anchor = Some(a);
                    }
                }
                Ok(seq)
            }
            Some(TokenType::FlowMappingStart) => {
                self.next();
                let mut mapping = self.parse_flow_mapping()?;
                if let Some(s) = standalone {
                    mapping.set_comment(s);
                }
                if let Some(t) = pending_tag {
                    mapping.set_tag(t);
                }
                if let Some(a) = pending_anchor {
                    if let CustomNode::Mapping { anchor, .. } = &mut mapping {
                        *anchor = Some(a);
                    }
                }
                Ok(mapping)
            }
            Some(TokenType::FlowSequenceStart) => {
                self.next();
                let mut seq = self.parse_flow_sequence()?;
                if let Some(s) = standalone {
                    seq.set_comment(s);
                }
                if let Some(t) = pending_tag {
                    seq.set_tag(t);
                }
                if let Some(a) = pending_anchor {
                    if let CustomNode::Sequence { anchor, .. } = &mut seq {
                        *anchor = Some(a);
                    }
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
                Ok(CustomNode::Null {
                    comment: standalone,
                    anchor: pending_anchor,
                    tag: pending_tag,
                })
            }
            Some(TokenType::Alias(name)) => {
                let alias_name = name.clone();
                self.next();
                Ok(CustomNode::Alias {
                    name: alias_name,
                })
            }
            None => Ok(CustomNode::Null {
                comment: standalone,
                anchor: pending_anchor,
                tag: pending_tag,
            }),
            _ => {
                self.next();
                if self.peek().is_some() {
                    self.parse_node()
                } else {
                    Ok(CustomNode::Null {
                        comment: standalone,
                        anchor: pending_anchor,
                        tag: pending_tag,
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
                        tag: None,
                        chomping: Chomping::Clip,
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
            tag: None,
        })
    }

    /// Parse a key in a key-value pair (handles Key token + scalar or complex key)
    fn parse_key_for_pair(&mut self) -> Result<CustomNode, String> {
        // Collect standalone comments before this key
        let (line, _col) = self.marker_line_col(0);
        let standalone = find_standalone_comment_before(
            &self.raw_comments,
            &mut self.comment_idx,
            line,
        );

        // Check for tag and anchor before key in any order
        let mut pending_tag = None;
        let mut pending_anchor = None;

        loop {
            match self.peek().cloned() {
                Some(TokenType::Tag(handle, suffix)) => {
                    self.next();
                    pending_tag = Some(Tag { handle, suffix });
                }
                Some(TokenType::Anchor(name)) => {
                    self.next();
                    pending_anchor = Some(name);
                }
                _ => break,
            }
        }

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
                    anchor: pending_anchor,
                    tag: pending_tag,
                    chomping: Chomping::Clip,
                })
            }
            Some(TokenType::BlockMappingStart) => {
                self.next();
                let mut mapping = self.parse_mapping()?;
                if let Some(s) = standalone {
                    mapping.set_comment(s);
                }
                if let Some(t) = pending_tag {
                    mapping.set_tag(t);
                }
                if let Some(a) = pending_anchor {
                    if let CustomNode::Mapping { anchor, .. } = &mut mapping {
                        *anchor = Some(a);
                    }
                }
                Ok(mapping)
            }
            Some(TokenType::BlockSequenceStart) => {
                self.next();
                let mut seq = self.parse_sequence()?;
                if let Some(s) = standalone {
                    seq.set_comment(s);
                }
                if let Some(t) = pending_tag {
                    seq.set_tag(t);
                }
                if let Some(a) = pending_anchor {
                    if let CustomNode::Sequence { anchor, .. } = &mut seq {
                        *anchor = Some(a);
                    }
                }
                Ok(seq)
            }
            Some(TokenType::FlowMappingStart) => {
                self.next();
                let mut mapping = self.parse_flow_mapping()?;
                if let Some(s) = standalone {
                    mapping.set_comment(s);
                }
                if let Some(t) = pending_tag {
                    mapping.set_tag(t);
                }
                if let Some(a) = pending_anchor {
                    if let CustomNode::Mapping { anchor, .. } = &mut mapping {
                        *anchor = Some(a);
                    }
                }
                Ok(mapping)
            }
            Some(TokenType::FlowSequenceStart) => {
                self.next();
                let mut seq = self.parse_flow_sequence()?;
                if let Some(s) = standalone {
                    seq.set_comment(s);
                }
                if let Some(t) = pending_tag {
                    seq.set_tag(t);
                }
                if let Some(a) = pending_anchor {
                    if let CustomNode::Sequence { anchor, .. } = &mut seq {
                        *anchor = Some(a);
                    }
                }
                Ok(seq)
            }
            _ => {
                // Complex key - parse as node
                let mut node = self.parse_node()?;
                if let Some(s) = standalone {
                    node.set_comment(s);
                }
                if let Some(t) = pending_tag {
                    node.set_tag(t);
                }
                if let Some(a) = pending_anchor {
                    match &mut node {
                        CustomNode::Scalar { anchor, .. }
                        | CustomNode::Mapping { anchor, .. }
                        | CustomNode::Sequence { anchor, .. }
                        | CustomNode::Null { anchor, .. } => {
                            *anchor = Some(a);
                        }
                        _ => {}
                    }
                }
                Ok(node)
            }
        }
    }

    /// Parse a value in a key-value pair (handles Value token + scalar)
    fn parse_value_for_pair(&mut self) -> Result<CustomNode, String> {
        if let Some(TokenType::Value) = self.peek() {
            // Get position of Value token
            let (_val_line, _val_col) = self.marker_line_col(0);
            self.next();

            // Get position of the value scalar
            let (val_scalar_line, val_scalar_col) = self.marker_line_col(0);

            // Parse the value node
            let mut node = self.parse_node()?;

            // If the value is a scalar, look for inline comment after it
            if let CustomNode::Scalar {
                value, comment, ..
            } = &mut node
            {
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
                        tag: None,
                        chomping: Chomping::Clip,
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
            tag: None,
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
            tag: None,
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
            tag: None,
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

    #[test]
    fn test_parse_tag() {
        let yaml = "key: !!str value";
        let result = parse(yaml);
        assert!(result.is_ok());
        if let Ok(CustomNode::Mapping { pairs, .. }) = result {
            for (k, v) in pairs {
                if let CustomNode::Scalar { value, .. } = k {
                    if value == "key" {
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
}
