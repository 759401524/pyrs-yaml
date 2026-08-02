pub mod stream;
pub mod yaml;

pub use crate::parser::stream::{parse_stream, StreamEvent, StreamEventType};

/// Error detail for YAML parsing failures, carrying line/column information.
#[derive(Debug, Clone)]
pub struct ParseErrorDetail {
    /// Human-readable error message.
    pub message: String,
    /// Line number (0-indexed) where the error occurred.
    pub line: usize,
    /// Column number (0-indexed) where the error occurred.
    pub col: usize,
}

use crate::ast::{Chomping, Comment, CustomNode, ScalarStyle, Tag};
use crate::parser::yaml::YamlSchema;
use indexmap::IndexMap;
use saphyr_parser::{
    Event, Parser as SaphyrParser, ScalarStyle as SaphyrScalarStyle, Span, SpannedEventReceiver,
};
use std::ops::Range;
use yaml::{
    compute_line_offsets, detect_chomping, extract_anchors, extract_comments, resolve_merge_keys,
    unescape_double_quoted, CommentAnchorTracker, RawAnchor, RawComment,
};

/// 使用 saphyr-parser 解析 YAML 字符串为 `CustomNode` AST。
///
/// # Arguments
/// * `yaml` - YAML 内容字符串。
///
/// # Returns
/// 成功时返回解析后的 AST 根节点，空内容返回 `Null` 节点。
///
/// # Errors
/// 返回 `Err(String)` 格式为 `"YAML parse error: <行号>:<列号>: <消息>"`。
///
/// # Examples
/// ```ignore
/// let ast = pyrs_yaml::parser::parse("key: value").unwrap();
/// ```
///
/// Parse a YAML string into a CustomNode AST using saphyr-parser
pub fn parse(yaml: &str, schema: YamlSchema) -> Result<CustomNode, ParseErrorDetail> {
    parse_with_options(yaml, true, schema, 1000, false)
}

/// 使用选项解析 YAML 字符串。
///
/// # Arguments
/// * `yaml` - YAML 内容字符串。
/// * `resolve_merges` - 是否在解析后解析合并键（`<<`）。
///   合并键会将源映射的键值对合并到目标映射中。
///
/// # Returns
/// 成功时返回解析后的 AST 根节点。
///
/// # Errors
/// 返回 `Err(String)` 格式为 `"YAML parse error: <行号>:<列号>: <消息>"`。
///
/// Parse a YAML string with options
pub fn parse_with_options(
    yaml: &str,
    resolve_merges: bool,
    _schema: YamlSchema,
    max_depth: usize,
    allow_duplicate_keys: bool,
) -> Result<CustomNode, ParseErrorDetail> {
    // Handle empty YAML
    if yaml.trim().is_empty() {
        return Ok(CustomNode::plain_null());
    }

    // Extract comments and anchors from raw text before parsing
    let raw_comments = extract_comments(yaml);
    let raw_anchors = extract_anchors(yaml);

    // Parse YAML using saphyr-parser
    let mut receiver = AstReceiver::new(
        yaml,
        raw_comments,
        raw_anchors,
        max_depth,
        allow_duplicate_keys,
    );
    let mut parser = SaphyrParser::new_from_str(yaml);

    parser
        .load(&mut receiver, true)
        .map_err(|e| ParseErrorDetail {
            message: format!("YAML parse error: {}", e),
            line: 0,
            col: 0,
        })?;

    // Check for duplicate key error
    if let Some(err) = receiver.duplicate_key_error {
        return Err(err);
    }

    if receiver.max_depth_exceeded {
        return Err(ParseErrorDetail {
            message: format!("YAML parse error: max depth exceeded (max={})", max_depth),
            line: 0,
            col: 0,
        });
    }

    // Get the parsed node (handle empty documents)
    let mut node = receiver.result.unwrap_or(CustomNode::plain_null());

    // Resolve merge keys (<<) after parsing (if enabled)
    if resolve_merges {
        resolve_merge_keys(&mut node);
    }

    Ok(node)
}

