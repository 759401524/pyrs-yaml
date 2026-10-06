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
    BlockHeader, anchor_name_before, compute_line_offsets, detect_block_header, resolve_merge_keys,
};

/// Return true if a mapping key *resolves to null*: a `Null` node, or an
/// untagged **plain** scalar spelled `~`, `null` (any case) or empty.
///
/// Style and tag are what decide it — YAML resolves implicit types on plain
/// scalars only, so a quoted `"NULL"` / `"~"` key or a `!!str null` key is an
/// ordinary string key. Testing the text alone made every quoted null spelling
/// look like a null key, which cost the duplicate-key exemption its precision and
/// would cost the fold below two distinct string keys: `{"": None, "NULL": None}`
/// lost its empty key through the JSON5 / TOML round trips in
/// `tests/test_property_dialects.py`.
///
/// The yaml-test-suite allows duplicate null keys (e.g. `: a\n: b`, see 2JQS),
/// so duplicate-key detection must not reject them.
fn is_null_key(key: &CustomNode) -> bool {
    match key {
        CustomNode::Null { meta, .. } if meta.tag.is_none() => true,
        CustomNode::Scalar {
            value,
            style: ScalarStyle::Plain,
            meta,
            ..
        } if meta.tag.is_none() => {
            value.is_empty() || value.as_ref() == "~" || value.eq_ignore_ascii_case("null")
        }
        _ => false,
    }
}

/// Return true if a mapping key is the `<<` merge key.
///
/// YAML explicitly permits a mapping to repeat `<<` (`<<: *a` plus `<<: *b`),
/// so the value-key duplicate check must exempt it — collapsing two merge keys
/// would reject well-formed documents (`nested_self_anchor_merge_terminates`).
fn is_merge_key(key: &CustomNode) -> bool {
    matches!(key, CustomNode::Scalar { value, .. } if value.as_ref() == "<<")
}

/// Inline the nodes that defined anchors the duplicate-key fold removed.
///
/// A fold may only drop a pair, but a pair can *define* an anchor that aliases elsewhere
/// in the same document still name. YAML resolves an alias against definitions inside
/// the document, so a dropped definition turns every surviving `*name` into an
/// unresolvable reference — the writer emits `<: *b` with no `&b` anywhere, and the
/// text fails our own reader ("found unknown anchor"), which the engine promises never
/// to produce. This AST is a value tree (an alias carries no identity beyond the node
/// it names), so replacing each use with the anchored node preserves the value and puts
/// the definition before it.
///
/// Only names with **no remaining definition** are inlined, so ordinary shared aliases
/// are never touched, and the walk runs solely when a fold actually dropped an
/// anchored entry — the common path pays nothing.
fn repair_orphaned_anchors(root: &mut CustomNode, orphans: Vec<(String, CustomNode)>) {
    if orphans.is_empty() {
        return;
    }
    let mut defined = Vec::new();
    collect_anchor_names(root, &mut defined);
    let mut replacements = std::collections::HashMap::new();
    for (name, node) in orphans {
        if defined.contains(&name) || replacements.contains_key(&name) {
            continue;
        }
        replacements.insert(name, node);
    }
    if !replacements.is_empty() {
        inline_aliases(root, &replacements);
    }
}

/// Every anchor name defined in `node`'s subtree, including keys' own.
fn collect_anchor_names(node: &CustomNode, out: &mut Vec<String>) {
    if let Some(name) = node.anchor() {
        out.push(name.to_string());
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs.iter() {
                collect_anchor_names(key, out);
                collect_anchor_names(value, out);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                collect_anchor_names(item, out);
            }
        }
        _ => {}
    }
}

