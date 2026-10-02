pub mod stream;
pub mod yaml;

pub use crate::error::{DepthError, ParseError};
pub use crate::parser::stream::{
    StreamEvent, StreamEventType, parse_stream, parse_stream_with_options,
};

use crate::ast::{Comment, CustomNode, NodeMeta, ScalarStyle, Tag};
use crate::parser::yaml::Schema;
use granit_parser::{
    Event, Parser as SaphyrParser, ScalarStyle as SaphyrScalarStyle, Span, SpannedEventReceiver,
};
use indexmap::IndexMap;
use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;
use yaml::{
    BlockHeader, RawAnchor, compute_line_offsets, detect_block_header, extract_anchors,
    resolve_merge_keys,
};

/// Return true if a mapping key is a null/empty key (`~`, empty, or null).
///
/// The yaml-test-suite allows duplicate null keys (e.g. `: a\n: b`, see 2JQS),
/// so duplicate-key detection must not reject them.
fn is_null_key(key: &CustomNode) -> bool {
    match key {
        CustomNode::Null { .. } => true,
        CustomNode::Scalar { value, .. } => {
            value.is_empty() || value.as_ref() == "~" || value.eq_ignore_ascii_case("null")
        }
        _ => false,
    }
}

/// 使用 granit-parser 解析 YAML 字符串为 `CustomNode` AST。
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
/// ```rust
/// use pyrs_yaml_core::parser::parse;
/// use pyrs_yaml_core::parser::yaml::Schema;
/// let ast = parse("key: value", Schema::Core).unwrap();
/// ```
///
/// Parse a YAML string into a CustomNode AST using granit-parser
pub fn parse(yaml: &str, schema: impl Into<Schema>) -> Result<CustomNode, ParseError> {
    parse_with_options(yaml, true, schema, 1000, false)
}

/// Whether a granit comment placement denotes a standalone (own-line)
/// comment, as opposed to an inline trailing one. Shared by the AST and
/// stream receivers so the placement taxonomy lives in exactly one place.
pub(crate) fn is_standalone_placement(p: &granit_parser::Placement) -> bool {
    matches!(
        p,
        granit_parser::Placement::Above
            | granit_parser::Placement::Free
            | granit_parser::Placement::Last
    )
}