/// 解析包含多个 YAML 文档的字符串（以 `---` 分隔），支持 `resolve_merges` 选项。
///
/// # Arguments
/// * `yaml` - 包含一个或多个 YAML 文档的字符串。
/// * `resolve_merges` - 是否在解析后解析合并键（`<<`）。
///
/// # Returns
/// `CustomNode` 列表，每个文档对应一个元素。
/// 空内容返回空列表，单文档也返回单元素列表。
///
/// # Errors
/// 返回 `Err(String)`，格式为 `"YAML parse error: document #{doc_index} at <行号>:<列号>: <消息>"`。
///
/// Parse multiple YAML documents from a single string using saphyr document events
pub fn parse_all(yaml: &str, schema: YamlSchema) -> Result<Vec<CustomNode>, ParseErrorDetail> {
    parse_all_with_options(yaml, true, schema, 1000, false)
}

/// 解析包含多个 YAML 文档的字符串，支持选项。
pub fn parse_all_with_options(
    yaml: &str,
    resolve_merges: bool,
    _schema: YamlSchema,
    max_depth: usize,
    allow_duplicate_keys: bool,
) -> Result<Vec<CustomNode>, ParseErrorDetail> {
    // Handle empty YAML
    if yaml.trim().is_empty() {
        return Ok(Vec::new());
    }

    let raw_comments = extract_comments(yaml);
    let raw_anchors = extract_anchors(yaml);

    let mut receiver = AstReceiver::new(
        yaml,
        raw_comments,
        raw_anchors,
        max_depth,
        allow_duplicate_keys,
    );
    let mut parser = SaphyrParser::new_from_str(yaml);

    parser
        .load(&mut receiver, true)
        .map_err(|e| ParseErrorDetail {
            message: format!("YAML parse error: {}", e),
            line: 0,
            col: 0,
        })?;

    // Check for duplicate key error
    if let Some(err) = receiver.duplicate_key_error {
        return Err(err);
    }

    if receiver.max_depth_exceeded {
        return Err(ParseErrorDetail {
            message: format!("YAML parse error: max depth exceeded (max={})", max_depth),
            line: 0,
            col: 0,
        });
    }

    // Collect all documents from receiver
    let docs = receiver.documents;
    if docs.is_empty() {
        // Single document — return as-is
        let mut node = receiver.result.unwrap_or(CustomNode::plain_null());
        if resolve_merges {
            resolve_merge_keys(&mut node);
        }
        return Ok(vec![node]);
    }

    let mut results: Vec<CustomNode> = docs;
    for node in &mut results {
        if resolve_merges {
            resolve_merge_keys(node);
        }
    }
    Ok(results)
}