/// Replace each `*name` value-alias in the subtree whose name is in `replacements`.
///
/// Mapping keys are deliberately not rewritten: swapping a key node changes the
/// mapping's identity and order mid-iteration, and the fold that orphans an anchor never
/// leaves an alias in a key slot pointing at the dropped definition (the surviving
/// entry supplies the key). If a future finding lands that shape, it owes a deliberate
/// key-slot rewrite with its own ordering proof — not a silent leave-behind here.
fn inline_aliases(
    node: &mut CustomNode,
    replacements: &std::collections::HashMap<String, CustomNode>,
) {
    if let Some(replacement) = match node {
        CustomNode::Alias { name } => replacements.get(name).cloned(),
        _ => None,
    } {
        *node = replacement;
        return;
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (_, value) in pairs.iter_mut() {
                inline_aliases(value, replacements);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items.iter_mut() {
                inline_aliases(item, replacements);
            }
        }
        _ => {}
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
    let mut receiver = AstReceiver::new(yaml, max_depth, allow_duplicate_keys);
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
    // Handle empty YAML - by YAML's own vocabulary. `str::trim` is Unicode-based
    // and also strips NBSP, which the reader treats as an ordinary scalar
    // character, so trimming with it turned a NBSP-only document into `null`
    // (libFuzzer `yaml_roundtrip` crash-512814).
    if pyrs_schema::is_yaml_blank_only(yaml) {
        return Ok(CustomNode::plain_null());
    }

    let receiver = load_ast(yaml, max_depth, allow_duplicate_keys, false)?;
    let has_merge_key = receiver.has_merge_key;

    // Get the parsed node (handle empty documents, and a document that carried
    // nothing but comments).
    let mut node = receiver.finish_document();

    // Resolve merge keys (<<) after parsing (if enabled and any were detected)
    if resolve_merges && has_merge_key {
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
    // Handle empty YAML (same YAML-blank rule as `parse`; see crash-512814).
    if pyrs_schema::is_yaml_blank_only(yaml) {
        return Ok(Vec::new());
    }

    let receiver = load_ast(yaml, max_depth, allow_duplicate_keys, true)?;
    let has_merge_key = receiver.has_merge_key;

    if receiver.documents.is_empty() {
        // Single document — return as-is, notes included.
        let mut node = receiver.finish_document();
        if resolve_merges && has_merge_key {
            resolve_merge_keys(&mut node);
        }
        return Ok(vec![node]);
    }

    let mut results = receiver.documents;
    if resolve_merges && has_merge_key {
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
pub(crate) fn char_to_byte_offsets(text: &str) -> Vec<usize> {
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
    /// Standalone/inline comments per in-progress container, parallel to
    /// `stack`. A single shared slot was clobbered by nested container starts
    /// (a block mapping's header comment vanished when its first value was
    /// itself a container), and a single `Option` per container kept only the
    /// last of any stacked notes — so the slot is a list.
    comment_stack: Vec<Vec<Comment>>,
    stack: Vec<ParseState>,
    result: Option<CustomNode>,
    /// Whether to collect completed documents for multi-doc parsing. When
    /// false (single-document parse), `DocumentEnd` skips the full AST clone.
    collect_documents: bool,
    /// Completed documents (for multi-doc parsing)
    documents: Vec<CustomNode>,
    /// Current anchor ID to name mapping
    anchors: std::collections::HashMap<usize, String>,
    /// Standalone notes seen so far that have not met a node yet, in source
    /// order. A list, not a slot: stacking any number of comment lines above one
    /// key is ordinary, and overwriting here kept only the last of them.
    pending_standalone_comment: Vec<Comment>,
    /// Set once `DocumentEnd` has fired for the current document, and cleared by the
    /// next `DocumentStart`. A note reported after that point cannot be any node's
    /// trailing note — there is no node left to trail — so it must travel through the
    /// pending slot to `finish_document` instead of being attached back onto the
    /// finished root.
    document_ended: bool,
    /// Maximum allowed nesting depth for mapping/sequence containers
    max_depth: usize,
    /// Set to true when the maximum nesting depth is exceeded
    max_depth_exceeded: bool,
    /// When false, duplicate mapping keys cause a parse error
    allow_duplicate_keys: bool,
    /// Stored duplicate key error (since on_event can't return Result)
    duplicate_key_error: Option<ParseError>,
    /// Anchors defined by entries the duplicate-key fold removed, paired with the node
    /// that carried them. An alias may still name one of these, and a definition that
    /// left the tree cannot be re-added by the writer — `finish_document` inlines the
    /// node at each remaining use so the emission never references an undefined anchor
    /// (libFuzzer `yaml_roundtrip` crash-43eca7a3).
    orphaned_anchors: Vec<(String, CustomNode)>,
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
        /// Scalar-key values already seen, tracked by content so YAML's
        /// "keys are their value" identity is enforced: `key`, `key # c` and
        /// `"key"` are the same key even though their `CustomNode` (style /
        /// trailing comment / anchor) differ. Without this the `IndexMap`
        /// (keyed by full node) kept both, no duplicate fired, yet the
        /// serializer drops key decor and emitted two colliding `key:` lines
        /// that re-parse rejected (libFuzzer `yaml_roundtrip` crash-3b0a7d1d).
        /// Holds `Arc<str>` so recording a seen key is a refcount bump off the
        /// node's existing allocation, not a fresh `String`.
        seen_value_keys: std::collections::HashSet<Arc<str>>,
        /// Slot of this mapping's null key, once one is inserted — see the fold in
        /// `push_node`. Only one can exist, so remembering where it is keeps a
        /// null-key-heavy document linear instead of rescanning every key.
        null_key: Option<usize>,
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
    fn new(yaml_text: &'a str, max_depth: usize, allow_duplicate_keys: bool) -> Self {
        let is_ascii = yaml_text.is_ascii();
        Self {
            yaml_text,
            char_offsets: (!is_ascii).then(|| char_to_byte_offsets(yaml_text)),
            stack: Vec::new(),
            result: None,
            collect_documents: true,
            documents: Vec::new(),
            anchors: std::collections::HashMap::new(),
            comment_stack: Vec::new(),
            pending_standalone_comment: Vec::new(),
            document_ended: false,
            max_depth,
            max_depth_exceeded: false,
            allow_duplicate_keys,
            duplicate_key_error: None,
            orphaned_anchors: Vec::new(),
            has_merge_key: false,
            flow_indent_error: None,
        }
    }

    /// Create a scalar node from value, style, and its byte range in the source
    fn create_scalar(
        &mut self,
        value: &str,
        style: &SaphyrScalarStyle,
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
        // from the source text, anchored to the scalar's own byte span. Both
        // indicators live only in the header, so a single scan recovers them
        // together; `range.start` is the first content byte (granit points a
        // block scalar's span at its content, not its header line).
        let block_header = if matches!(scalar_style, ScalarStyle::Literal | ScalarStyle::Folded) {
            detect_block_header(self.yaml_text, range.start)
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

    /// True when a `\n` separates the candidate node's end from the comment,
    /// i.e. the comment is on a strictly later line.
    ///
    /// Only the gap between the two is scanned, so this stays `O(distance)`;
    /// building a whole-document line table here would put an `O(len)` pass on
    /// the first comment of every document, on the parse hot path. A comment
    /// that starts at or before the candidate's real end - notably a note on a
    /// block scalar's header line, while granit spans that node at its
    /// *content* - is never reported as separated, so it keeps binding inline.
    fn line_break_between(text: &str, node_end: usize, comment_byte: usize) -> bool {
        let bytes = text.as_bytes();
        // A block collection's span runs through the line break that ends its own
        // line, so `node_end` can already sit on the *next* line and the gap it
        // produces holds no `\n` even though the note is on later text. Back over
        // the blanks the span swallowed before looking for the break: `b:` /
        // ` ?` (span `4..6`) / `? #i` otherwise looked same-line, the note landed
        // as the nested mapping's trailing comment, the writer spilled it inside
        // that mapping, the re-read gave it to the shallower next entry, and
        // ownership climbed a level every round (libFuzzer `yaml_roundtrip`
        // crash-0e1c4378, minimised to `b:\n ?\n? #i`).
        let mut end = node_end.min(bytes.len());
        while end > 0 && matches!(bytes[end - 1], b' ' | b'\t' | b'\r' | b'\n') {
            end -= 1;
        }
        comment_byte > end
            && bytes
                .get(end..comment_byte.min(bytes.len()))
                .is_some_and(|gap| gap.contains(&b'\n'))
    }

    /// Record a trailing (`Placement::Right`) note.
    ///
    /// Returns `true` when the note landed on an existing node (or on the
    /// currently-open empty container, which has no child to compare against),
    /// and `false` when the note sits on a *later* line than the backwards
    /// candidate, meaning it annotates a node that has not been produced yet and
    /// the caller must carry it forward as that node's leading note.
    ///
    /// `key: value # note` is on the value's own line and is that value's
    /// trailing note, and a note on a block scalar's header line (`y: |  # n`)
    /// precedes the node's range because granit spans a block scalar at its
    /// *content*, not its header. Only a note separated from the candidate by a
    /// line break annotates something not yet produced. That is exactly `- #e`:
    /// the dash line of a *following* sequence item whose own content is empty.
    /// Binding it backwards mis-homed it on the previous item's value, the writer
    /// spilled it inside that item's block, and the re-read bound it to the next
    /// item instead, so ownership flipped every round (libFuzzer
    /// `yaml_roundtrip` crash-aee06aca, minimized `- :\u{feff}:\n- #e`).
    ///
    /// A note reported after `DocumentEnd` is refused **when the finished root is not
    /// a block container**: the document has ended, so nothing it could trail remains.
    /// `!m` CR `...` SP `# -o` reaches here with the root already finished, and binding
    /// the note back onto that root as an *inline* note made the first emission
    /// `!m   # -o` — a spelling whose own re-read reports the note **before** the node
    /// and homes it as a leading note. Two ingest orders, two slots, and the round trip
    /// never settled (crash-7918272c, 11 bytes). Carrying it forward instead puts both
    /// orders in the same slot.
    ///
    /// A container root keeps the inline home, because there the reader really does
    /// report the note back from the last line of the body: `a: 1\n# trailing note`
    /// ingests inline, emits `…  # note` on that last value line and re-reads
    /// identically — the shape `flush_trailing_comment` was written for
    /// (crash-96fa252c). Refusing there would trade a settled shape for a relocated
    /// one. That stops holding when the body's last value carries no text (`!`, `!-`,
    /// any tag): the scanner leaves that value pending, so a note sitting on the pair
    /// line re-reads as the *value's* leading note — and the writer emits it there
    /// instead of on the line, which is the routed half of
    /// `Serializer::carried_container_note`. Pinned by
    /// `a_containers_inline_note_after_a_text_less_value_settles_at_once` and
    /// `a_quoted_hash_key_settles_the_containers_note_at_once`, whose `"+#"` document is
    /// exactly that case; both tests first recorded the routed form as *round two's*
    /// output, which is how the rule was found.
    fn attach_inline_comment(&mut self, text: Arc<str>, comment_byte: usize) -> bool {
        if self.document_ended
            && !matches!(
                self.result,
                Some(CustomNode::Mapping { .. } | CustomNode::Sequence { .. })
            )
        {
            return false;
        }
        let comment = Comment {
            text,
            standalone: false,
        };

        // Resolve the backwards candidate without mutating anything, so the line
        // test can decide before we commit the note anywhere.
        let candidate: Option<Range<usize>> = match self.stack.last() {
            Some(ParseState::Mapping {
                current_key, pairs, ..
            }) => {
                let current: &Option<CustomNode> = current_key;
                let node = match current {
                    Some(k) => Some(k),
                    None => pairs.iter().next_back().map(|(_, v)| v),
                };
                node.and_then(CustomNode::source_range).cloned()
            }
            Some(ParseState::Sequence { items, .. }) => {
                items.last().and_then(CustomNode::source_range).cloned()
            }
            None => self
                .result
                .as_ref()
                .and_then(CustomNode::source_range)
                .cloned(),
        };

        if candidate
            .as_ref()
            .is_some_and(|range| Self::line_break_between(self.yaml_text, range.end, comment_byte))
        {
            return false;
        }

        // The note still belongs to the candidate (same line, or earlier for a
        // block-scalar header): keep it where it lived before.
        //
        // Reporting success when there was *nothing* to hang the note on is how a
        // note silently vanished. granit delivers the note of a root node that
        // carries only properties (`!x # note`, `&a # note`, and the whole `#` wall of
        // crash-ce106ccc) as a Right-placed comment **before** the `Scalar` event, so
        // at that moment the stack is empty and `result` is `None` — there is no
        // candidate at all. The old code fell through every branch, returned `true`
        // anyway, and the caller trusted it: the note was neither attached nor
        // carried forward, and the loss was invisible to a text-idempotence oracle
        // because a document short of its note is a perfectly stable document. Only a
        // real attachment may report success; anything else goes back to the caller
        // as "carry it forward", which `on_scalar_event` then homes on the node that
        // follows (crash-7918272c's `!m   # -o` is that same note written inline, and
        // is why re-reading our own emission used to lose it).
        let mut attached = false;
        if let Some(top) = self.stack.last_mut() {
            match top {
                ParseState::Mapping {
                    current_key, pairs, ..
                } => {
                    // Attach to the last value if complete, or the current key
                    let target = if current_key.is_some() {
                        current_key.as_mut().as_mut()
                    } else {
                        pairs.iter_mut().last().map(|(_, v)| v)
                    };
                    if target.is_some() {
                        Self::set_scalar_comment(target, comment);
                        attached = true;
                    } else if let Some(slot) = self.comment_stack.last_mut() {
                        // Empty mapping `{}` — stash in the container's own slot
                        // (on_mapping_end attaches it to the node).
                        slot.push(comment);
                        attached = true;
                    }
                }
                ParseState::Sequence { items, .. } => {
                    if !items.is_empty() {
                        Self::set_scalar_comment(items.last_mut(), comment);
                        attached = true;
                    } else if let Some(slot) = self.comment_stack.last_mut() {
                        // Empty sequence `[]` — stash in the container's slot.
                        slot.push(comment);
                        attached = true;
                    }
                }
            }
        } else if let Some(result) = &mut self.result {
            Self::set_scalar_comment(Some(result), comment);
            attached = true;
        }
        attached
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
                current_key,
                pairs,
                seen_value_keys,
                null_key,
                ..
            }) => {
                if current_key.is_none() {
                    **current_key = Some(node);
                } else if let Some(mut key) = current_key.take() {
                    if self.max_depth_exceeded || self.allow_duplicate_keys {
                        pairs.insert(key, node);
                    } else if is_null_key(&key) || is_merge_key(&key) {
                        // A YAML mapping holds exactly one null key — and exactly one
                        // merge key. The exemption that keeps 2JQS (`: a` + `: b`)
                        // parsing used to hand the pair straight to `IndexMap`, which
                        // compares *whole* nodes — and two spellings of null differ in
                        // metadata (style, source range, comment), so an empty key and a
                        // `~` key both stayed, both rendered as `~:`, and the reader
                        // folded them on re-parse: the document lost a line every round
                        // (libFuzzer `yaml_roundtrip` crash-00e31785, 9 bytes
                        // `: &b #*\r:`). Drop the earlier entry of the same identity in
                        // place and keep this one, which is what re-reading the emitted
                        // text actually yields.
                        //
                        // The merge key needs the same fold for the same reason, and did
                        // not become a problem until `<<` was recognised by identity
                        // rather than by whole-node equality: a plain `<<` and a `<<`
                        // carrying a note then sat side by side, the merge pass addresses
                        // one entry per round, and a pair vanished between serialisations
                        // (crash-973bd522).
                        //
                        // The slot is remembered so a document full of null keys
                        // stays linear. Rescanning is not a theory: with 2k distinct
                        // keys followed by 2k null keys, growing to 8k + 8k cost
                        // 12.4x for a 4x input (99 ms vs 8 ms), because every fold
                        // walked the whole map — and an all-null document looks linear
                        // only because its folded entry sits at slot 0. The scan stays
                        // as the fallback whenever the tracked slot does not hold a
                        // null key (appends cannot invalidate it, but the guard makes
                        // the fold correct without relying on that).
                        let is_merge = is_merge_key(&key);
                        let prior = match *null_key {
                            Some(index)
                                if !is_merge
                                    && pairs
                                        .get_index(index)
                                        .is_some_and(|(k, _)| is_null_key(k)) =>
                            {
                                Some(index)
                            }
                            _ => pairs.keys().position(|k| {
                                if is_merge {
                                    is_merge_key(k)
                                } else {
                                    is_null_key(k)
                                }
                            }),
                        };
                        if let Some(index) = prior {
                            // The folded entry takes its comments with it unless they
                            // are re-homed: `: &b #*<CR>:` kept its note on the entry
                            // that the fold removed, so the text vanished from the
                            // document (`~: ~` alone) — silently, and stably, so only
                            // the fuzz tier's note-survival oracle sees it. The note is
                            // content of this mapping either way, so it moves onto the
                            // key that survives.
                            let mut orphans: Vec<Comment> = Vec::new();
                            if let Some((dropped_key, dropped_value)) =
                                pairs.shift_remove_index(index)
                            {
                                for carried in [dropped_key, dropped_value] {
                                    orphans.extend(carried.leading_comments().iter().cloned());
                                    if let Some(name) = carried.anchor() {
                                        // The anchor's definition leaves the tree with this
                                        // entry, so hold the node: an alias that still names it
                                        // gets inlined at `finish_document`. Notes alone were
                                        // re-homed here; an anchor was silently dropped, and the
                                        // loss only shows as unparseable output, not as drift.
                                        self.orphaned_anchors
                                            .push((name.to_string(), carried.clone()));
                                    }
                                    if let Some(inline) =
                                        carried.comment().filter(|c| !c.standalone)
                                    {
                                        orphans.push(inline.clone());
                                    }
                                }
                            }
                            for note in orphans {
                                key.push_leading_comment(note);
                            }
                        }
                        // Removing the tracked entry and appending this one: after
                        // `insert` the fresh key is the last pair, so that is its slot.
                        // Only the null case is tracked; a merge key is rare enough that
                        // the scan above is the whole cost, and tracking it would let a
                        // stale slot mislead the next null fold.
                        pairs.insert(key, node);
                        if !is_merge {
                            *null_key = Some(pairs.len() - 1);
                        }
                    } else {
                        // YAML identifies a key by its *value*: two scalar keys
                        // with the same text collide even when their style,
                        // trailing comment or anchor differ, and the serializer
                        // emits them identically. The `IndexMap` (keyed by the
                        // full node) alone misses those, so track the emitted
                        // value here and reject the collision (crash-3b0a7d1d).
                        // `HashSet::insert` does the lookup and the store in a
                        // single hashing pass and returns whether the value was
                        // new, so the common no-duplicate key costs one probe and
                        // an `Arc` refcount bump (no `String` allocation); the
                        // `String` is materialized only when a collision is found.
                        let value_dup: Option<String> = match &key {
                            // A tag replaces the implicit resolution, so `!a null`
                            // and `!A null` are different keys: their emitted lines
                            // carry the tag and re-read apart, which is exactly what
                            // must not be reported as a collision.
                            CustomNode::Scalar { value, meta, .. } if meta.tag.is_none() => {
                                if seen_value_keys.insert(Arc::clone(value)) {
                                    None
                                } else {
                                    Some(value.to_string())
                                }
                            }
                            _ => None,
                        };
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
                        if let Some(v) = value_dup
                            && self.duplicate_key_error.is_none()
                        {
                            self.duplicate_key_error = Some(ParseError::DuplicateKey(v));
                        }
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
            Event::StreamStart | Event::StreamEnd => {}
            Event::DocumentStart(..) => self.document_ended = false,
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
            Event::Comment(text, placement) => self.on_comment_event(&text, placement, span),
            _ => {} // granit_parser::Event is #[non_exhaustive]
        }
    }
}

impl<'a> AstReceiver<'a> {
    /// Handle `DocumentEnd`: flush any trailing note onto the finished
    /// document, then move the document into the multi-doc collection.
    /// Ownership is *moved* (not cloned): the next document rebuilds `result`
    /// from scratch, and `parse_all_with_options` reads the documents list — so
    /// a per-document deep clone would be pure overhead.
    /// When `DocumentEnd` never fires, `result` is untouched and the
    /// empty-`documents` fallback in the callers still applies.
    fn on_document_end(&mut self) {
        self.flush_trailing_comment();
        self.document_ended = true;
        if self.collect_documents
            && let Some(doc) = self.result.take()
        {
            self.documents.push(doc);
        }
    }

    /// The parsed document, or — when the stream produced no node at all — a null
    /// root that inherits every note the reader reported but never got to hand to
    /// a node.
    ///
    /// A comment-only document is a null document, but `DocumentEnd` never fires
    /// for it, so its notes were stranded in the pending slot and dropped:
    /// `#&l<TAB><TAB>:` serialised to `null`, losing the entire content of the
    /// file. The data is `None` either way, so nothing about the value changes;
    /// the notes now ride the fallback root and come back above it.
    ///
    /// The stranding is not limited to the no-node case. A note that arrives after
    /// `DocumentEnd` (`!m` CR `...` SP `# -o`: the document-end marker cuts the
    /// stream off before the note) also sits in the slot when the document is
    /// finished, and the `Some` arm used to return the node and drop the notes with
    /// it — the emitted document then carried no note at all, silently, and the
    /// survival oracle could not even see the loss because there was no text left to
    /// miss (libFuzzer `yaml_roundtrip` crash-7918272c, 11 bytes). Every pending note
    /// now rides the root as a leading note: that is the position the writer can
    /// print it from and the reader hand it back from, so the emission is a fixed
    /// point rather than merely stable-and-short-a-note. Leading, not inline: a note
    /// in this position re-ingests as the node's *leading* note (probe: `!m   # -o`
    /// arrives as `lead=["-o"]`), so homing it inline would make the very next read
    /// move it and the emission would drift by one round again.
    fn finish_document(mut self) -> CustomNode {
        let notes = core::mem::take(&mut self.pending_standalone_comment);
        let mut node = self.result.unwrap_or_else(CustomNode::plain_null);
        for note in notes {
            node.push_leading_comment(note);
        }
        let orphans = core::mem::take(&mut self.orphaned_anchors);
        repair_orphaned_anchors(&mut node, orphans);
        node
    }

    /// Attach a standalone comment still pending at end-of-document to the
    /// document root as an inline trailing note.
    ///
    /// The writer cannot hang an inline note (`meta.comment`, `standalone =
    /// false`) on the same line as a *block* container — there is no line left
    /// after the last item — so it spills the note onto its own trailing line.
    /// On re-read granit reports that shape as a standalone comment with no
    /// node following it, which the receiver would otherwise strand in
    /// `pending_standalone_comment` and drop, so the second serialize lost the
    /// note and the round trip was not idempotent (libFuzzer `yaml_roundtrip`
    /// crash-96fa252c: `&"\n-\r... #-o` → `&" \n- ~\n# -o\n` → `&" \n- ~\n`).
    /// Storing it back into the very slot the writer read it from restores
    /// stability without changing the AST shape. An existing root note wins.
    fn flush_trailing_comment(&mut self) {
        if self.pending_standalone_comment.is_empty() {
            return;
        }
        let Some(root) = self.result.as_mut() else {
            self.pending_standalone_comment.clear();
            return;
        };
        let notes = core::mem::take(&mut self.pending_standalone_comment);
        // The writer hangs exactly one note on the document's last line; that is the
        // slot a re-read reports this shape from, so the first note goes back there
        // exactly as before (crash-96fa252c) and any extra keeps its existing home
        // rather than being re-homed. Leading them to the root instead made the two
        // positions swap every round (crash-11ced252, 13 bytes), so a stacked note at
        // end-of-stream is recorded as still unresolved rather than traded for a new
        // drift.
        let Some(trailing) = notes.first() else {
            return;
        };
        if root.comment().is_none() {
            root.set_comment(Comment {
                text: trailing.text.clone(),
                standalone: false,
            });
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
        let range = self.span_to_byte_range(&span);

        // `9C9N` guard: a flow entry resuming under-indented is invalid.
        self.check_flow_continuation(&span);

        let standalone = core::mem::take(&mut self.pending_standalone_comment);

        let range_start = range.start;
        let mut node = self.create_scalar(value, style, range);

        // PR #117b: standalone notes ride onto the dedicated
        // `decor.leading_comments` list rather than the shared
        // `comment` field with `standalone = true`. Hand-built
        // fixtures and pre-#117b tests keep comparing equal thanks to
        // the AST-layer normalisation introduced in #117.
        if !standalone.is_empty()
            && let CustomNode::Scalar { meta: m, .. } = &mut node
            && m.standalone_slot().is_none()
        {
            let decor = m.decor.get_or_insert_with(Default::default);
            for comment in standalone {
                decor.push_leading_comment(comment);
            }
        }

        if let Some(name) = self.register_anchor(anchor_id, range_start)
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

    /// Recover the `&name` granit attached to an anchored node by reading it
    /// back at the node's own source location (`byte_start` = byte offset of the
    /// event span start). granit only surfaces the numeric `anchor_id`, not the
    /// name text, and `anchor_id != 0` is its authoritative "this node is
    /// anchored" decision — so the name is resolved per node, position-isolated,
    /// instead of consumed from a whole-text pre-scan paired by a counter. Records
    /// the id->name mapping for later alias resolution; `None` when the node
    /// carries no anchor.
    fn register_anchor(&mut self, anchor_id: usize, byte_start: usize) -> Option<String> {
        if anchor_id == 0 {
            return None;
        }
        let mut name = anchor_name_before(self.yaml_text, byte_start)?;
        // Same ingest-side rule already applied to comment text: an anchor is
        // emitted as a bare `&name`, a position with no escape syntax, so a code
        // point that cannot appear inside a document - U+FEFF is restricted to the
        // stream's own leading BOM - would otherwise make our output unparseable
        // (libFuzzer `yaml_roundtrip` crash-2d14c6f6, whose emitted text carried
        // `&eeeeeeeo\u{FEFF}`). Filtering here, rather than at each writer, keeps
        // the anchor and every alias pointing at it in agreement because both read
        // this one stored name; a name with nothing readable left is treated as
        // unanchored instead of storing a name that cannot be written back.
        // The `all` probe keeps the common case allocation-free: rebuilding the
        // string is reserved for text that actually carries a rejected code point.
        if !name.chars().all(pyrs_schema::is_yaml_document_char) {
            name = name
                .chars()
                .filter(|&c| pyrs_schema::is_yaml_document_char(c))
                .collect();
            if name.is_empty() {
                return None;
            }
        }
        self.anchors.insert(anchor_id, name.clone());
        Some(name)
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

        let standalone = core::mem::take(&mut self.pending_standalone_comment);

        self.register_anchor(anchor_id, start_byte);

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
            seen_value_keys: std::collections::HashSet::new(),
            null_key: None,
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
            let standalone = self.comment_stack.pop().unwrap_or_default();

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
            // `decor.leading_comments`. The `comment` slot is left for
            // inline trailing notes (currently only set by
            // `attach_inline_comment`).
            //
            // Every note in this stack goes to the leading list regardless of its
            // `standalone` flag, and that is deliberate: granit reports a note
            // trailing a marker line with Right placement while meaning the line
            // above it, so routing by the flag would strand such a note at the end
            // of the block body — where it drifts one level per round again
            // (`marker_note_settles_on_the_marker_line` pins this).
            for comment in standalone {
                let decor = meta.decor.get_or_insert_with(Default::default);
                decor.push_leading_comment(comment);
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
            let standalone = self.comment_stack.pop().unwrap_or_default();

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
            // PR #117b: same slot split as the mapping case above, same
            // flag-blind reason.
            for comment in standalone {
                let decor = meta.decor.get_or_insert_with(Default::default);
                decor.push_leading_comment(comment);
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
    fn on_comment_event(&mut self, text: &str, placement: granit_parser::Placement, span: Span) {
        // Trim with YAML's separation set, not `char::is_whitespace`: granit keeps
        // an NBSP as comment content, and `str::trim` would silently eat one at
        // either edge (or delete a comment made of nothing but one).
        let trimmed = text.trim_matches(pyrs_schema::is_yaml_blank);
        // granit does not re-read a comment whose text is empty (a bare `#` or
        // `# `): first parse surfaces an empty `Event::Comment`, the writer emits
        // `# `, and the re-parse then drops it - so the document drifts one stray
        // `# ` line every serialize round (libFuzzer `yaml_roundtrip`
        // crash-0de6be17). A contentless comment carries nothing to preserve, so
        // we do not record it: the AST reflects the re-readable form and
        // serialization is idempotent.
        if trimmed.is_empty() {
            return;
        }
        // Drop code points that cannot sit inside a document at all. A comment has
        // no escape syntax (unlike a double-quoted scalar), so one forbidden
        // character does not cost detail - it makes the *whole* document
        // unparseable: granit hands us decoded text, so a source comment carrying
        // U+FEFF (restricted to the stream's own leading BOM) came back verbatim
        // and our parser then rejected our own output - "a BOM must not appear
        // inside a document" (libFuzzer `yaml_roundtrip` crash-2d14c6f6).
        // Sanitising on ingest keeps the AST the single authoritative, already-safe
        // form, so every serializer site is correct by construction instead of
        // each needing its own filter. Re-trim after filtering: removing a
        // character can expose edge whitespace that granit would strip on re-read,
        // which alone would break idempotence. If nothing readable survives, the
        // note is not recorded at all - the same rule as a contentless comment.
        let text: Arc<str> = if trimmed.chars().all(pyrs_schema::is_yaml_document_char) {
            Arc::from(trimmed)
        } else {
            let cleaned: String = trimmed
                .chars()
                .filter(|&c| pyrs_schema::is_yaml_document_char(c))
                .collect();
            let cleaned = cleaned.trim_matches(pyrs_schema::is_yaml_blank);
            if cleaned.is_empty() {
                return;
            }
            Arc::from(cleaned)
        };
        if is_standalone_placement(&placement) {
            // Append, never overwrite: a stack of comment lines above one key is
            // ordinary input, and a single slot here used to keep only the last
            // of them. See `stacked_comments_above_a_key_all_survive`.
            self.pending_standalone_comment.push(Comment {
                text,
                standalone: true,
            });
        } else if placement == granit_parser::Placement::Right {
            let at = self.span_to_byte_range(&span).start;
            if !self.attach_inline_comment(Arc::clone(&text), at) {
                // The note is not on the previous node's line, so it annotates a
                // node that has not arrived yet: carry it forward as that node's
                // leading note instead of binding it backwards.
                self.pending_standalone_comment.push(Comment {
                    text,
                    standalone: true,
                });
            }
        }
        // Placement is #[non_exhaustive]: other variants are ignored.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::yaml::YamlSchema;

    /// The folded entry's comment survives the null-key fold. `: &b #*<CR>:` lost
    /// `# *` together with the entry the fold removed, leaving `~: ~` — and the fold
    /// never looked like a comment problem, so the loss hid behind a stable text
    /// until the fuzz tier gained a note-survival oracle.
    #[test]
    fn a_folded_null_key_keeps_its_comment() {
        let src = ": &b #*\r:";
        let node = parse(src, YamlSchema::Core).expect("input parses");
        let one = crate::serializer::to_yaml(&node);
        assert!(one.contains('*'), "{src:?} lost the folded note: {one:?}");
        let again = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).unwrap());
        assert_eq!(again, one, "{one:?} must be a fixed point: {again:?}");
    }

    /// A document that carries nothing but comments still carries content. Its
    /// notes used to be stranded in the pending slot — `DocumentEnd` never fires
    /// when there is no node — and vanished, so `#&l<TAB><TAB>:` serialised to
    /// `null`. Found by the fuzz tier's note-survival oracle rather than the drift
    /// oracle, because the lossy output was perfectly stable.
    #[test]
    fn a_comment_only_document_keeps_its_notes() {
        for src in ["#&l\t\t:", "# one\n# two", "# only"] {
            let node = parse(src, YamlSchema::Core).expect("input parses");
            assert!(
                matches!(node, CustomNode::Null { .. }),
                "{src:?} stays a null document: {node:?}"
            );
            let one = crate::serializer::to_yaml(&node);
            for line in src.split(['\r', '\n']) {
                let Some(body) = line.trim_start().strip_prefix('#').map(str::trim) else {
                    continue;
                };
                assert!(!body.is_empty(), "{src:?} has an empty note");
                assert!(one.contains(body), "{src:?} lost {body:?}: {one:?}");
            }
            let again = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).unwrap());
            assert_eq!(again, one, "{one:?} must be a fixed point: {again:?}");
        }
    }

    /// Stacked standalone comments are content: every note line above a key has
    /// to survive ingest and emission, and the emission must already be a fixed
    /// point. The receiver carried the next node's notes in a single slot and
    /// overwrote it, so `# alpha` + `# beta` + `key: 1` came back one note short —
    /// silently, and invisible to the round-trip tier, whose oracle is text
    /// idempotence and therefore passes for any output that is stable and merely
    /// missing a note (libFuzzer `yaml_roundtrip` crash-f8525a9e surfaced the same
    /// shape under explicit-key markers, where it even changed position every
    /// round and lost a note by round 3).
    #[test]
    fn stacked_comments_above_a_key_all_survive() {
        for src in [
            "# alpha\n# beta\nkey: 1\n",
            "# a\n# b\n# c\nkey: 1\n",
            "top:\n  # one\n  # two\n  k: v\n",
            "# head\n- one\n# mid\n- two\n",
        ] {
            let ast = parse(src, YamlSchema::Core).expect("input parses");
            let notes: Vec<&str> = ast.leading_comments().iter().map(|c| &*c.text).collect();
            let one = crate::serializer::to_yaml(&ast);
            let expected_lines = src
                .lines()
                .filter(|line| line.trim_start().starts_with('#'))
                .count();
            let emitted_lines = one
                .lines()
                .filter(|line| line.trim_start().starts_with('#'))
                .count();
            assert_eq!(
                emitted_lines, expected_lines,
                "{src:?} kept {notes:?} but emitted {one:?}"
            );
            let again = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).unwrap());
            assert_eq!(again, one, "{one:?} must be a fixed point: {again:?}");
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-f44eca1d, 36 bytes minimised to 12): a tag
    /// URI that contains `&` next to an anchor let `anchor_name_before` harvest the
    /// tag's ampersand instead of the real anchor, so `&F !-&l ` re-read as anchor
    /// `l`. The name then mutated on every round and emission never reached a fixed
    /// point - and a renamed anchor silently orphans every `*F` alias that referred
    /// to it. Rejecting a `&` whose token opens with `!` pins the name.
    #[test]
    fn anchor_beside_tag_containing_ampersand_round_trips_stably() {
        let input = "!-&l &F";
        let one =
            crate::serializer::to_yaml(&parse(input, YamlSchema::Core).expect("input parses"));
        assert_eq!(
            one, "&F !-&l \n",
            "anchor must keep its name and the tag its URI"
        );
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "emission must be a fixed point");
    }

    /// The same crash at its authoritative bytes, replayed from the committed seed
    /// rather than a hand-copy: the minimised case above is derived, this is what
    /// libFuzzer actually found (36 bytes), and it carries a comment as well as a
    /// tag and anchor, so it also proves the fix did not disturb comment recovery.
    /// Asserted as a property because the exact emission is not what regressed.
    #[test]
    fn former_crash_f44eca1d_reaches_a_fixed_point() {
        let raw =
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-f44eca1d.seed");
        let src = std::str::from_utf8(raw).expect("seed is utf-8");
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(
            two, one,
            "second round must not mutate the first: {one:?} -> {two:?}"
        );
    }

    /// libFuzzer `yaml_roundtrip` (crash-b91536ce, 7 bytes `!y5%7c `): granit
    /// hands the reader the *decoded* tag suffix, so the source tag `!y5%7c`
    /// arrived as `y5|`. The writer emitted that decoded text, but `|` is not a
    /// permitted tag character, so the output no longer parsed at all ("while
    /// scanning a tag, did not find expected whitespace or line break"). Tag
    /// emission now re-percent-encodes URI-illegal characters, which restores a
    /// readable spelling - and the same one every round, so the text is stable.
    #[test]
    fn tag_suffix_with_illegal_uri_chars_round_trips_stably() {
        let crash = String::from_utf8(vec![0x21u8, 0x79, 0x35, 0x25, 0x37, 0x63, 0x20]).unwrap();
        for src in [
            crash,
            "!tag foo".to_string(),
            "key: !a%20b \n".to_string(),
            "key: !t x\n".to_string(),
        ] {
            let node =
                parse(&src, YamlSchema::Core).unwrap_or_else(|e| panic!("{src:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = parse(&once, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("{once:?} must re-parse: {e}\nfor {src:?}"));
            let twice = crate::serializer::to_yaml(&again);
            assert_eq!(once, twice, "tag drift for {src:?}: {once:?} vs {twice:?}");
        }
        // The escaped spelling survives rather than degrading to a bare `|`.
        let node = parse(&crash_input(), YamlSchema::Core).expect("crash input parses");
        let once = crate::serializer::to_yaml(&node);
        assert!(once.contains("%7c"), "tag not re-encoded: {once:?}");
        assert!(!once.contains("!y5|"), "illegal raw `|` emitted: {once:?}");
    }

    fn crash_input() -> String {
        String::from_utf8(vec![0x21u8, 0x79, 0x35, 0x25, 0x37, 0x63, 0x20]).unwrap()
    }

    /// libFuzzer `yaml_roundtrip` (crash-e92ce66f, 43 bytes `!5y…%2c `): granit
    /// decodes that tag to a suffix carrying a literal `,`, but `,` is a flow
    /// indicator, so `is_tag_char` refuses it and the suffix scan stops there — and
    /// at flow level 0 the scanner then requires a blank or a line break. The writer
    /// re-emitted the decoded `,` as it stood, so `to_yaml` produced text our own
    /// reader rejects (`InvalidTagTerminator`). Same root cause as the 8-byte
    /// `!5%2cy7 ` shape: the write set was taken from RFC 3986's punctuation instead
    /// of the reader's character class, so it allowed four characters
    /// (`,` `[` `]` `!`) that end the scan. Shorthand tags now encode exactly those.
    #[test]
    fn tag_suffix_flow_indicators_round_trips_stably() {
        // (source, the escape the emitted tag must carry instead of the raw char)
        for (src, marker) in [
            ("!a%2cb ", "%2c"),
            ("!5%2cy7 ", "%2c"),
            ("!a%5bb ", "%5b"),
            ("!a%5db ", "%5d"),
            ("!a%21b ", "%21"),
            ("k: !a%2cb v\n", "%2c"),
            ("- !a%2cb\n", "%2c"),
        ] {
            let node =
                parse(src, YamlSchema::Core).unwrap_or_else(|e| panic!("{src:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = parse(&once, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("{once:?} must re-parse: {e}\nfor {src:?}"));
            let twice = crate::serializer::to_yaml(&again);
            assert_eq!(twice, once, "tag drift for {src:?}: {once:?} vs {twice:?}");
            assert!(
                once.contains(marker),
                "flow indicator not re-encoded for {src:?}: {once:?}"
            );
        }

        // The authoritative bytes, replayed from the committed seed rather than a
        // hand-copy: it pairs the long suffix with a `'`, which is a legal tag
        // character, so it also proves the tightening did not over-encode.
        let raw =
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-e92ce66f.seed");
        let src = std::str::from_utf8(raw).expect("seed is utf-8");
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        assert!(
            one.contains("%2c") && !one.contains("'rrr,"),
            "raw comma emitted: {one:?}"
        );
        assert!(one.contains('\''), "legal `'` must stay raw: {one:?}");
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "second round must not mutate the first");
    }

    /// The other half of the same read map. A verbatim `!<uri>` is scanned with
    /// `is_uri_char`, where the flow indicators are legal and decode to themselves,
    /// so they must stay raw: `!<tag:yaml.org,2002:str>` is ordinary YAML, and
    /// percent-spelling it would trade a crash for needless source drift on every
    /// verbatim tag. Characters the verbatim scan really refuses (a space, `>`) are
    /// still encoded.
    #[test]
    fn verbatim_tag_keeps_its_uri_spelling() {
        let input = "k: !<tag:yaml.org,2002:str> v\n";
        let one =
            crate::serializer::to_yaml(&parse(input, YamlSchema::Core).expect("input parses"));
        assert!(
            one.contains("!<tag:yaml.org,2002:str>"),
            "a legal verbatim URI must not be re-encoded: {one:?}"
        );
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "verbatim tag must be a fixed point");

        // `!<a b>` cannot be read at all (the scan stops at the space and the `>`
        // is then missing), so the space has to arrive escaped and leave escaped.
        let src = "k: !<a%20b> v\n";
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        assert!(one.contains("!<a%20b>"), "space in a verbatim URI: {one:?}");
    }

    /// libFuzzer `yaml_roundtrip` (crash-512814, 5 bytes: a stream's own BOM then a
    /// single NBSP): the first round emitted the NBSP scalar and the second emitted
    /// `null`. The empty-document fast path asked `str::trim().is_empty()`, and
    /// Rust's `trim` is Unicode-based — it also strips NBSP, which YAML treats as
    /// ordinary content (granit's blank set is SP and TAB only). The scan now uses
    /// that set, so a NBSP-only document stays a string scalar.
    #[test]
    fn nbsp_only_document_is_a_scalar_not_null() {
        let raw = include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-512814.seed");
        let src = std::str::from_utf8(raw).expect("seed is utf-8");
        let node = parse(src, YamlSchema::Core).expect("input parses");
        assert!(
            matches!(&node, CustomNode::Scalar { value, .. } if value.as_ref() == "\u{a0}"),
            "a NBSP document must stay a string scalar, not null"
        );
        // Quoting is the engine's standing policy for any value containing NBSP
        // (`needs_double_quoted` lists U+00A0); before the fix that rule was never
        // reached, because the value mis-resolved to `Null` and plain emission
        // looked safe — which is exactly what lost the scalar on re-read.
        let one = crate::serializer::to_yaml(&node);
        assert_eq!(one, "\"\u{a0}\"\n", "NBSP document emission");
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "second round must not mutate the first");

        // The same trap one level in: a mapping value that is only a NBSP.
        let src = "k: \u{a0}\n";
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        assert!(!one.contains("null"), "NBSP value became null: {one:?}");
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "NBSP value must be a fixed point");
    }

    /// The same wrong vocabulary sat in two other text scans. granit keeps NBSP
    /// inside an anchor name and inside comment text, so a Unicode-based blank test
    /// truncated `&a<NBSP>b` to `a` — and a renamed anchor silently orphans every
    /// alias that refers to it, the same data-loss class as #265 — and ate an NBSP
    /// at either edge of a note. Both scans now ask YAML's own question.
    #[test]
    fn nbsp_survives_in_anchor_names_and_comment_text() {
        let src = "&a\u{a0}b v";
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        assert!(
            one.contains("&a\u{a0}b"),
            "anchor name truncated at the NBSP: {one:?}"
        );
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "anchor name must be a fixed point");

        let src = "#\u{a0}x\nk: v\n";
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        assert!(
            one.contains("\u{a0}x"),
            "leading NBSP eaten from the comment: {one:?}"
        );
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "comment text must be a fixed point");
    }

    /// The same Unicode-trim trap sat one level deeper, in schema resolution:
    /// `resolve_core_type` trimmed with `str::trim`, so a value whose content is
    /// only NBSP resolved to `Null`, and `<NBSP>42` resolved to the *integer* 42 —
    /// silent type corruption either way. The crash shape is the writer side of
    /// it: `needs_double_quoted` returns early for any value that resolves to a
    /// non-string type, so a multi-line NBSP scalar skipped quoting, went out with
    /// raw line breaks, and those collapse on re-read — emission never settled
    /// (libFuzzer `yaml_roundtrip` crash-b44481b2, 7 bytes: NBSP CR CR CR NBSP).
    #[test]
    fn nbsp_is_content_not_separation_for_schema_resolution() {
        use pyrs_schema::schema::core_type_is_non_string;

        // Separation the Unicode way is not separation the YAML way.
        assert!(
            !core_type_is_non_string("\u{a0}"),
            "a NBSP-only value resolved to a non-string type (null)"
        );
        assert!(
            !core_type_is_non_string("\u{a0}42"),
            "a NBSP-prefixed 42 resolved to a non-string type (int)"
        );
        assert!(
            !core_type_is_non_string("\u{2028}7"),
            "U+2028 treated as separation"
        );
        // Real YAML separation still is, so every ordinary spelling holds.
        assert!(core_type_is_non_string("42"));
        assert!(core_type_is_non_string("  42  "));
        assert!(core_type_is_non_string("\t42\t"));
        assert!(core_type_is_non_string(""));
        assert!(core_type_is_non_string("~"));
        assert!(core_type_is_non_string("true"));
        // YAML 1.1's legacy booleans read the same edge.
        assert!(
            matches!(
                pyrs_schema::schema::resolve_yaml11_type("\u{a0}yes"),
                pyrs_schema::types::YamlType::Str(_)
            ),
            "`<NBSP>yes` resolved to a legacy boolean"
        );

        // The crash input now reaches a fixed point: the value is quoted, so its
        // line breaks survive as escapes instead of folding into one space.
        let raw =
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-b44481b2.seed");
        let src = std::str::from_utf8(raw).expect("seed is utf-8");
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        assert!(
            one.contains('"'),
            "multi-line scalar emitted unquoted: {one:?}"
        );
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(
            two, one,
            "second round must not mutate the first: {one:?} -> {two:?}"
        );
    }

    /// granit spells a lone `!` tag as an empty handle whose suffix is `!`. The
    /// encoder must not turn that sentinel into `%21` — the output would lose its
    /// leading `!` and stop being a tag token entirely.
    #[test]
    fn lone_bang_tag_still_emits_a_bang() {
        for src in ["!\n", "k: !\n", "! \n"] {
            let Ok(node) = parse(src, YamlSchema::Core) else {
                continue; // shapes the reader refuses are not our contract
            };
            let one = crate::serializer::to_yaml(&node);
            assert!(
                !one.contains("%21"),
                "lone `!` tag escaped: {one:?} (from {src:?})"
            );
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-04fddeb8): the anchor name granit never
    /// reports is read back by scanning left from the node's own content, and a
    /// standalone comment sitting between that anchor and the content donated its
    /// own `&l` — so the mapping was anchored `&~:` on one round and `&l` on the
    /// next. A renamed anchor orphans every alias that used the real name, so this
    /// is the same data-loss class as #265, reached from the other token that may
    /// legally contain `&`. Recovery now refuses a `&` whose line is already inside
    /// a comment, exactly as it refuses one inside a tag.
    #[test]
    fn anchor_name_is_not_donated_by_a_comment_line() {
        let src = "chi&&&: &~:\n  # &l\n  y: 2\n";
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        assert!(
            one.contains("&~:"),
            "anchor name lost to the comment below it: {one:?}"
        );
        // The note itself stays — the fix rejects the comment's `&` as an anchor
        // *candidate*, it does not throw the comment away.
        assert!(one.contains("# &l"), "comment was dropped: {one:?}");
        assert_eq!(
            one, "chi&&&: &~:\n  # &l\n  y: 2\n",
            "emission must keep the real anchor and the note in place"
        );
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(
            two, one,
            "anchor name must be a fixed point: {one:?} -> {two:?}"
        );
    }

    /// libFuzzer `yaml_roundtrip` (crash-89d81d99, 5 bytes `>+8<CR>#`, and
    /// crash-b5dcc38f, 55 bytes): a block scalar with an **empty** body cannot carry
    /// a chomping indicator, because granit reports the default chomping when it
    /// re-reads a header that has no content to act on. The first round wrote `>+`
    /// (the explicit indentation indicator in the source is what made the reader
    /// report `Keep` that once) and the second wrote `>`, so `to_yaml` never reached
    /// a fixed point. The writers now drop the indicator for an empty body — the
    /// same "emit the re-readable form" rule they already apply to the indentation
    /// indicator there, and nothing is lost because an empty body has no trailing
    /// break to keep or strip.
    #[test]
    fn empty_block_header_reaches_a_fixed_point() {
        let first =
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-89d81d99.seed");
        let src = std::str::from_utf8(first).expect("seed is utf-8");
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        assert_eq!(
            one, ">\n\n",
            "empty folded body must be written without a `+`"
        );
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "second round must not mutate the first");

        let second =
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-b5dcc38f.seed");
        let src = std::str::from_utf8(second).expect("seed is utf-8");
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        assert!(
            !one.contains("|+\n") && !one.contains(">+\n"),
            "an empty body still advertised a chomping indicator: {one:?}"
        );
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(
            two, one,
            "block header must be a fixed point: {one:?} -> {two:?}"
        );

        // A body that is not empty keeps its `+` — the rule is scoped to empty.
        let src = "k: |+\n  x\n\n";
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        assert!(
            one.contains("|+"),
            "non-empty Keep body lost its indicator: {one:?}"
        );
    }

    /// A YAML mapping holds exactly one null key. `IndexMap` compares whole
    /// `CustomNode`s and two spellings of null differ in metadata, so an empty key
    /// and a `~` key both stayed and both rendered as `~:`; the reader folded them
    /// on re-parse, so the document lost a line every round (libFuzzer
    /// `yaml_roundtrip` crash-00e31785, minimised to 9 bytes `: &b #*\r:`). Ingest
    /// folds them now — which is what re-reading the emitted text produces anyway —
    /// while yaml-test-suite 2JQS still parses without error.
    #[test]
    fn null_keys_fold_to_one_entry() {
        // 2JQS: duplicate null keys are accepted, and the value that survives is
        // the one a reader sees when it folds them: the last.
        let node = parse(": a\n: b\n", YamlSchema::Core).expect("2JQS must parse");
        {
            let CustomNode::Mapping { pairs, .. } = &node else {
                panic!("expected a mapping, got {node:?}");
            };
            assert_eq!(pairs.len(), 1, "two null keys must fold to one entry");
        }
        assert_eq!(
            crate::serializer::to_yaml(&node),
            "~: b\n",
            "folded null key must carry the last value"
        );

        // Mixed spellings fold too, and the rest of the mapping keeps its order.
        let src = "~: 1\n: 2\nk: 3\n";
        let node = parse(src, YamlSchema::Core).expect("mixed null spellings parse");
        let one = crate::serializer::to_yaml(&node);
        assert_eq!(one, "~: 2\nk: 3\n", "null spellings must unify in place");
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "folded mapping must be a fixed point");

        // The crash input itself, at its authoritative bytes.
        let src = ": &b #*\r:";
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(
            two, one,
            "anchored null-key mapping must settle: {one:?} -> {two:?}"
        );

        // The fold sits inside the strict path only: a caller who asked to keep
        // duplicate keys still gets no error, and still sees the last value (both
        // null spellings normalize to the same node, so `IndexMap` overwrite is what
        // collapses them there — the fold is for the pair the node keys keep apart).
        let node = parse_with_options("~: 1\n: 2\n", true, YamlSchema::Core, 1000, true)
            .expect("duplicates allowed on request");
        assert_eq!(
            crate::serializer::to_yaml(&node),
            "~: 2\n",
            "the lenient path must agree on the surviving value"
        );

        // A quoted null spelling is a *string* key, so it survives beside a real
        // null key instead of being folded into it — the distinction the dialect
        // property tests caught when `is_null_key` looked at text only.
        let src = "\"\": 1\nNULL: 2\n~: 3\n";
        let node = parse(src, YamlSchema::Core).expect("string and null keys parse");
        assert_eq!(
            crate::serializer::to_yaml(&node),
            "\"\": 1\n~: 3\n",
            "only the plain null spellings may fold; the quoted key stays"
        );
    }

    /// libFuzzer `yaml_roundtrip` crash-ac5d9043 (18 bytes, minimised by `cargo fuzz
    /// tmin` to `? ? ? #~`): a note trailing a line of explicit-key markers is
    /// reported by granit one level shallower than the node it was attached to, so
    /// writing it inside the key body let the note climb a level every serialization
    /// and emission never settled. The serializer now brings such a note up to the
    /// marker line that owns it — the spelling both sides agree on, and the one a
    /// single-marker line already produced. crash-0e1c4378 looked like a second
    /// carrier of exactly this shape, and needed this hoist too — but the hoist alone
    /// did not settle it; the rest was a note-binding geometry fixed on the ingest
    /// side, see `a_dedented_note_does_not_trail_a_deeper_node`.
    #[test]
    fn marker_note_settles_on_the_marker_line() {
        let raw =
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-ac5d9043.seed");
        let seed = std::str::from_utf8(raw).expect("seed is utf-8");
        for src in [seed, "? ? ? #~"] {
            let node =
                parse(src, YamlSchema::Core).unwrap_or_else(|e| panic!("{src:?} parses: {e}"));
            let one = crate::serializer::to_yaml(&node);
            assert_eq!(
                one.lines()
                    .filter(|line| line.trim_start().starts_with('#'))
                    .count(),
                1,
                "the note must be kept exactly once: {one:?}"
            );
            assert!(
                one.starts_with("# "),
                "the hoisted note must open the document: {one:?}"
            );
            let again = parse(&one, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("{one:?} re-parses: {e}\nfor {src:?}"));
            let two = crate::serializer::to_yaml(&again);
            assert_eq!(
                two, one,
                "marker note must be a fixed point: {one:?} -> {two:?}"
            );
        }

        // Blast radius: a note granit lexed on its own line is reported where it
        // sits, so hoisting that one would move a note that already round-trips.
        // This shape was a fixed point before the change and must stay one.
        let src = "k:\n  ?\n    # c\n    ? a\n    : b\n";
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("input parses"));
        assert_eq!(
            one, "k:\n  # c\n  ?\n    a: b\n  :\n    ~\n",
            "an own-line note inside a key body must not be hoisted"
        );
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "own-line note must stay a fixed point");
    }

    /// The two lifts a marker line performs have to compose. A chain of `?` markers
    /// can hold a note on the key node *and* a second one riding the first entry of
    /// the key body, and the reader reports both at the marker's own level. Taking
    /// only the key's note - which the `if`/`else if` chain did, because the key had
    /// one - wrote the body note one indent deeper, so the first emission was not the
    /// fixed point and the note climbed a level on the re-read (libFuzzer
    /// `yaml_roundtrip` crash-456176be, 40 bytes as found: `?` + ` ### standab:` +
    /// ` ?` + `  # ! y%% yam2:#l: tr` + `  ~: ~`, whose source tree keeps the first
    /// note on the key mapping and the second on its inner `~` key).
    #[test]
    fn a_marker_carries_both_its_own_note_and_its_bodys_first_note() {
        let raw =
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-456176be.seed");
        let src = std::str::from_utf8(raw).expect("seed is utf-8");
        let node = parse(src, YamlSchema::Core).expect("input parses");

        let one = crate::serializer::to_yaml(&node);
        let lines: Vec<&str> = one.lines().collect();
        let notes: Vec<&str> = one.lines().filter(|line| line.starts_with("# ")).collect();
        assert_eq!(notes.len(), 2, "both notes survive as note lines: {one:?}");
        assert!(
            lines
                .iter()
                .all(|line| !line.starts_with(' ') || !line.trim_start().starts_with('#')),
            "neither note may sit inside a body: {one:?}"
        );
        assert_eq!(
            lines[0], "# ## standab:",
            "the key's own note opens the document: {one:?}"
        );
        assert_eq!(
            lines[1], "# ! y%% yam2:#l: tr",
            "the body's first note follows it at the same level, above the marker: {one:?}"
        );
        assert_eq!(lines[2], "?", "and only then comes the marker: {one:?}");

        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "one emission has to reach the fixed point");
    }

    /// A marker chain can carry more than one stack of notes along its first-pair-key
    /// spine, and granit reports every one of them at the outermost marker's level.
    /// Hoisting only the first stack left a second one a level deeper, so the re-read
    /// lifted it too and the emission settled only on its second round. The tree names
    /// the shape: `? ? ? ##` + 60 CR + `  #!!"#~` keeps `#` on the middle mapping and
    /// `!!"#~` on the innermost `~` key - and the 99-byte crash-c9031de4 carries the
    /// same layout three markers deep (libFuzzer `yaml_roundtrip`).
    #[test]
    fn every_note_on_the_marker_spine_lifts_to_the_marker_line() {
        let deep =
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-f8525a9e.seed");
        let wider =
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-c9031de4.seed");
        for (name, raw) in [
            ("f8525a9e", deep.as_slice()),
            ("c9031de4", wider.as_slice()),
        ] {
            let src = std::str::from_utf8(raw).expect("seed is utf-8");
            let node = parse(src, YamlSchema::Core).unwrap_or_else(|e| panic!("{name}: {e}"));
            let one = crate::serializer::to_yaml(&node);
            let lifted = one.lines().filter(|line| line.starts_with("# ")).count();
            assert!(
                lifted >= 2,
                "both stacks must sit at column 0: {name} -> {one:?}"
            );
            assert!(
                !one.lines()
                    .any(|line| line.starts_with(' ') && line.trim_start().starts_with('#')),
                "no note may be left inside a body: {name} -> {one:?}"
            );
            let two = crate::serializer::to_yaml(
                &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
            );
            assert_eq!(two, one, "one emission must reach the fixed point: {name}");
        }
    }

    /// The same accounting for a container's tag header: the container's own stack and
    /// the body's first stack both belong above the header, in that order, and one
    /// emission lands there (crash-fbc8f2ae, 32 bytes - the smallest carrier of this
    /// shape, produced by the discovery window after the header guard was re-derived).
    #[test]
    fn both_note_stacks_land_above_a_tag_header_in_one_round() {
        let raw =
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-fbc8f2ae.seed");
        let src = std::str::from_utf8(raw).expect("seed is utf-8");
        let node = parse(src, YamlSchema::Core).expect("input parses");
        let one = crate::serializer::to_yaml(&node);
        assert_eq!(
            one, "# yr-\n# y?\n# yr-\n# yrrr\n!5b54? \n~: ~\n",
            "both stacks above the header, in source order"
        );
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "one emission has to reach the fixed point");
    }

    /// A note trailing a simple key used to vanish. granit reports it on the *key*
    /// node, but once the pair is written as `key: value` that position has no
    /// spelling, so `? a # note` + `: b` serialized to `a: b` — a silent comment loss
    /// the round-trip tier cannot see (the text is stable, it is simply short a note).
    /// It now rides the one slot a reader reports it back from, after the value, and
    /// the line is a fixed point there.
    #[test]
    fn a_note_trailing_a_key_survives() {
        let src = "? a # note\n: b\n";
        let node = parse(src, YamlSchema::Core).expect("input parses");
        let CustomNode::Mapping { pairs, .. } = &node else {
            panic!("expected a mapping, got {node:?}");
        };
        let (key, value) = pairs.iter().next().expect("one pair");
        assert!(
            matches!(key, CustomNode::Scalar { meta, .. }
                if meta.comment.as_ref().is_some_and(|c| c.text.as_ref() == "note")),
            "the source tree keeps the note on the key node"
        );
        assert!(
            value.comment().is_none(),
            "the value carries no note of its own"
        );

        let one = crate::serializer::to_yaml(&node);
        assert_eq!(
            one, "a: b  # note\n",
            "the note must be emitted, not dropped"
        );
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "the borrowed slot must be a fixed point");

        // One line has one trailing slot: a value with a note of its own keeps it,
        // so the key note can never overwrite or duplicate it.
        let src = "? a # kn\n: b # vv\n";
        let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("parses"));
        assert_eq!(one, "a: b  # vv\n", "the value's own note wins");
    }

    /// A note granit reports in a *container's* inline slot has to be written where a
    /// reader reports it from. When the pair ending the body has a value with no text
    /// (`:<TAB>!-<CR>... #-o` → a bare `!-`), the value stays pending to the scanner,
    /// so a note line below it is handed back as that value's leading note — the shape
    /// that made libFuzzer `yaml_roundtrip` crash-11ced252 settle only on its second
    /// round. The first fix here wrote the note inline on the pair line instead
    /// (`~: !-   # -o`), which settles for this document too; the uniform rule now
    /// emits the shape a re-read reports, so one emission is enough — and note the
    /// routed form is exactly what this test first recorded as round two's output.
    /// Measured: inlining the note is stable only while the container holds a single
    /// note; with two notes on the same shape every tag spelling drifted unless the
    /// rule stopped keying on the tag (see `crash-cf49fe85`'s test in `serializer.rs`).
    #[test]
    fn a_containers_inline_note_after_a_text_less_value_settles_at_once() {
        let src = ":\t!-\r... #-o\n";
        let node = parse(src, YamlSchema::Core).expect("input parses");
        let CustomNode::Mapping { pairs, meta, .. } = &node else {
            panic!("expected a mapping, got {node:?}");
        };
        assert_eq!(
            pairs.len(),
            1,
            "one pair: the empty key and its tagged value"
        );
        assert!(
            meta.comment
                .as_ref()
                .is_some_and(|c| !c.standalone && c.text.as_ref() == "-o"),
            "granit reports the note as the mapping's own inline slot: {node:?}"
        );
        let (_, value) = pairs.iter().next().expect("the pair");
        assert!(
            value.comment().is_none() && value.leading_comments().is_empty(),
            "in the source the note belongs to the container, not to the value"
        );

        let one = crate::serializer::to_yaml(&node);
        assert_eq!(
            one, "~:\n  # -o\n  !- \n",
            "the note goes where a re-read reports it — inside the value with no text"
        );
        assert_eq!(
            one.matches('#').count(),
            1,
            "the note is moved, not duplicated: {one:?}"
        );
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(two, one, "one emission has to reach the fixed point");
    }

    /// The other half of the same rule: a note may only borrow a line that can host
    /// a trailing note. A block scalar's body line cannot — `  y  # cn` would read
    /// back as the literal `y  # cn`, silently changing the value. The writer tracks
    /// slot ownership where lines are written rather than guessing from the text, so
    /// this shape falls back to a note line and the content stays intact.
    #[test]
    fn a_block_scalar_body_never_borrows_the_containers_note() {
        let mut node = parse("a: |\n  x\n  y\n", YamlSchema::Core).expect("input parses");
        let CustomNode::Mapping { meta, .. } = &mut node else {
            panic!("expected a mapping, got {node:?}");
        };
        meta.comment = Some(Comment {
            text: Arc::from("cn"),
            standalone: false,
        });

        let one = crate::serializer::to_yaml(&node);
        assert_eq!(
            one, "a: |\n  x\n  y\n# cn\n",
            "the note must not be appended into the block body"
        );
        assert!(
            !one.contains("y  #"),
            "a note inside the body would become content: {one:?}"
        );
        let again = parse(&one, YamlSchema::Core).expect("emitted document re-parses");
        let CustomNode::Mapping { pairs, meta, .. } = &again else {
            panic!("expected a mapping, got {again:?}");
        };
        let (_, value) = pairs.iter().next().expect("the `a` pair");
        assert!(
            matches!(value, CustomNode::Scalar { value, .. } if value.as_ref() == "x\ny\n"),
            "the block scalar keeps its content: {again:?}"
        );
        assert!(
            meta.comment
                .as_ref()
                .is_some_and(|c| c.text.as_ref() == "cn"),
            "and the fallback line still reports the note back to the container: {again:?}"
        );
    }

    /// A note that starts a dedented line cannot trail a node living deeper than
    /// that line. granit spans the nested mapping through the line break ending
    /// its own line, so the note's byte landed on the far side of `range.end` and
    /// the old same-line test saw nothing between them; the note homed on the
    /// deeper mapping, the writer spilled it inside the block, the re-read gave it
    /// to the shallower next entry, and ownership climbed a level every round
    /// (libFuzzer `yaml_roundtrip` crash-0e1c4378, minimised to `b:\n ?\n? #i` and
    /// committed as `former-crash-0e1c4378.seed`).
    #[test]
    fn a_dedented_note_does_not_trail_a_deeper_node() {
        let src = "b:\n ?\n? #i";
        let node = parse(src, YamlSchema::Core).expect("input parses");
        let CustomNode::Mapping { pairs, .. } = &node else {
            panic!("expected a mapping, got {node:?}");
        };
        let (_, nested) = pairs.iter().next().expect("the `b` pair");
        assert!(
            nested.comment().is_none(),
            "the dedented note must not home on the deeper mapping: {node:?}"
        );

        let one = crate::serializer::to_yaml(&node);
        assert!(one.contains('#'), "the note has to survive: {one:?}");
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(
            two, one,
            "dedented note must be a fixed point: {one:?} -> {two:?}"
        );
    }

    /// Backing the candidate's end over swallowed blanks must not *over*-refuse: a
    /// note on the last line of a multi-line node still trails that node. Had the
    /// rule been written as "same line as the node's first byte" this input would
    /// have flipped the note onto the following entry and drifted.
    #[test]
    fn a_note_on_a_multi_line_nodes_last_line_still_trails_it() {
        let src = "a: [1,\n  2] # note\nb: 2\n";
        let node = parse(src, YamlSchema::Core).expect("input parses");
        let CustomNode::Mapping { pairs, .. } = &node else {
            panic!("expected a mapping, got {node:?}");
        };
        let (_, sequence) = pairs.iter().next().expect("the `a` pair");
        assert!(
            matches!(sequence, CustomNode::Sequence { .. }),
            "the value stays a flow sequence: {sequence:?}"
        );
        assert!(
            sequence.comment().is_some(),
            "the note stays on the flow sequence it annotated: {node:?}"
        );
        let one = crate::serializer::to_yaml(&node);
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(
            two, one,
            "multi-line trailing note must be a fixed point: {one:?} -> {two:?}"
        );
    }

    /// libFuzzer `yaml_roundtrip` crash-105de752 (47 bytes) went CLEAN with the same
    /// fix, but the attribution was *measured* rather than assumed: disabling the
    /// blank-trim makes it crash again next to crash-0e1c4378, so the two inputs
    /// share one root cause instead of merely one failing assertion — the
    /// distinction the ledger owes after the earlier "confirmed twice" call. Read
    /// from the committed seed so the test cannot drift from the bytes that
    /// actually crashed.
    #[test]
    fn the_second_carrier_input_shares_the_trimmed_span_end_fix() {
        let artifact =
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-105de752.seed");
        let src = String::from_utf8(artifact.to_vec()).expect("seed is valid utf-8");
        let node = parse(&src, YamlSchema::Core).expect("input parses");
        let one = crate::serializer::to_yaml(&node);
        let two = crate::serializer::to_yaml(
            &parse(&one, YamlSchema::Core).expect("emitted document re-parses"),
        );
        assert_eq!(
            two, one,
            "{src:?} must reach a fixed point: {one:?} -> {two:?}"
        );
    }

    /// libFuzzer `yaml_roundtrip` (crash-2d14c6f6, 55 bytes): U+FEFF is *restricted*
    /// to a stream's own leading byte-order mark, yet granit surfaces it inside
    /// decoded comment text, and our `anchor_name_before` text re-scanner swept it
    /// into an anchor name too. Both positions are emitted bare (`# note`,
    /// `&name`) with no escape syntax available, so re-emitting it produced output
    /// our own parser rejected outright ("a BOM must not appear inside a
    /// document"). Comment and anchor text are now filtered to in-document
    /// characters on ingest, so the AST is the single already-safe form.
    #[test]
    fn unrepresentable_code_points_never_reach_emitted_text() {
        // The pinned artifact bytes, read straight from the regression seed so this
        // test cannot drift from the input that actually failed.
        let artifact =
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-2d14c6f6.seed");
        let shapes = [
            String::from_utf8(artifact.to_vec()).expect("seed is valid utf-8"),
            "key: v # a\u{feff}b\n".to_string(),
            "# t\u{feff}u\nkey: v\n".to_string(),
            "\u{feff}key: v\n".to_string(),
        ];
        for src in shapes {
            let node =
                parse(&src, YamlSchema::Core).unwrap_or_else(|e| panic!("{src:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = parse(&once, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("{once:?} must re-parse: {e}\nfor {src:?}"));
            let twice = crate::serializer::to_yaml(&again);
            assert_eq!(once, twice, "BOM drift for {src:?}: {once:?} vs {twice:?}");
            assert!(
                !once.contains('\u{feff}'),
                "a BOM leaked into emitted text for {src:?}: {once:?}"
            );
        }
        // The note must survive with its readable text, not be dropped wholesale.
        let kept =
            crate::serializer::to_yaml(&parse("key: v # a\u{feff}b\n", YamlSchema::Core).unwrap());
        assert!(
            kept.contains("# ab"),
            "comment text lost rather than cleaned: {kept:?}"
        );
    }

    /// libFuzzer `yaml_roundtrip` (crash-aee06aca, 65 bytes, minimized to 12:
    /// `- :\u{feff}:\n- #e`): a note on the dash line of a sequence item whose own
    /// content is empty was bound *backwards* onto the previous item's value, so
    /// the writer spilled it inside that item's block while the re-read bound it
    /// to the next item - ownership flipped every round. A trailing note now
    /// travels forward when it sits on a later line than its backwards candidate.
    /// Two shapes must keep binding inline: a normal same-line note, and a note
    /// on a block scalar's header line, where granit spans the node at its
    /// *content* so the note's line precedes the node's range (regression pinned
    /// by `block_scalar_emission_is_closed_under_reparse`).
    #[test]
    fn comment_line_ownership_is_stable_across_rounds() {
        let minimized = String::from_utf8(vec![
            0x2Du8, 0x20, 0x3A, 0xEF, 0xBB, 0xBF, 0x3A, 0x0A, 0x2D, 0x20, 0x23, 0x65,
        ])
        .unwrap();
        for (src, note) in [
            (minimized, "e"),
            ("- :\u{feff}:\n- #e\n- x: 1\n".to_string(), "e"),
            ("- a: 1\n  # c\n- 2\n".to_string(), "c"),
            ("key: value # own line note\n".to_string(), "own line note"),
            (
                "base: &b\n  x: 2  f,1\n:&hcild:\n  <<: *b\n  y: |  # inlEEE\n".to_string(),
                "inlEEE",
            ),
        ] {
            let node =
                parse(&src, YamlSchema::Core).unwrap_or_else(|e| panic!("{src:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = parse(&once, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            let twice = crate::serializer::to_yaml(&again);
            assert_eq!(
                once, twice,
                "comment-ownership drift for {src:?}: {once:?} vs {twice:?}"
            );
            // The note must survive somewhere, not be silently dropped.
            assert!(once.contains(note), "comment lost for {src:?}: {once:?}");
        }
    }

    /// libFuzzer `yaml_roundtrip` crash-7918272c (11 bytes) and crash-ce106ccc
    /// (69 bytes): a root node that carries only properties — a bare tag, a bare
    /// anchor — never met its note. granit reports that note `Right` (trailing) but
    /// delivers it *before* the `Scalar` event for `!x # note`, and *after*
    /// `DocumentEnd` for `!m` CR `...` SP `# -o`; both orders used to end with the
    /// note discarded, the first because `attach_inline_comment` claimed success while
    /// nothing was attached, the second because the note was bound back onto the
    /// finished root as an inline note whose own re-read homes it as a leading note.
    /// Every case here must keep its text **and** settle in one emission.
    #[test]
    fn a_note_beside_a_property_only_root_survives_and_settles() {
        for (src, expected) in [
            ("!x # note\n", "# note\n!x \n"),
            ("&a # note\n", "# note\n&a ~\n"),
            ("!m   # -o\n", "# -o\n!m \n"),
            ("!m\r... #-o\n", "# -o\n!m \n"),
            (
                "!###0 ##################################################, #######&b #",
                "# #################################################, #######&b #\n!###0 \n",
            ),
        ] {
            let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("parses"));
            assert_eq!(one, expected, "the note has to come back: {src:?}");
            let two = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).expect("reparses"));
            assert_eq!(
                two, one,
                "one emission is the fixed point: {one:?} -> {two:?}"
            );
        }

        // The same two inputs as committed seeds, so a regression cannot slip past
        // the tier that found them.
        for raw in [
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-7918272c.seed")
                as &[u8],
            include_bytes!("../../../../fuzz/seeds/yaml_roundtrip/former-crash-ce106ccc.seed")
                as &[u8],
        ] {
            let src = std::str::from_utf8(raw).expect("seed is utf-8");
            let one = crate::serializer::to_yaml(&parse(src, YamlSchema::Core).expect("parses"));
            let two = crate::serializer::to_yaml(&parse(&one, YamlSchema::Core).expect("reparses"));
            assert_eq!(two, one, "seed must settle at once: {one:?} -> {two:?}");
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-96fa252c, 12 bytes `&"\n-\r... #-o`):
    /// the writer spills a block container's inline note onto its own trailing
    /// line, but that shape re-reads as an EOF standalone comment with no node
    /// after it, so the receiver used to strand and drop it — the second
    /// serialize lost `# -o`. A still-pending note is now flushed onto the
    /// document root, which is exactly where the writer read it from, so the
    /// round trip is stable *and* keeps the comment.
    #[test]
    fn eof_trailing_comment_round_trips_stably() {
        let crash = String::from_utf8(vec![
            0x26u8, 0x22, 0x0A, 0x2D, 0x0D, 0x2E, 0x2E, 0x2E, 0x20, 0x23, 0x2D, 0x6F,
        ])
        .unwrap();
        for src in [
            crash,
            "&\" \n- ~\n# -o\n".to_string(),
            "a: 1\n# trailing note\n".to_string(),
            "- 1\n- 2\n# last\n".to_string(),
        ] {
            let node =
                parse(&src, YamlSchema::Core).unwrap_or_else(|e| panic!("{src:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = parse(&once, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            let twice = crate::serializer::to_yaml(&again);
            assert_eq!(
                once, twice,
                "EOF-comment drift for {src:?}: {once:?} != {twice:?}"
            );
            // The note must genuinely survive, not merely become stably absent.
            let key = if src.contains("trailing note") {
                "trailing note"
            } else if src.contains("last") {
                "last"
            } else {
                "-o"
            };
            assert!(once.contains(key), "comment lost for {src:?}: {once:?}");
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-cad17b2b, 36 bytes): a mapping whose key
    /// contains `|` (a plain `k:yam  |1` or a quoted `"k:yam  |1"`) and whose value
    /// is a block scalar. An upward line scan used to land on the key's `|` and
    /// parse a bogus indentation indicator, so the block drifted `|` -> `|1` across
    /// rounds. `detect_block_header` is now byte-anchored to the content span and
    /// takes the first sigil whose tail satisfies the block-header grammar (only
    /// digit/chomping then end-of-line/comment), so the key's `|` is rejected.
    #[test]
    fn block_header_after_quoted_key_pipe() {
        let bytes: &[u8] = &[
            0x6b, 0x3a, 0x79, 0x61, 0x6d, 0x20, 0x20, 0x7c, 0x31, 0x3a, 0x20, 0x7c, 0x32, 0x0d,
            0x20, 0x20, 0x78, 0x7c, 0x26, 0x22, 0x2f, 0x26, 0x32, 0x0d, 0x20, 0x20, 0x58, 0x6f,
            0x6f, 0x6f, 0x6f, 0x6f, 0x6f, 0x6f, 0x2e, 0x2e,
        ];
        for src in [
            std::str::from_utf8(bytes).unwrap().to_string(),
            "\"q|1\": |\n  body\n".to_string(),
        ] {
            let node =
                parse(&src, YamlSchema::Core).unwrap_or_else(|e| panic!("{src:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = parse(&once, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(
                once,
                crate::serializer::to_yaml(&again),
                "drift for {src:?}: {once:?}"
            );
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-9ee754bf, 83 bytes): a long plain scalar
    /// (>80) whose value contains a run of 2+ spaces. Width-folding broke beside
    /// the run, leaving trailing spaces that re-parse to a different space count,
    /// so the fold was not idempotent (`…999  y|` -> `…999 \n y|` -> `…999\n y|`).
    /// Such values are now emitted unwrapped, preserving the exact spacing. A
    /// long value with only single spaces still wraps and stays stable.
    #[test]
    fn long_plain_scalar_multispace_roundtrips() {
        let crash =
            "j99999999999999999999999999999999999999999999999999999999999999999999999999999  y|\n";
        for src in [crash.to_string(), format!("{}  b\n", "a".repeat(90))] {
            let node =
                parse(&src, YamlSchema::Core).unwrap_or_else(|e| panic!("{src:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = parse(&once, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(
                once,
                crate::serializer::to_yaml(&again),
                "wrap drift for {src:?}: {once:?}"
            );
            // The double-space value must survive verbatim (not collapsed by a
            // fold). Both inputs are root-level plain scalars.
            fn scalar_value(n: &CustomNode) -> String {
                match n {
                    CustomNode::Scalar { value, .. } => value.as_ref().to_string(),
                    _ => String::new(),
                }
            }
            let v = scalar_value(&node);
            assert!(
                v.contains("  ") || v.contains('\t'),
                "test premise: value has multi-ws run: {v:?}"
            );
            assert_eq!(v, scalar_value(&again), "value changed across round-trip");
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-bdf3f15f, and its 12-byte distillation
    /// `k: |2\r  x|y\n`): the old `detect_block_header` scanned up from the content
    /// line by the parser's line number, but granit counts only `\n` as a break, so
    /// a source `\r` kept `key: |2` and a `|`-bearing content line on one logical
    /// line; emitting `\n` shifted which line the scan hit, flipping the indicator
    /// `|2` <-> `|` each round. Byte-anchoring to the scalar's source span reads the
    /// single physical line above the content, so content lines and `\r` shifts can
    /// never be mistaken for the header.
    #[test]
    fn block_header_detected_above_content_lines() {
        let crash: &[u8] = &[
            0x26, 0x62, 0x58, 0x2d, 0x0d, 0x57, 0x26, 0x44, 0x44, 0x44, 0x44, 0x21, 0x20, 0x20,
            0x66, 0x44, 0x44, 0x44, 0x44, 0x44, 0x21, 0x26, 0x26, 0x3a, 0x20, 0x7c, 0x32, 0x0a,
            0x20, 0x20, 0x66, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x69, 0x72, 0x73, 0x74, 0x65,
            0x2a, 0x22, 0x61, 0x20, 0x66, 0x21, 0x66, 0x5b, 0x5b, 0x26, 0x26, 0x3a, 0x20, 0x7c,
            0x26, 0x26, 0x3a, 0x20, 0x7c, 0x32, 0x0a, 0x20, 0x20, 0x66, 0x44, 0x44, 0x44, 0x44,
            0x44, 0x21, 0x26, 0x26, 0x3a, 0x20, 0x7c, 0x32, 0x0a, 0x20, 0x20, 0x66, 0x44, 0x44,
            0x44, 0x44, 0x44, 0x32,
        ];
        let inputs: Vec<String> = vec![
            "k: |2\r  x|y\n".into(),
            "&a\rk: |2\n  x|y\n".into(),
            "k: |2\n  data |2\n  more\n".into(),
            std::str::from_utf8(crash).unwrap().to_string(),
        ];
        for src in inputs {
            let node =
                parse(&src, YamlSchema::Core).unwrap_or_else(|e| panic!("{src:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = parse(&once, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(
                once,
                crate::serializer::to_yaml(&again),
                "drift for {src:?}: {once:?}"
            );
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-41acfbbe, 4 bytes ` ...`): a plain
    /// scalar equal to a document indicator. granit reads ` ...` as the string
    /// `"..."`, but emitting it bare as `...` re-parses as a document-end marker
    /// (null), drifting `...` -> `null` every round. Such scalars are now quoted.
    #[test]
    fn document_indicator_scalar_is_quoted() {
        for input in [
            " ...",
            " ---",
            "a: ...\n",
            "a: ---\n",
            "- ...\n",
            // crash-08f05e25: granit folds ` ...\nk` into the plain scalar `... k`,
            // which emitted at column 0 re-reads as a document-end marker plus
            // invalid trailing content. The `<marker> ` prefix must be quoted too.
            " ...\nk",
            " ... k\n",
            "\r\r ...\nk",
        ] {
            let node = parse(input, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = parse(&once, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(
                once,
                crate::serializer::to_yaml(&again),
                "drift for {input:?}: {once:?}"
            );
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-0de6be17): a bare `#` / `# ` comment
    /// with no text. granit surfaces an empty `Event::Comment` on first parse,
    /// the writer emitted it as a stray `# ` line, and the re-parse then dropped
    /// it - so ONCE had a trailing `  # ` that TWICE lost, drifting every round.
    /// A contentless comment is now not recorded, so the AST matches the
    /// re-readable form. Non-empty comments must survive untouched.
    #[test]
    fn empty_comment_is_not_round_tripped_as_drift() {
        let bytes: &[u8] = &[
            0x62, 0x61, 0x73, 0x65, 0x3a, 0x20, 0x26, 0x62, 0x0a, 0x20, 0x68, 0x69, 0x6c, 0x64,
            0x3a, 0x0a, 0x20, 0x20, 0x3c, 0x3c, 0x3a, 0x20, 0x2a, 0x62, 0x0a, 0x3a, 0x20, 0x32,
            0x20, 0x20, 0x23, 0x20, 0x68, 0x6e, 0x6c, 0x69, 0x6e, 0x65, 0x60, 0x65, 0x2a, 0x62,
            0x0a, 0x3a, 0x20, 0x32, 0x20, 0x20, 0x23, 0x20, 0x62, 0x0a, 0x3a, 0x68, 0x61, 0x73,
            0x65, 0x3a, 0x20, 0x26, 0x62, 0x0a, 0x20, 0x68, 0x69, 0x6c, 0x64, 0x3a, 0x0a, 0x20,
            0x20, 0x3c, 0x3c, 0x3a, 0x20, 0x2a, 0x62, 0x0a, 0x3a, 0x20, 0x32, 0x20, 0x20, 0x23,
            0x20, 0x20,
        ];
        let crash = std::str::from_utf8(bytes).unwrap();
        for input in [crash, "a: 1 # \n", "a: 1  # \nb: 2\n", "# \n- x\n"] {
            let node = parse(input, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = crate::serializer::to_yaml(&node);
            let again = parse(&once, YamlSchema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(
                once,
                crate::serializer::to_yaml(&again),
                "drift for {input:?}: {once:?}"
            );
            assert!(
                !once.contains("# \n"),
                "empty comment leaked into output for {input:?}: {once:?}"
            );
        }
        // A non-empty comment still survives parse -> emit -> re-parse.
        let node = parse("k: v # keep\n", YamlSchema::Core).unwrap();
        let once = crate::serializer::to_yaml(&node);
        assert!(
            once.contains("# keep"),
            "non-empty comment dropped: {once:?}"
        );
    }

    #[test]
    fn test_parse_simple_scalar() {
        let result = parse("hello", YamlSchema::Core);
        assert!(result.is_ok());
        if let Ok(CustomNode::Scalar { value, style, .. }) = result {
            assert_eq!(value.as_ref(), "hello");
            assert_eq!(style, ScalarStyle::Plain);
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-3b0a7d1d): the `IndexMap` keys by the
    /// full `CustomNode`, so two scalar keys with the same *value* but different
    /// trailing comment / style stayed distinct and evaded the exact-node
    /// duplicate check — yet `to_yaml` drops key decor and emitted two colliding
    /// `key:` lines that re-parse rejected. Duplicate detection now identifies a
    /// key by its value (what the serializer emits), while still allowing the
    /// repeated `<<` merge key YAML permits.
    #[test]
    fn duplicate_scalar_key_by_value_is_rejected() {
        // same value, different inline comment on the key
        let err = parse("key: &a # tng\nkey: &a # trail\n", YamlSchema::Core)
            .expect_err("comment-differing duplicate keys must be rejected");
        assert!(matches!(err, ParseError::DuplicateKey(_)), "got {err:?}");
        // plain vs quoted are the same YAML key
        let err = parse("key: 1\n\"key\": 2\n", YamlSchema::Core)
            .expect_err("style-differing duplicate keys must be rejected");
        assert!(matches!(err, ParseError::DuplicateKey(_)), "got {err:?}");
        // repeated `<<` merge keys are legal and must NOT be flagged
        assert!(
            parse(
                "a: &x {p: 1}\nb: &y {q: 2}\nc:\n  <<: *x\n  <<: *y\n",
                YamlSchema::Core
            )
            .is_ok(),
            "duplicate merge keys must be allowed"
        );
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
                .and_then(|d| d.leading_comment())
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