/// Drive `AstReceiver` over `yaml` with the shared error contract:
/// parse failure, duplicate-key rejection and max-depth rejection are
/// all mapped here so the single-document and multi-document entry
/// points can never drift. The receiver is returned so callers can
/// pick `result` (single) or `documents` (stream) as they need.
fn load_ast<'a>(
    yaml: &'a str,
    max_depth: usize,
    allow_duplicate_keys: bool,
    collect_documents: bool,
) -> Result<AstReceiver<'a>, ParseError> {
    let raw_anchors = extract_anchors(yaml);
    let mut receiver = AstReceiver::new(yaml, raw_anchors, max_depth, allow_duplicate_keys);
    receiver.collect_documents = collect_documents;
    let mut parser = SaphyrParser::new_from_str(yaml);
    parser
        .load(&mut receiver, true)
        .map_err(|e| ParseError::Syntax {
            message: format!("YAML parse error: {}", e),
            line: 0,
            col: 0,
        })?;
    if let Some(err) = receiver.duplicate_key_error {
        return Err(err);
    }
    if let Some(err) = receiver.flow_indent_error {
        return Err(err);
    }
    if receiver.max_depth_exceeded {
        return Err(ParseError::MaxDepthExceeded(DepthError(max_depth)));
    }
    Ok(receiver)
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
    schema: impl Into<Schema>,
    max_depth: usize,
    allow_duplicate_keys: bool,
) -> Result<CustomNode, ParseError> {
    let _schema = schema.into();
    // Handle empty YAML
    if yaml.trim().is_empty() {
        return Ok(CustomNode::plain_null());
    }

    let receiver = load_ast(yaml, max_depth, allow_duplicate_keys, false)?;

    // Get the parsed node (handle empty documents)
    let mut node = receiver.result.unwrap_or(CustomNode::plain_null());

    // Resolve merge keys (<<) after parsing (if enabled and any were detected)
    if resolve_merges && receiver.has_merge_key {
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
pub fn parse_all(yaml: &str, schema: impl Into<Schema>) -> Result<Vec<CustomNode>, ParseError> {
    parse_all_with_options(yaml, true, schema, 1000, false)
}

/// 解析包含多个 YAML 文档的字符串，支持选项。
pub fn parse_all_with_options(
    yaml: &str,
    resolve_merges: bool,
    _schema: impl Into<Schema>,
    max_depth: usize,
    allow_duplicate_keys: bool,
) -> Result<Vec<CustomNode>, ParseError> {
    let _schema = _schema.into();
    // Handle empty YAML
    if yaml.trim().is_empty() {
        return Ok(Vec::new());
    }

    let receiver = load_ast(yaml, max_depth, allow_duplicate_keys, true)?;

    // Collect all documents from receiver
    let docs = receiver.documents;
    if docs.is_empty() {
        // Single document — return as-is
        let mut node = receiver.result.unwrap_or(CustomNode::plain_null());
        if resolve_merges && receiver.has_merge_key {
            resolve_merge_keys(&mut node);
        }
        return Ok(vec![node]);
    }

    let mut results: Vec<CustomNode> = docs;
    if resolve_merges && receiver.has_merge_key {
        for node in &mut results {
            resolve_merge_keys(node);
        }
    }
    Ok(results)
}

/// Convert saphyr tag to our Tag format
pub(crate) fn convert_tag(tag: &granit_parser::Tag) -> Tag {
    // granit-parser uses full URIs like "tag:yaml.org,2002:str"
    // We need to convert back to short form like "!!str"
    let handle = tag.handle();
    let suffix = tag.suffix();

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
pub fn check_default_layout(node: &CustomNode, text: &str) -> bool {
    if text.contains('\r') || text.starts_with('\u{FEFF}') {
        return false;
    }
    let def = crate::serializer::SerializeOptions::default();
    let line_offsets = compute_line_offsets(text);
    let mut cursor = LineCursor::new(&line_offsets);
    check_node_layout(node, text, &mut cursor, &def, def.indent_offset)
}

/// Linear line-cursor over precomputed line offsets. Children of a container
/// appear in source order, so their offsets are monotonically increasing —
/// a single forward pass replaces per-node binary search.
pub struct LineCursor<'a> {
    offsets: &'a [usize],
    idx: usize,
}

impl<'a> LineCursor<'a> {
    fn new(offsets: &'a [usize]) -> Self {
        Self { offsets, idx: 0 }
    }

    /// Advance to the line containing `byte_offset`; returns its start.
    fn line_start_of(&mut self, byte_offset: usize) -> usize {
        while self.idx + 1 < self.offsets.len() && self.offsets[self.idx + 1] <= byte_offset {
            self.idx += 1;
        }
        self.offsets[self.idx]
    }

    /// Column of `byte_offset` on its line.
    fn column_of(&mut self, byte_offset: usize) -> usize {
        byte_offset - self.line_start_of(byte_offset)
    }
}

/// Recursive layout walk: verifies each block container's direct children sit
/// at `content_indent` (the indent its children must occupy).
pub fn check_node_layout(
    node: &CustomNode,
    text: &str,
    cursor: &mut LineCursor<'_>,
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
                if cursor.column_of(key_range.start) != content_indent {
                    return false;
                }
                // Complex block container keys sit after "? " (content_indent + 2)
                if !check_node_layout(key, text, cursor, def, content_indent + 2) {
                    return false;
                }
                match value {
                    CustomNode::Mapping {
                        flow_style: false, ..
                    } if !check_node_layout(
                        value,
                        text,
                        cursor,
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
                        cursor,
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
                let line_start = cursor.line_start_of(item_range.start);
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
                    } if !check_node_layout(item, text, cursor, def, content_indent + 2) => {
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
    char_offsets: Option<Vec<usize>>,
    /// Anchor names extracted from raw text (indexed by anchor_id)
    anchor_names: Vec<String>,
    /// Current index into anchor_names
    anchor_name_idx: usize,
    /// Standalone/inline comment slot per in-progress container, parallel to
    /// `stack`. A single shared slot was clobbered by nested container starts
    /// (a block mapping's header comment vanished when its first value was
    /// itself a container).
    comment_stack: Vec<Option<Comment>>,
    stack: Vec<ParseState>,
    result: Option<CustomNode>,
    /// Whether to collect completed documents for multi-doc parsing. When
    /// false (single-document parse), `DocumentEnd` skips the full AST clone.
    collect_documents: bool,
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
    duplicate_key_error: Option<ParseError>,
    /// Whether any `<<` merge key was detected during parsing
    has_merge_key: bool,
    /// Set when a multi-line flow collection has a continuation line indented
    /// no further than its enclosing block collection (yaml-test-suite `9C9N`).
    /// granit-parser 1.3 wrongly accepts these; this in-tree guard restores the
    /// strictness the v0.11.5 audit pinned. Stored because `on_event` cannot
    /// return a `Result`, mirroring `duplicate_key_error`.
    flow_indent_error: Option<ParseError>,
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
        /// 0-indexed source column of the container start, cached so the flow
        /// guard can read block indentation in O(1).
        start_col: usize,
        /// `(flow_open_line, block_ancestor_col)` for a multi-line flow
        /// collection used as a block value; `None` for block containers and
        /// for flows with no enclosing block collection (document-root flows).
        guard: Option<(usize, usize)>,
    },
    /// Building a sequence
    Sequence {
        items: Vec<CustomNode>,
        anchor_id: usize,
        tag: Option<Tag>,
        flow_style: bool,
        start_byte: usize,
        start_col: usize,
        guard: Option<(usize, usize)>,
    },
}

/// Extract the flow-indent guard of a parse state, or `None` for block
/// containers (a guard only applies while inside a flow collection).
fn container_guard(state: &ParseState) -> Option<(usize, usize)> {
    match state {
        ParseState::Mapping {
            flow_style, guard, ..
        }
        | ParseState::Sequence {
            flow_style, guard, ..
        } => {
            if *flow_style {
                *guard
            } else {
                None
            }
        }
    }
}

impl<'a> AstReceiver<'a> {
    fn new(
        yaml_text: &'a str,
        raw_anchors: Vec<RawAnchor>,
        max_depth: usize,
        allow_duplicate_keys: bool,
    ) -> Self {
        let is_ascii = yaml_text.is_ascii();
        Self {
            yaml_text,
            char_offsets: (!is_ascii).then(|| char_to_byte_offsets(yaml_text)),
            stack: Vec::new(),
            result: None,
            collect_documents: true,
            documents: Vec::new(),
            anchors: std::collections::HashMap::new(),
            anchor_names: raw_anchors.iter().map(|a| a.name.clone()).collect(),
            anchor_name_idx: 0,
            comment_stack: Vec::new(),
            pending_standalone_comment: None,
            max_depth,
            max_depth_exceeded: false,
            allow_duplicate_keys,
            duplicate_key_error: None,
            has_merge_key: false,
            flow_indent_error: None,
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

        // Detect the block scalar header (chomping + explicit indent indicator)
        // from the source text. Both indicators live only in the header, so a
        // single scan recovers them together.
        let block_header = if matches!(scalar_style, ScalarStyle::Literal | ScalarStyle::Folded) {
            detect_block_header(self.yaml_text, line)
        } else {
            BlockHeader::default()
        };
        let chomping = block_header.chomping;

        // No unescaping here: granit delivers the *decoded* double-quoted
        // value, and running our own unescape on top was a second decode —
        // `"\\n"` (raw backslash-n, meaning the two characters `\` `n`)
        // silently collapsed to LF (libFuzzer `yaml_roundtrip`). The same
        // double decode turned the fuzz repro's `\\f` into FF.
        let scalar_value = Arc::from(value);

        // Inline comment scanning now done by granit-parser natively via Event::Comment
        CustomNode::Scalar {
            value: scalar_value,
            style: scalar_style,
            chomping,
            block_indent: block_header.indent,
            meta: NodeMeta {
                source_range: Some(range),
                ..Default::default()
            },
        }
    }

    /// Attach an inline comment to the most recently created scalar node, or
    /// to the currently-open empty container (a `{}`/`[]` has no child to
    /// attach to, so the comment belongs to the container itself).
    fn attach_inline_comment(&mut self, text: Arc<str>) {
        let comment = Comment {
            text,
            standalone: false,
        };
        // Check the top of the stack for a scalar to attach to
        if let Some(top) = self.stack.last_mut() {
            match top {
                ParseState::Mapping {
                    current_key, pairs, ..
                } => {
                    // Attach to the last value if complete, or the current key
                    let target = if current_key.is_none() {
                        pairs.iter_mut().last().map(|(_, v)| v)
                    } else {
                        current_key.as_mut().as_mut()
                    };
                    if target.is_some() {
                        Self::set_scalar_comment(target, comment);
                    } else if let Some(slot) = self.comment_stack.last_mut() {
                        // Empty mapping `{}` — stash in the container's own slot
                        // (on_mapping_end attaches it to the node).
                        *slot = Some(comment);
                    }
                }
                ParseState::Sequence { items, .. } => {
                    if !items.is_empty() {
                        Self::set_scalar_comment(items.last_mut(), comment);
                    } else if let Some(slot) = self.comment_stack.last_mut() {
                        // Empty sequence `[]` — stash in the container's slot.
                        *slot = Some(comment);
                    }
                }
            }
        } else if let Some(result) = &mut self.result {
            Self::set_scalar_comment(Some(result), comment);
        }
    }

    /// Set the comment on a node if it's a Scalar with no existing comment.
    fn set_scalar_comment(node: Option<&mut CustomNode>, comment: Comment) {
        if let Some(node) = node
            && node.comment().is_none()
            && !matches!(node, CustomNode::Alias { .. })
        {
            node.set_comment(comment);
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
                    if self.max_depth_exceeded || self.allow_duplicate_keys || is_null_key(&key) {
                        pairs.insert(key, node);
                    } else {
                        use indexmap::map::Entry;
                        match pairs.entry(key) {
                            Entry::Vacant(v) => {
                                v.insert(node);
                            }
                            Entry::Occupied(mut o) => {
                                let key_str = match o.key() {
                                    CustomNode::Scalar { value, .. } => value.to_string(),
                                    k => format!("{:?}", k),
                                };
                                o.insert(node);
                                self.duplicate_key_error = Some(ParseError::DuplicateKey(key_str));
                            }
                        };
                    }
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

    /// Column of the nearest enclosing *block* collection (skipping flow ones),
    /// read from its cached `start_col` so it is O(1). `None` when the container
    /// sits directly under the document root, i.e. a document-root flow
    /// collection that carries no block indentation to violate.
    fn nearest_block_ancestor_col(&self) -> Option<usize> {
        self.stack.iter().rev().find_map(|state| match state {
            ParseState::Mapping {
                flow_style: false,
                start_col,
                ..
            }
            | ParseState::Sequence {
                flow_style: false,
                start_col,
                ..
            } => Some(*start_col),
            _ => None,
        })
    }

    /// Guard for a freshly-opened flow collection: `(open_line, ancestor_col)`
    /// when it is a value inside a block collection (so its continuation lines
    /// must out-indent the block), else `None`. Called before the container is
    /// pushed, so `self.stack` holds only its ancestors.
    fn flow_guard(&self, flow_style: bool, span: &Span) -> Option<(usize, usize)> {
        if !flow_style {
            return None;
        }
        self.nearest_block_ancestor_col()
            .map(|anc| (span.start.line(), anc))
    }

    /// Reject a flow token that resumes on a later line at a column no greater
    /// than its enclosing block collection -- the `9C9N` pattern that
    /// granit-parser 1.3 wrongly accepts. Reads only the O(1) line/column granit
    /// already computed per marker, so it adds no per-scalar text scan and no
    /// quadratic parse cost; correctly indented multi-line flows (the valid
    /// majority) and document-root flows are untouched.
    fn check_flow_continuation(&mut self, span: &Span) {
        if self.flow_indent_error.is_some() {
            return;
        }
        let line = span.start.line();
        let col = span.start.col();
        let violated = self.stack.iter().any(|state| match container_guard(state) {
            Some((open_line, anc)) => line > open_line && col <= anc,
            None => false,
        });
        if violated {
            self.flow_indent_error = Some(ParseError::Syntax {
                message: "YAML parse error: wrongly indented flow collection continuation"
                    .to_string(),
                line: 0,
                col: 0,
            });
        }
    }
}

impl<'a> SpannedEventReceiver<'a> for AstReceiver<'a> {
    /// 处理 saphyr 解析器事件，构建 AST 节点并管理解析栈。
    fn on_event(&mut self, event: Event<'a>, span: Span) {
        match event {
            Event::StreamStart | Event::StreamEnd | Event::DocumentStart(..) => {}
            Event::DocumentEnd => self.on_document_end(),
            Event::Scalar(value, style, anchor_id, tag) => {
                self.on_scalar_event(&value, &style, anchor_id, tag, span);
            }
            Event::MappingStart(_, anchor_id, tag) => {
                self.on_mapping_start(anchor_id, tag, span);
            }
            Event::MappingEnd => self.on_mapping_end(span),
            Event::SequenceStart(_, anchor_id, tag) => {
                self.on_sequence_start(anchor_id, tag, span);
            }
            Event::SequenceEnd => self.on_sequence_end(span),
            Event::Alias(anchor_id) => self.on_alias_event(anchor_id),
            Event::Comment(text, placement) => self.on_comment_event(&text, placement),
            _ => {} // granit_parser::Event is #[non_exhaustive]
        }
    }
}

impl<'a> AstReceiver<'a> {
    /// Handle `DocumentEnd`: move the completed document into the multi-doc
    /// collection. Ownership is *moved* (not cloned): the next document
    /// rebuilds `result` from scratch, and `parse_all_with_options` reads the
    /// documents list — so a per-document deep clone would be pure overhead.
    /// When `DocumentEnd` never fires, `result` is untouched and the
    /// empty-`documents` fallback in the callers still applies.
    fn on_document_end(&mut self) {
        if self.collect_documents
            && let Some(doc) = self.result.take()
        {
            self.documents.push(doc);
        }
    }

    /// Handle `Scalar`: build a scalar node, attach standalone comment, anchor
    /// and tag. Extracted from `on_event`.
    fn on_scalar_event(
        &mut self,
        value: &str,
        style: &SaphyrScalarStyle,
        anchor_id: usize,
        tag: Option<Cow<'a, granit_parser::Tag>>,
        span: Span,
    ) {
        if value == "<<" {
            self.has_merge_key = true;
        }
        let line = span.start.line() - 1; // Convert to 0-indexed
        let range = self.span_to_byte_range(&span);

        // `9C9N` guard: a flow entry resuming under-indented is invalid.
        self.check_flow_continuation(&span);

        let standalone = self.pending_standalone_comment.take();

        let mut node = self.create_scalar(value, style, line, range);

        // PR #117b: standalone notes now ride onto the dedicated
        // `decor.leading_comment` slot rather than the shared
        // `comment` field with `standalone = true`. Hand-built
        // fixtures and pre-#117b tests keep comparing equal thanks to
        // the AST-layer normalisation introduced in #117.
        if let Some(comment) = standalone
            && let CustomNode::Scalar { meta: m, .. } = &mut node
            && m.standalone_slot().is_none()
        {
            let decor = m.decor.get_or_insert_with(Default::default);
            decor.leading_comment = Some(comment);
        }

        if let Some(name) = self.register_anchor(anchor_id)
            && let CustomNode::Scalar { meta: m, .. } = &mut node
        {
            m.anchor = Some(name);
        }

        if let Some(tag) = tag
            && let CustomNode::Scalar { meta: m, .. } = &mut node
        {
            m.tag = Some(convert_tag(&tag));
        }

        self.push_node(node);
    }

    /// Consume the next raw anchor name for a granit numeric `anchor_id`,
    /// recording the mapping for later alias resolution. `None` when this
    /// node carries no anchor or the name list is exhausted.
    fn register_anchor(&mut self, anchor_id: usize) -> Option<String> {
        if anchor_id != 0 && self.anchor_name_idx < self.anchor_names.len() {
            let name = self.anchor_names[self.anchor_name_idx].clone();
            self.anchor_name_idx += 1;
            self.anchors.insert(anchor_id, name.clone());
            Some(name)
        } else {
            None
        }
    }

    /// Shared prologue of `MappingStart`/`SequenceStart`: depth guard, flow
    /// detection against the expected opening bracket, anchor registration
    /// and tag conversion. `None` means max depth was hit (flag already set).
    fn begin_container(
        &mut self,
        anchor_id: usize,
        tag: Option<Cow<'a, granit_parser::Tag>>,
        span: Span,
        expect: u8,
    ) -> Option<(bool, usize, Option<crate::ast::Tag>)> {
        if self.stack.len() >= self.max_depth {
            self.max_depth_exceeded = true;
            return None;
        }
        let flow_style = self.detect_flow_style(&span, expect);
        let start_byte = self.span_to_byte_range(&span).start;

        let standalone = self.pending_standalone_comment.take();

        self.register_anchor(anchor_id);

        let tag_obj = tag.map(|t| convert_tag(&t));

        // Own slot for this container; popped by the matching End event.
        self.comment_stack.push(standalone);
        Some((flow_style, start_byte, tag_obj))
    }

    /// Handle `MappingStart`: push a new mapping parse state with flow style,
    /// anchor, tag and pending standalone comment. Extracted from `on_event`.
    fn on_mapping_start(
        &mut self,
        anchor_id: usize,
        tag: Option<Cow<'a, granit_parser::Tag>>,
        span: Span,
    ) {
        let Some((flow_style, start_byte, tag_obj)) =
            self.begin_container(anchor_id, tag, span, b'{')
        else {
            return;
        };
        let guard = self.flow_guard(flow_style, &span);
        self.stack.push(ParseState::Mapping {
            pairs: IndexMap::new(),
            current_key: Box::new(None),
            anchor_id,
            tag: tag_obj,
            flow_style,
            start_byte,
            start_col: span.start.col(),
            guard,
        });
    }

    /// Handle `MappingEnd`: finalize the mapping node and push it. Extracted
    /// from `on_event`.
    fn on_mapping_end(&mut self, span: Span) {
        if let Some(ParseState::Mapping {
            pairs,
            anchor_id,
            tag,
            flow_style,
            start_byte,
            ..
        }) = self.stack.pop()
        {
            let anchor = self.anchors.get(&anchor_id).cloned();
            let standalone = self.comment_stack.pop().flatten();

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

            let mut meta = NodeMeta {
                comment: None,
                decor: None,
                anchor,
                tag,
                source_range: Some(start_byte..end),
            };
            // PR #117b: standalone notes on a container surface through
            // `decor.leading_comment`. The `comment` slot is left for
            // inline trailing notes (currently only set by
            // `attach_inline_comment`).
            if let Some(comment) = standalone {
                let decor = meta.decor.get_or_insert_with(Default::default);
                decor.leading_comment = Some(comment);
            }

            let mapping = CustomNode::Mapping {
                pairs,
                flow_style,
                meta,
            };
            self.push_node(mapping);
        }
    }

    /// Handle `SequenceStart`: push a new sequence parse state with flow style,
    /// anchor, tag and pending standalone comment. Extracted from `on_event`.
    fn on_sequence_start(
        &mut self,
        anchor_id: usize,
        tag: Option<Cow<'a, granit_parser::Tag>>,
        span: Span,
    ) {
        let Some((flow_style, start_byte, tag_obj)) =
            self.begin_container(anchor_id, tag, span, b'[')
        else {
            return;
        };
        let guard = self.flow_guard(flow_style, &span);
        self.stack.push(ParseState::Sequence {
            items: Vec::new(),
            anchor_id,
            tag: tag_obj,
            flow_style,
            start_byte,
            start_col: span.start.col(),
            guard,
        });
    }

    /// Handle `SequenceEnd`: finalize the sequence node and push it. Extracted
    /// from `on_event`.
    fn on_sequence_end(&mut self, span: Span) {
        if let Some(ParseState::Sequence {
            items,
            anchor_id,
            tag,
            flow_style,
            start_byte,
            ..
        }) = self.stack.pop()
        {
            let anchor = self.anchors.get(&anchor_id).cloned();
            let standalone = self.comment_stack.pop().flatten();

            let end = if !flow_style {
                items
                    .iter()
                    .filter_map(|i| i.source_range().map(|r| r.end))
                    .max()
                    .unwrap_or_else(|| self.span_to_byte_range(&span).start)
            } else {
                self.span_to_byte_range(&span).end
            };

            let mut meta = NodeMeta {
                comment: None,
                decor: None,
                anchor,
                tag,
                source_range: Some(start_byte..end),
            };
            // PR #117b: same slot split as the mapping case above.
            if let Some(comment) = standalone {
                let decor = meta.decor.get_or_insert_with(Default::default);
                decor.leading_comment = Some(comment);
            }

            let seq = CustomNode::Sequence {
                items,
                flow_style,
                meta,
            };
            self.push_node(seq);
        }
    }

    /// Handle `Alias`: resolve the anchor name and push an alias node.
    /// Extracted from `on_event`.
    fn on_alias_event(&mut self, anchor_id: usize) {
        let alias_name = self
            .anchors
            .get(&anchor_id)
            .cloned()
            .unwrap_or_else(|| format!("alias_{}", anchor_id));

        let node = CustomNode::Alias { name: alias_name };
        self.push_node(node);
    }

    /// Handle `Comment`: stash standalone comments or attach inline comments.
    /// Extracted from `on_event`.
    fn on_comment_event(&mut self, text: &str, placement: granit_parser::Placement) {
        let text = Arc::from(text.trim());
        if is_standalone_placement(&placement) {
            self.pending_standalone_comment = Some(Comment {
                text,
                standalone: true,
            });
        } else if placement == granit_parser::Placement::Right {
            self.attach_inline_comment(text);
        }
        // Placement is #[non_exhaustive]: other variants are ignored.
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
            assert_eq!(value.as_ref(), "hello");
            assert_eq!(style, ScalarStyle::Plain);
        }
    }

    #[test]
    fn standalone_comment_survives_nested_first_value() {
        // Regression: container starts clobbered a single shared comment
        // slot, so a document header comment was dropped at PARSE time
        // whenever the first key's value was itself a container.
        let ast = parse("# header\napp:\n  name: demo\nport: 1\n", YamlSchema::Core).unwrap();
        let CustomNode::Mapping { meta, .. } = &ast else {
            panic!("expected root mapping")
        };
        // PR #117b: standalone notes now ride onto `decor.leading_comment`;
        // the normalised `standalone_slot()` accessor reads whichever slot
        // the parser wrote to.
        assert_eq!(meta.standalone_slot().unwrap().text.as_ref(), "header");
        let yaml = crate::serializer::to_yaml(&ast);
        assert!(yaml.starts_with("# header\n"), "{yaml:?}");
    }

    #[test]
    fn receiver_writes_standalone_into_leading_comment_slot() {
        // Structural invariant introduced by PR #117b: after the
        // receiver migration a parsed standalone note lands in
        // `decor.leading_comment`, NOT in the older `comment` field.
        // The accessor equality from #117 would pass either way, so
        // this test locks the WRITE location specifically.
        let ast = parse("key:\n  # above the value\n  value\n", YamlSchema::Core).unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            panic!("expected root mapping")
        };
        let (_, val) = pairs.iter().next().unwrap();
        let CustomNode::Scalar { meta, .. } = val else {
            panic!("expected scalar value")
        };
        assert!(
            meta.decor
                .as_ref()
                .and_then(|d| d.leading_comment.as_ref())
                .is_some(),
            "standalone comment did not land in the new leading_comment slot"
        );
        assert!(
            meta.comment.is_none(),
            "legacy `comment` slot should be empty after #117b"
        );
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
    fn flow_continuation_under_indented_is_rejected() {
        // Regression for the strictness gap granit-parser 1.3 opened
        // (yaml-test-suite `9C9N`): a multi-line flow collection used as a block
        // value may not have a continuation line indented at or below the block
        // key. The in-tree guard rejects it and restores 405/406 compliance.
        assert!(parse("flow: [a,\nb,\nc]\n", YamlSchema::Core).is_err());
        // Correctly out-dented continuations still parse (no over-rejection).
        assert!(parse("flow: [a,\n  b,\n  c]\n", YamlSchema::Core).is_ok());
        // Single-line flow never has a continuation to check.
        assert!(parse("flow: [a, b, c]\n", YamlSchema::Core).is_ok());
        // Same rule via a flow mapping value.
        assert!(parse("flow: {a: 1,\nb: 2}\n", YamlSchema::Core).is_err());
    }

    #[test]
    fn test_parse_tag() {
        let yaml = "name: !!str John";
        let result = parse(yaml, YamlSchema::Core);
        assert!(result.is_ok());
        if let Ok(CustomNode::Mapping { pairs, .. }) = result {
            for (k, v) in pairs {
                if let CustomNode::Scalar { value, .. } = k
                    && value.as_ref() == "name"
                    && let CustomNode::Scalar { meta, .. } = v
                {
                    assert!(meta.tag.is_some());
                    assert_eq!(meta.tag.unwrap().suffix, "str");
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
                if let CustomNode::Scalar { value, .. } = k
                    && value.as_ref() == "defaults"
                    && let CustomNode::Mapping { meta, .. } = v
                {
                    assert!(meta.anchor.is_some());
                    assert_eq!(meta.anchor.unwrap(), "defaults");
                }
            }
        }
    }

    /// The multi-doc collection path moves each finished document out of
    /// `result` (no deep clone). Every document must still arrive intact and
    /// in order, and no state may leak between documents.
    #[test]
    fn test_parse_all_moves_documents_intact() {
        let yaml = "first: 1\n# c\n---\nsecond: [a, b]\n---\nthird: &t x\nfourth: *t\n";
        let docs = parse_all(yaml, YamlSchema::Core).unwrap();
        assert_eq!(docs.len(), 3);
        // doc 1: single mapping entry
        let CustomNode::Mapping { pairs, .. } = &docs[0] else {
            panic!("doc0")
        };
        assert_eq!(pairs.len(), 1);
        let CustomNode::Scalar {
            value: key_value, ..
        } = pairs.keys().next().unwrap()
        else {
            panic!("doc0 key")
        };
        assert_eq!(key_value.as_ref(), "first");
        // doc 2: mapping with a 2-item sequence
        let CustomNode::Mapping { pairs, .. } = &docs[1] else {
            panic!("doc1")
        };
        assert_eq!(pairs.len(), 1);
        let CustomNode::Sequence { items, .. } = pairs.values().next().unwrap() else {
            panic!("doc1 seq")
        };
        assert_eq!(items.len(), 2);
        // doc 3: anchor + alias preserved through the move
        let CustomNode::Mapping { pairs, .. } = &docs[2] else {
            panic!("doc2")
        };
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs.values().next().unwrap().anchor(), Some("t"));
        assert!(matches!(
            pairs.values().nth(1).unwrap(),
            CustomNode::Alias { .. }
        ));
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
        let CustomNode::Mapping { pairs, meta, .. } = node else {
            panic!()
        };
        assert_eq!(meta.source_range, Some(0usize..9)); // up to the last child ("1") end
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
        let CustomNode::Mapping { meta, .. } = node else {
            panic!()
        };
        assert_eq!(meta.source_range, Some(0usize..6)); // covers "{a: 1}" incl. '}'
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