/// Convert saphyr tag to our Tag format
pub(crate) fn convert_tag(tag: &saphyr_parser::Tag) -> Tag {
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

/// A document is splice-eligible when every block container's direct children
/// sit at the default serializer's indentation (layout parameters read from
/// `SerializeOptions::default()`, not hardcoded). Flow containers are skipped:
/// the gate is doc-wide, so flow docs stay eligible — only flow *regions* fall
/// back (Task 4 P4). CRLF/BOM docs and docs whose layout can't be verified
/// (missing source ranges: merged keys, aliases, programmatic AST) always fall
/// back (P1).
pub(crate) fn check_default_layout(node: &CustomNode, text: &str) -> bool {
    if text.contains('\r') || text.starts_with('\u{FEFF}') {
        return false;
    }
    let def = crate::serializer::SerializeOptions::default();
    let line_offsets = compute_line_offsets(text);
    check_node_layout(node, text, &line_offsets, &def, def.indent_offset)
}

/// Byte offset of the start of the line containing `byte_offset`.
fn line_start_of(line_offsets: &[usize], byte_offset: usize) -> usize {
    match line_offsets.binary_search(&byte_offset) {
        Ok(i) => line_offsets[i],
        Err(0) => 0,
        Err(i) => line_offsets[i - 1],
    }
}

/// Column of `byte_offset` on its line.
fn column_of(line_offsets: &[usize], byte_offset: usize) -> usize {
    byte_offset - line_start_of(line_offsets, byte_offset)
}

/// Recursive layout walk: verifies each block container's direct children sit
/// at `content_indent` (the indent its children must occupy).
fn check_node_layout(
    node: &CustomNode,
    text: &str,
    line_offsets: &[usize],
    def: &crate::serializer::SerializeOptions,
    content_indent: usize,
) -> bool {
    match node {
        CustomNode::Scalar { .. } | CustomNode::Null { .. } | CustomNode::Alias { .. } => true,
        CustomNode::Mapping {
            pairs, flow_style, ..
        } => {
            if *flow_style {
                return true;
            }
            for (key, value) in pairs {
                let Some(key_range) = key.source_range() else {
                    return false; // merged keys / programmatic nodes: not verifiable
                };
                if column_of(line_offsets, key_range.start) != content_indent {
                    return false;
                }
                // Complex block container keys sit after "? " (content_indent + 2)
                if !check_node_layout(key, text, line_offsets, def, content_indent + 2) {
                    return false;
                }
                match value {
                    CustomNode::Mapping {
                        flow_style: false, ..
                    } if !check_node_layout(
                        value,
                        text,
                        line_offsets,
                        def,
                        content_indent + def.indent_mapping,
                    ) =>
                    {
                        return false;
                    }
                    CustomNode::Sequence {
                        flow_style: false, ..
                    } if !check_node_layout(
                        value,
                        text,
                        line_offsets,
                        def,
                        content_indent + def.indent_sequence,
                    ) =>
                    {
                        return false;
                    }
                    _ => {} // flow container, scalar, null, alias: nothing to check
                }
            }
            true
        }
        CustomNode::Sequence {
            items, flow_style, ..
        } => {
            if *flow_style {
                return true;
            }
            for item in items {
                let Some(item_range) = item.source_range() else {
                    return false;
                };
                // The item's line must start with `<content_indent>- `
                let line_start = line_start_of(line_offsets, item_range.start);
                let bytes = text.as_bytes();
                if bytes.get(line_start + content_indent) != Some(&b'-') {
                    return false;
                }
                match bytes.get(line_start + content_indent + 1) {
                    None | Some(b' ' | b'#') => {}
                    _ => return false,
                }
                // Block container items are emitted compact ("- key: value"), so
                // their content sits at content_indent + 2 (after the dash)
                match item {
                    CustomNode::Mapping {
                        flow_style: false, ..
                    }
                    | CustomNode::Sequence {
                        flow_style: false, ..
                    } if !check_node_layout(item, text, line_offsets, def, content_indent + 2) => {
                        return false;
                    }
                    _ => {}
                }
            }
            true
        }
    }
}

/// Build a char-index → byte-offset table. `offsets[char_idx]` is the byte
/// offset of the `char_idx`-th char. saphyr `Marker::index()` is a char index.
fn char_to_byte_offsets(text: &str) -> Vec<usize> {
    let mut out = Vec::with_capacity(text.chars().count() + 1);
    out.push(0);
    for (i, c) in text.char_indices() {
        out.push(i + c.len_utf8());
    }
    out.push(text.len());
    out
}

/// Event receiver that builds CustomNode AST
struct AstReceiver<'a> {
    yaml_text: &'a str,
    /// Pre-computed byte offsets for each line start (O(1) line access)
    line_offsets: Vec<usize>,
    /// Char-index → byte-offset table; `Some` only for non-ASCII input
    char_offsets: Option<Vec<usize>>,
    comment_anchor_tracker: CommentAnchorTracker,
    stack: Vec<ParseState>,
    result: Option<CustomNode>,
    /// Completed documents (for multi-doc parsing)
    documents: Vec<CustomNode>,
    /// Current anchor ID to name mapping
    anchors: std::collections::HashMap<usize, String>,
    /// Pending standalone comment for the next node
    pending_standalone_comment: Option<Comment>,
    /// Maximum allowed nesting depth for mapping/sequence containers
    max_depth: usize,
    /// Set to true when the maximum nesting depth is exceeded
    max_depth_exceeded: bool,
    /// When false, duplicate mapping keys cause a parse error
    allow_duplicate_keys: bool,
    /// Stored duplicate key error (since on_event can't return Result)
    duplicate_key_error: Option<ParseErrorDetail>,
}

#[derive(Debug)]
enum ParseState {
    /// Building a mapping
    Mapping {
        pairs: IndexMap<CustomNode, CustomNode>,
        current_key: Box<Option<CustomNode>>,
        anchor_id: usize,
        tag: Option<Tag>,
        flow_style: bool,
        start_byte: usize,
    },
    /// Building a sequence
    Sequence {
        items: Vec<CustomNode>,
        anchor_id: usize,
        tag: Option<Tag>,
        flow_style: bool,
        start_byte: usize,
    },
}

impl<'a> AstReceiver<'a> {
    fn new(
        yaml_text: &'a str,
        raw_comments: Vec<RawComment>,
        raw_anchors: Vec<RawAnchor>,
        max_depth: usize,
        allow_duplicate_keys: bool,
    ) -> Self {
        // Pre-compute line start offsets for O(1) line access
        Self {
            yaml_text,
            line_offsets: compute_line_offsets(yaml_text),
            char_offsets: (!yaml_text.is_ascii()).then(|| char_to_byte_offsets(yaml_text)),
            comment_anchor_tracker: CommentAnchorTracker::new(raw_comments, raw_anchors),
            stack: Vec::new(),
            result: None,
            documents: Vec::new(),
            anchors: std::collections::HashMap::new(),
            pending_standalone_comment: None,
            max_depth,
            max_depth_exceeded: false,
            allow_duplicate_keys,
            duplicate_key_error: None,
        }
    }

    /// Create a scalar node from value, style, and its byte range in the source
    fn create_scalar(
        &mut self,
        value: &str,
        style: &SaphyrScalarStyle,
        line: usize,
        range: Range<usize>,
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
        let value_end_col = if line < self.line_offsets.len() {
            let start = self.line_offsets[line];
            let end = if line + 1 < self.line_offsets.len() {
                // Exclude the trailing \n from the line
                self.line_offsets[line + 1].saturating_sub(1)
            } else {
                self.yaml_text.len()
            };
            let line_text = &self.yaml_text[start..end];
            // Find where the value ends by looking for the comment
            if let Some(comment_pos) = line_text.find('#') {
                comment_pos
            } else {
                line_text.len()
            }
        } else {
            value.len()
        };

        let inline = self
            .comment_anchor_tracker
            .find_inline_comment(line, value_end_col);

        CustomNode::Scalar {
            value: scalar_value,
            style: scalar_style,
            comment: inline,
            anchor: None,
            tag: None,
            chomping,
            source_range: Some(range),
        }
    }

    /// Convert a saphyr span (char-indexed markers) to a byte range
    fn span_to_byte_range(&self, span: &Span) -> Range<usize> {
        match &self.char_offsets {
            None => span.start.index()..span.end.index(),
            Some(t) => t[span.start.index()]..t[span.end.index()],
        }
    }

    /// Push a node to the current context
    fn push_node(&mut self, node: CustomNode) {
        match self.stack.last_mut() {
            Some(ParseState::Mapping {
                current_key, pairs, ..
            }) => {
                if current_key.is_none() {
                    **current_key = Some(node);
                } else if let Some(key) = current_key.take() {
                    if !self.max_depth_exceeded
                        && !self.allow_duplicate_keys
                        && pairs.contains_key(&key)
                    {
                        let key_str = match &key {
                            CustomNode::Scalar { value, .. } => value.clone(),
                            _ => format!("{:?}", key),
                        };
                        self.duplicate_key_error = Some(ParseErrorDetail {
                            message: format!("duplicate key: {}", key_str),
                            line: 0,
                            col: 0,
                        });
                        return;
                    }
                    pairs.insert(key, node);
                }
            }
            Some(ParseState::Sequence { items, .. }) => {
                items.push(node);
            }
            None => {
                self.result = Some(node);
            }
        }
    }

    /// Detect flow style by checking if the byte at the span start matches
    fn detect_flow_style(&self, span: &Span, expected_byte: u8) -> bool {
        let byte_offset = self.span_to_byte_range(span).start;
        byte_offset < self.yaml_text.len()
            && self.yaml_text.as_bytes()[byte_offset] == expected_byte
    }
}

impl<'a> SpannedEventReceiver<'a> for AstReceiver<'a> {
    /// 处理 saphyr 解析器事件，构建 AST 节点并管理解析栈。
    fn on_event(&mut self, event: Event<'a>, span: Span) {
        match event {
            Event::StreamStart | Event::StreamEnd | Event::DocumentStart(_) => {
                // Ignore these events
            }
            Event::DocumentEnd => {
                // Clone the completed document for multi-doc support
                if let Some(ref doc) = self.result {
                    self.documents.push(doc.clone());
                }
            }
            Event::Scalar(value, style, anchor_id, tag) => {
                let line = span.start.line() - 1; // Convert to 0-indexed
                let range = self.span_to_byte_range(&span);

                // Find standalone comments before this line
                let standalone = self
                    .comment_anchor_tracker
                    .find_standalone_before_line(line);

                let mut node = self.create_scalar(&value, &style, line, range);

                // Attach standalone comment if found
                if let Some(comment) = standalone {
                    if let CustomNode::Scalar { comment: c, .. } = &mut node {
                        // If there's already an inline comment, keep it
                        // Otherwise, use the standalone comment
                        if c.is_none() {
                            *c = Some(comment);
                        }
                    }
                }

                // Handle anchor - use next raw anchor name
                if anchor_id != 0 {
                    if let Some(name) = self.comment_anchor_tracker.next_anchor_name() {
                        self.anchors.insert(anchor_id, name.clone());
                        if let CustomNode::Scalar { anchor, .. } = &mut node {
                            *anchor = Some(name);
                        }
                    }
                }

                // Handle tag
                if let Some(tag) = tag {
                    if let CustomNode::Scalar { tag: t, .. } = &mut node {
                        *t = Some(convert_tag(&tag));
                    }
                }

                self.push_node(node);
            }
            Event::MappingStart(anchor_id, tag) => {
                if self.stack.len() >= self.max_depth {
                    self.max_depth_exceeded = true;
                    return;
                }
                let line = span.start.line() - 1;
                let flow_style = self.detect_flow_style(&span, b'{');
                let start_byte = self.span_to_byte_range(&span).start;

                // Find standalone comments before this line
                let standalone = self
                    .comment_anchor_tracker
                    .find_standalone_before_line(line);

                // Handle anchor - use next raw anchor name
                if anchor_id != 0 {
                    if let Some(name) = self.comment_anchor_tracker.next_anchor_name() {
                        self.anchors.insert(anchor_id, name.clone());
                    }
                }

                // Handle tag
                let tag_obj = tag.map(|t| convert_tag(&t));

                self.stack.push(ParseState::Mapping {
                    pairs: IndexMap::new(),
                    current_key: Box::new(None),
                    anchor_id,
                    tag: tag_obj,
                    flow_style,
                    start_byte,
                });

                // Store standalone comment for when we pop
                if let Some(comment) = standalone {
                    // We'll attach this to the mapping when we pop
                    // For now, store it temporarily
                    self.pending_standalone_comment = Some(comment);
                }
            }
            Event::MappingEnd => {
                if let Some(ParseState::Mapping {
                    pairs,
                    anchor_id,
                    tag,
                    flow_style,
                    start_byte,
                    ..
                }) = self.stack.pop()
                {
                    // Get anchor name from self.anchors
                    let anchor = self.anchors.get(&anchor_id).cloned();

                    // Get pending standalone comment
                    let comment = self.pending_standalone_comment.take();

                    // Block containers span from their start to the last child's
                    // end; flow containers span to the closing token (inclusive).
                    let end = if !flow_style {
                        pairs
                            .iter()
                            .flat_map(|(k, v)| {
                                k.source_range()
                                    .map(|r| r.end)
                                    .into_iter()
                                    .chain(v.source_range().map(|r| r.end))
                            })
                            .max()
                            .unwrap_or_else(|| self.span_to_byte_range(&span).start)
                    } else {
                        self.span_to_byte_range(&span).end
                    };

                    let mapping = CustomNode::Mapping {
                        pairs,
                        comment,
                        anchor,
                        tag,
                        flow_style,
                        source_range: Some(start_byte..end),
                    };
                    self.push_node(mapping);
                }
            }
            Event::SequenceStart(anchor_id, tag) => {
                if self.stack.len() >= self.max_depth {
                    self.max_depth_exceeded = true;
                    return;
                }
                let line = span.start.line() - 1;
                let flow_style = self.detect_flow_style(&span, b'[');
                let start_byte = self.span_to_byte_range(&span).start;

                // Find standalone comments before this line
                let standalone = self
                    .comment_anchor_tracker
                    .find_standalone_before_line(line);

                // Handle anchor - use next raw anchor name
                if anchor_id != 0 {
                    if let Some(name) = self.comment_anchor_tracker.next_anchor_name() {
                        self.anchors.insert(anchor_id, name.clone());
                    }
                }

                // Handle tag
                let tag_obj = tag.map(|t| convert_tag(&t));

                self.stack.push(ParseState::Sequence {
                    items: Vec::new(),
                    anchor_id,
                    tag: tag_obj,
                    flow_style,
                    start_byte,
                });

                // Store standalone comment for when we pop
                if let Some(comment) = standalone {
                    self.pending_standalone_comment = Some(comment);
                }
            }
            Event::SequenceEnd => {
                if let Some(ParseState::Sequence {
                    items,
                    anchor_id,
                    tag,
                    flow_style,
                    start_byte,
                    ..
                }) = self.stack.pop()
                {
                    // Get anchor name from self.anchors
                    let anchor = self.anchors.get(&anchor_id).cloned();

                    // Get pending standalone comment
                    let comment = self.pending_standalone_comment.take();

                    let end = if !flow_style {
                        items
                            .iter()
                            .filter_map(|i| i.source_range().map(|r| r.end))
                            .max()
                            .unwrap_or_else(|| self.span_to_byte_range(&span).start)
                    } else {
                        // Flow containers span to the closing token (inclusive)
                        self.span_to_byte_range(&span).end
                    };

                    let seq = CustomNode::Sequence {
                        items,
                        comment,
                        anchor,
                        tag,
                        flow_style,
                        source_range: Some(start_byte..end),
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

                let node = CustomNode::Alias { name: alias_name };
                self.push_node(node);
            }
            Event::Nothing => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::yaml::YamlSchema;

    #[test]
    fn test_parse_simple_scalar() {
        let result = parse("hello", YamlSchema::Core);
        assert!(result.is_ok());
        if let Ok(CustomNode::Scalar { value, style, .. }) = result {
            assert_eq!(value, "hello");
            assert_eq!(style, ScalarStyle::Plain);
        }
    }

    #[test]
    fn test_parse_mapping() {
        let yaml = "key: value";
        let result = parse(yaml, YamlSchema::Core);
        assert!(result.is_ok());
        if let Ok(CustomNode::Mapping { pairs, .. }) = result {
            assert_eq!(pairs.len(), 1);
        }
    }

    #[test]
    fn test_parse_sequence() {
        let yaml = "- item1\n- item2";
        let result = parse(yaml, YamlSchema::Core);
        assert!(result.is_ok());
        if let Ok(CustomNode::Sequence { items, .. }) = result {
            assert_eq!(items.len(), 2);
        }
    }

    #[test]
    fn test_parse_tag() {
        let yaml = "name: !!str John";
        let result = parse(yaml, YamlSchema::Core);
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
        let result = parse(yaml, YamlSchema::Core);
        assert!(result.is_ok());
        if let Ok(CustomNode::Mapping { pairs, .. }) = result {
            assert_eq!(pairs.len(), 1);
        }
    }

    #[test]
    fn test_parse_empty() {
        let result = parse("", YamlSchema::Core);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), CustomNode::Null { .. }));
    }

    #[test]
    fn test_parse_anchor() {
        let yaml = "defaults: &defaults\n  timeout: 30";
        let result = parse(yaml, YamlSchema::Core);
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

    #[test]
    fn test_scalar_byte_range() {
        let node = parse_with_options("key: value\n", true, YamlSchema::Core, 1000, false).unwrap();
        let CustomNode::Mapping { pairs, .. } = node else {
            panic!()
        };
        let key = pairs.keys().next().unwrap();
        let val = pairs.values().next().unwrap();
        assert_eq!(key.source_range(), Some(&(0usize..3))); // "key"
        assert_eq!(val.source_range(), Some(&(5usize..10))); // "value"
    }

    #[test]
    fn test_mapping_range_spans_children() {
        let node = parse_with_options("a:\n  b: 1\n", true, YamlSchema::Core, 1000, false).unwrap();
        let CustomNode::Mapping {
            pairs,
            source_range,
            ..
        } = node
        else {
            panic!()
        };
        assert_eq!(source_range, Some(0usize..9)); // up to the last child ("1") end
        let inner = pairs.values().next().unwrap();
        let CustomNode::Mapping { pairs, .. } = inner else {
            panic!()
        };
        assert_eq!(
            pairs.values().next().unwrap().source_range(),
            Some(&(8usize..9))
        );
    }

    #[test]
    fn test_non_ascii_byte_range() {
        // '值' is 3 bytes; char index 5 != byte offset 7
        let node = parse_with_options("key: 值\n", true, YamlSchema::Core, 1000, false).unwrap();
        let CustomNode::Mapping { pairs, .. } = node else {
            panic!()
        };
        assert_eq!(pairs.values().next().unwrap().source_range(), Some(&(5..8)));
    }

    #[test]
    fn test_flow_mapping_range_includes_closing_token() {
        let node = parse_with_options("{a: 1}\n", true, YamlSchema::Core, 1000, false).unwrap();
        let CustomNode::Mapping { source_range, .. } = node else {
            panic!()
        };
        assert_eq!(source_range, Some(0usize..6)); // covers "{a: 1}" incl. '}'
    }

    #[test]
    fn test_splice_gate_default_layout_ok() {
        let node = parse_with_options("a:\n  b: 1\n", true, YamlSchema::Core, 1000, false).unwrap();
        assert!(check_default_layout(&node, "a:\n  b: 1\n"));
    }

    #[test]
    fn test_splice_gate_non_default_indent_rejected() {
        let node =
            parse_with_options("a:\n    b: 1\n", true, YamlSchema::Core, 1000, false).unwrap();
        assert!(!check_default_layout(&node, "a:\n    b: 1\n")); // 4-space indent violates indent_mapping=2
    }

    #[test]
    fn test_splice_gate_crlf_rejected() {
        let node =
            parse_with_options("a: 1\r\nb: 2\r\n", true, YamlSchema::Core, 1000, false).unwrap();
        assert!(!check_default_layout(&node, "a: 1\r\nb: 2\r\n")); // CRLF -> fallback (P1)
    }

    #[test]
    fn test_splice_gate_bom_rejected() {
        let node =
            parse_with_options("\u{FEFF}a: 1\n", true, YamlSchema::Core, 1000, false).unwrap();
        assert!(!check_default_layout(&node, "\u{FEFF}a: 1\n")); // BOM -> fallback (P1)
    }

    #[test]
    fn test_splice_gate_nested_layout_ok() {
        // nested mapping (indent_mapping) + sequence value (indent_sequence)
        let node = parse_with_options(
            "a:\n  b:\n    c: 1\nd:\n  - 1\n",
            true,
            YamlSchema::Core,
            1000,
            false,
        )
        .unwrap();
        assert!(check_default_layout(
            &node,
            "a:\n  b:\n    c: 1\nd:\n  - 1\n"
        ));
    }

    #[test]
    fn test_splice_gate_compact_item_layout_ok() {
        let node =
            parse_with_options("- a: 1\n  b: 2\n", true, YamlSchema::Core, 1000, false).unwrap();
        assert!(check_default_layout(&node, "- a: 1\n  b: 2\n"));
    }

    #[test]
    fn test_splice_gate_sequence_bad_indent_rejected() {
        let node = parse_with_options("a:\n   - 1\n", true, YamlSchema::Core, 1000, false).unwrap();
        assert!(!check_default_layout(&node, "a:\n   - 1\n")); // dash at col 3 instead of indent_sequence=2
    }

    #[test]
    fn test_splice_gate_flow_doc_eligible() {
        let node = parse_with_options("{a: 1}\n", true, YamlSchema::Core, 1000, false).unwrap();
        assert!(check_default_layout(&node, "{a: 1}\n")); // flow docs stay eligible (only flow *regions* fall back)
    }
}
