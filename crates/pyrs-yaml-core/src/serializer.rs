use crate::ast::{Chomping, CustomNode, NodeMeta, ScalarStyle, Tag};
use crate::error::{DepthError, SerializeError};
use crate::parser::yaml::schema::core_type_is_non_string;
use indexmap::IndexMap;

/// 块标量头部的两项指示符，作为一组传递，避免在 `write_scalar` 及其下游
/// 逐层加参数。
#[derive(Debug, Clone, Copy)]
struct BlockScalarHeader<'a> {
    /// Chomping 指示符（`|` / `|-` / `|+`）。
    chomping: &'a Chomping,
    /// 显式缩进指示器（`|2` 中的 `2`）；`None` 表示重新探测。
    indent: Option<u8>,
    /// Inline (`# text`) tail comment for the block header line. granit only
    /// reads a block-scalar header comment on the header line itself — a
    /// comment written on the line *after* `|` is absorbed as block content
    /// (libFuzzer `yaml_roundtrip` crash-cfb3fa83), so it rides here and
    /// `write_block_header` places it before the newline.
    comment: Option<&'a str>,
}

/// Serialization options
#[derive(Debug, Clone, PartialEq)]
pub struct SerializeOptions {
    pub indent_size: usize,
    pub explicit_start: bool,
    pub explicit_end: bool,
    pub sort_keys: bool,
    pub max_depth: usize,
    pub width: usize,
    pub indent_mapping: usize,
    pub indent_sequence: usize,
    pub indent_offset: usize,
}

impl Default for SerializeOptions {
    fn default() -> Self {
        Self {
            indent_size: 2,
            explicit_start: false,
            explicit_end: false,
            sort_keys: false,
            max_depth: 1000,
            width: 80,
            indent_mapping: 2,
            indent_sequence: 2,
            indent_offset: 0,
        }
    }
}

/// Serialize a CustomNode AST back to YAML string
///
/// ```
/// use pyrs_yaml_core::ast::CustomNode;
/// use pyrs_yaml_core::serializer::to_yaml;
/// use indexmap::IndexMap;
/// let mut pairs = IndexMap::new();
/// pairs.insert(CustomNode::plain_scalar("a"), CustomNode::plain_scalar("1"));
/// let node = CustomNode::Mapping {
///     pairs,
///     flow_style: false,
///     meta: Default::default(),
/// };
/// let output = to_yaml(&node);
/// assert_eq!(output, "a: 1\n");
/// ```
pub fn to_yaml(node: &CustomNode) -> String {
    to_yaml_with_options(node, &SerializeOptions::default()).expect("serialization failed")
}

/// Serialize with custom options
///
/// ```
/// use pyrs_yaml_core::ast::CustomNode;
/// use pyrs_yaml_core::serializer::{to_yaml_with_options, SerializeOptions};
/// use indexmap::IndexMap;
/// let mut pairs = IndexMap::new();
/// pairs.insert(CustomNode::plain_scalar("a"), CustomNode::plain_scalar("1"));
/// let node = CustomNode::Mapping {
///     pairs,
///     flow_style: false,
///     meta: Default::default(),
/// };
/// let opts = SerializeOptions {
///     indent_size: 4,
///     explicit_start: true,
///     explicit_end: false,
///     sort_keys: false,
///     ..Default::default()
/// };
/// let output = to_yaml_with_options(&node, &opts).unwrap();
/// assert!(output.starts_with("---\n"));
/// ```
pub fn to_yaml_with_options(
    node: &CustomNode,
    options: &SerializeOptions,
) -> Result<String, SerializeError> {
    let mut serializer = Serializer::new(options);
    if options.explicit_start {
        serializer.output.push_str("---\n");
    }
    serializer.serialize_node_internal(
        node,
        options.indent_offset,
        options.indent_offset,
        false,
        0,
    )?;
    if options.explicit_end {
        serializer.output.push_str("...\n");
    }
    Ok(serializer.output)
}

struct Serializer {
    output: String,
    indent_size: usize,
    sort_keys: bool,
    /// Memoized indent strings by width (avoids `repeat()` on every call)
    indent_cache: Vec<String>,
    /// Indent step per block-mapping nesting level
    indent_mapping: usize,
    /// Indent step per block-sequence nesting level
    indent_sequence: usize,
    /// Maximum recursion depth before serialization fails
    max_depth: usize,
    /// Line width for wrapping (0 = no wrapping)
    width: usize,
}

/// Chomping actually written for a block scalar. A Clip-chomped value whose
/// content carries trailing blank lines cannot round-trip as Clip: granit's
/// Clip read strips every trailing line that ends with a newline, so the only
/// header form that re-reads to the same value at any position is Keep
/// (`+`) — libFuzzer `yaml_roundtrip` crash-c18cb1fd. The promotion is a
/// pure emit-side normalization: the AST keeps its parsed `Clip`.
fn effective_chomping(value: &str, chomping: &Chomping) -> Chomping {
    if matches!(chomping, Chomping::Clip) && value.ends_with("\n\n") {
        Chomping::Keep
    } else {
        *chomping
    }
}

/// Whether a node can be serialized inline on the same line as a mapping
/// key's colon (compact sequence item form): scalars, nulls, aliases, and
/// flow-style containers all end with a newline of their own.
fn inlineable_value(v: &CustomNode) -> bool {
    matches!(
        v,
        CustomNode::Scalar { .. }
            | CustomNode::Null { .. }
            | CustomNode::Alias { .. }
            | CustomNode::Mapping {
                flow_style: true,
                ..
            }
            | CustomNode::Sequence {
                flow_style: true,
                ..
            }
    )
}

/// Shared shape parameters for the mapping/sequence container skeleton
/// consumed by [`Serializer::write_container_node`].
#[derive(Clone, Copy)]
struct ContainerSkeleton<'a> {
    empty: bool,
    meta: &'a NodeMeta,
    flow_style: bool,
    indent_width: usize,
    in_value_context: bool,
    open: char,
    close: char,
}

/// Whether a block mapping can be emitted in the compact `- key: value`
/// sequence-item form: no metadata, non-empty, all keys simple scalars and
/// all values inlineable. Mirrors the guard used by `write_sequence_item`;
/// shared so the splice layer and the serializer can never drift.
pub fn is_compact_item(node: &CustomNode) -> bool {
    matches!(
        node,
        CustomNode::Mapping {
            pairs,
            meta: NodeMeta {
                comment: None,
                anchor: None,
                tag: None,
                ..
            },
            flow_style: false,
            ..
        } if !pairs.is_empty()
            && pairs.iter().all(|(k, v)| {
                !matches!(k, CustomNode::Mapping { .. } | CustomNode::Sequence { .. })
                    // A standalone note cannot share the `- key:` line: it
                    // would emit `# …` mid-line and strand the value.
                    && v.leading_comment().is_none()
                    && inlineable_value(v)
            })
    )
}

/// Serialize a single block-mapping pair via [`Serializer::write_mapping_pair`],
/// producing exactly the bytes a splice regeneration splices in.
pub fn pair_to_string(
    key: &CustomNode,
    value: &CustomNode,
    indent_width: usize,
    depth: usize,
) -> Result<String, SerializeError> {
    let mut s = Serializer::new(&SerializeOptions::default());
    s.write_mapping_pair(key, value, indent_width, depth)?;
    Ok(s.output)
}

/// Serialize a single block-sequence item via [`Serializer::write_sequence_item`].
pub fn item_to_string(
    item: &CustomNode,
    indent_width: usize,
    depth: usize,
) -> Result<String, SerializeError> {
    let mut s = Serializer::new(&SerializeOptions::default());
    s.write_sequence_item(item, indent_width, depth)?;
    Ok(s.output)
}

impl Serializer {
    fn new(options: &SerializeOptions) -> Self {
        let mut cache = Vec::with_capacity(128);
        cache.push(String::new()); // width 0 = empty
        Self {
            output: String::new(),
            indent_size: options.indent_size,
            sort_keys: options.sort_keys,
            indent_cache: cache,
            indent_mapping: options.indent_mapping,
            indent_sequence: options.indent_sequence,
            max_depth: options.max_depth,
            width: options.width,
        }
    }

    /// Ensure indent_cache has an entry for the given width, then write it to output.
    /// Does not return a reference to avoid overlapping borrows.
    fn write_indent(&mut self, width: usize) {
        if self.indent_cache.len() <= width {
            self.indent_cache.resize(width + 1, String::new());
        }
        if self.indent_cache[width].is_empty() {
            self.indent_cache[width] = " ".repeat(width);
        }
        self.output.push_str(&self.indent_cache[width]);
    }

    /// 写入锚点（`&name`）和标签（`!!type`）前缀。
    fn write_anchor_tag(&mut self, anchor: &Option<String>, tag: &Option<Tag>) {
        if anchor.is_none() && tag.is_none() {
            return;
        }
        if let Some(anchor_name) = anchor {
            // Emit the bare `&name ` token. Under the granit-aligned scanner
            // (`scan_anchor_name` = maximal `is_anchor_char` run) any name the
            // AST can hold re-scans to itself after the trailing space, so no
            // quoting is needed — the old quoted-emit branch was compensating
            // for the scanner's invented value-indicator-colon stripping.
            self.output.push('&');
            self.output.push_str(anchor_name);
            self.output.push(' ');
        }
        if let Some(t) = tag {
            self.output.push_str(&t.to_string());
            self.output.push(' ');
        }
    }

    /// An empty container: a `Mapping`/`Sequence` with no entries/items,
    /// regardless of flow style (a block-style empty collection is serialized
    /// by its flow token `{}`/`[]`). Such a node cannot host a standalone
    /// comment on its own line in value position — YAML would interpret the
    /// bare `{}`/`[]` as a node needing a key or item — so standalone comments
    /// on empty containers are demoted to inline.
    fn is_empty_container(node: &CustomNode) -> bool {
        match node {
            CustomNode::Mapping { pairs, .. } => pairs.is_empty(),
            CustomNode::Sequence { items, .. } => items.is_empty(),
            _ => false,
        }
    }

    /// Emit a comment inline after an empty container token (`  # text`).
    ///
    /// Standalone notes on an empty container demote to inline (a bare
    /// `{}` / `[]` on its own line is invalid YAML), so the writer
    /// merges the container's `leading_comment` / `comment` slots here.
    /// PR #117 keeps the semantics identical across both conventions:
    /// `leading_comment()` normalises so a TOML-origin node's leading
    /// note lands on the same line as its `{}`.
    fn output_empty_node_comment(&mut self, meta: &NodeMeta) {
        if let Some(c) = meta.standalone_slot().or_else(|| meta.inline_slot()) {
            self.output.push_str("  # ");
            self.output.push_str(&c.text);
        }
    }

    /// 核心递归序列化方法，处理所有节点类型的缩进和格式化。
    /// `block_base` is the indentation column governing a block scalar's
    /// parent line: the `|`/`>` body must sit deeper than that line, so it is
    /// the *pair/item* indent — not the scalar's own write_indent — whenever a
    /// header shares a line with its parent (`key: |`, `- |`, compact items).
    fn serialize_node_internal(
        &mut self,
        node: &CustomNode,
        indent_width: usize,
        block_base: usize,
        in_value_context: bool,
        depth: usize,
    ) -> Result<(), SerializeError> {
        if depth >= self.max_depth {
            return Err(SerializeError::MaxDepthExceeded(DepthError(self.max_depth)));
        }

        // Handle standalone comments first (but not on empty containers,
        // where a bare `{}`/`[]` on its own line would be invalid YAML —
        // those comments are demoted to inline by the writer below).
        // PR #117: `leading_comment()` normalises across the new
        // `decor.leading_comment` slot (used by the native TOML and
        // JSON engines) and the older `comment(standalone = true)`
        // slot the YAML receiver still writes today, so a document
        // converted from TOML / JSON keeps its leading notes when
        // re-serialised as YAML.
        if let Some(comment) = node.leading_comment()
            && !Self::is_empty_container(node)
        {
            self.write_indent(indent_width);
            self.output.push_str("# ");
            self.output.push_str(&comment.text);
            self.output.push('\n');
        }

        match node {
            CustomNode::Scalar {
                value,
                style,
                meta,
                chomping,
                block_indent,
                ..
            } => self.write_scalar_node(
                value,
                style,
                meta,
                BlockScalarHeader {
                    chomping,
                    indent: *block_indent,
                    comment: meta
                        .comment
                        .as_ref()
                        .filter(|c| !c.standalone)
                        .map(|c| &c.text[..]),
                },
                indent_width,
                block_base,
            )?,
            CustomNode::Mapping {
                pairs,
                meta,
                flow_style,
                ..
            } => self.write_mapping_node(
                pairs,
                meta,
                *flow_style,
                indent_width,
                depth,
                in_value_context,
            )?,
            CustomNode::Sequence {
                items,
                meta,
                flow_style,
                ..
            } => self.write_sequence_node(
                items,
                meta,
                *flow_style,
                indent_width,
                depth,
                in_value_context,
            )?,
            CustomNode::Null { meta, .. } => self.write_null_node(meta, indent_width)?,
            CustomNode::Alias { name } => self.write_alias_node(name, indent_width)?,
        }
        Ok(())
    }

    /// Serialize a scalar node (plain / quoted / block) with its anchor, tag and
    /// trailing comment. Extracted from `serialize_node_internal`.
    fn write_scalar_node(
        &mut self,
        value: &str,
        style: &ScalarStyle,
        meta: &NodeMeta,
        block: BlockScalarHeader<'_>,
        indent_width: usize,
        block_base: usize,
    ) -> Result<(), SerializeError> {
        self.write_indent(indent_width);
        if meta.anchor.is_some() || meta.tag.is_some() {
            self.write_anchor_tag(&meta.anchor, &meta.tag);
        }
        self.write_scalar(value, style, block, self.width, block_base);
        if let Some(c) = &meta.comment
            && !c.standalone
            && !matches!(style, ScalarStyle::Literal | ScalarStyle::Folded)
        {
            self.output.push_str("  # ");
            self.output.push_str(&c.text);
        }
        self.output.push('\n');
        Ok(())
    }

    /// Serialize a mapping node in either flow (`{ ... }`) or block style,
    /// including anchor/tag and trailing comment. The container skeleton is
    /// shared with sequences by [`write_container_node`]; only the element
    /// renderer, pair separator, and block iteration differ.
    fn write_mapping_node(
        &mut self,
        pairs: &IndexMap<CustomNode, CustomNode>,
        meta: &NodeMeta,
        flow_style: bool,
        indent_width: usize,
        depth: usize,
        in_value_context: bool,
    ) -> Result<(), SerializeError> {
        let sk = ContainerSkeleton {
            empty: pairs.is_empty(),
            meta,
            flow_style,
            indent_width,
            in_value_context,
            open: '{',
            close: '}',
        };
        self.write_container_node(
            &sk,
            |s| {
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        s.output.push_str(", ");
                    }
                    s.write_scalar_for_key(key, true);
                    s.output.push_str(": ");
                    s.serialize_flow_value(value, depth + 1)?;
                }
                Ok(())
            },
            |s| {
                let mut pairs_vec: Vec<(&CustomNode, &CustomNode)> = pairs.iter().collect();
                pairs_vec.sort_by(|a, b| {
                    let ka = match a.0 {
                        CustomNode::Scalar { value, .. } => value.as_ref(),
                        _ => "",
                    };
                    let kb = match b.0 {
                        CustomNode::Scalar { value, .. } => value.as_ref(),
                        _ => "",
                    };
                    ka.cmp(kb)
                });
                for (key, value) in pairs_vec.iter().copied() {
                    s.write_mapping_pair(key, value, indent_width, depth)?;
                }
                Ok(())
            },
            |s| {
                for (key, value) in pairs.iter() {
                    s.write_mapping_pair(key, value, indent_width, depth)?;
                }
                Ok(())
            },
        )
    }

    /// Serialize a sequence node in either flow (`[ ... ]`) or block style,
    /// including anchor/tag and trailing comment. See [`write_mapping_node`]
    /// for the shared skeleton.
    fn write_sequence_node(
        &mut self,
        items: &[CustomNode],
        meta: &NodeMeta,
        flow_style: bool,
        indent_width: usize,
        depth: usize,
        in_value_context: bool,
    ) -> Result<(), SerializeError> {
        let sk = ContainerSkeleton {
            empty: items.is_empty(),
            meta,
            flow_style,
            indent_width,
            in_value_context,
            open: '[',
            close: ']',
        };
        self.write_container_node(
            &sk,
            |s| {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        s.output.push_str(", ");
                    }
                    s.serialize_flow_value(item, depth + 1)?;
                }
                Ok(())
            },
            // Sequences never sort their items: the sorted branch is the
            // ordinary block pass (mirrors the pre-refactor behavior).
            move |s: &mut Self| {
                for item in items.iter() {
                    s.write_sequence_item(item, indent_width, depth)?;
                }
                Ok(())
            },
            |s| {
                for item in items.iter() {
                    s.write_sequence_item(item, indent_width, depth)?;
                }
                Ok(())
            },
        )
    }

    /// Shared mapping/sequence skeleton: flow form (bracketed, comma-separated
    /// inline elements), the block preamble (anchor/tag line outside a value
    /// context), the empty-container fallback with its node comment, and the
    /// trailing inline comment after block elements.
    fn write_container_node(
        &mut self,
        sk: &ContainerSkeleton<'_>,
        flow: impl FnOnce(&mut Self) -> Result<(), SerializeError>,
        block_sorted: impl FnOnce(&mut Self) -> Result<(), SerializeError>,
        block: impl FnOnce(&mut Self) -> Result<(), SerializeError>,
    ) -> Result<(), SerializeError> {
        let ContainerSkeleton {
            empty,
            meta,
            flow_style,
            indent_width,
            in_value_context,
            open,
            close,
        } = *sk;
        if flow_style {
            // When a flow container starts its own line (a block-context
            // value under a `key:` whose standalone comment forced the newline
            // branch), it still needs the line indent; the inline callers
            // pass indent_width = 0, so this is a no-op for `key: [..]`.
            self.write_indent(indent_width);
            if meta.anchor.is_some() || meta.tag.is_some() {
                self.write_anchor_tag(&meta.anchor, &meta.tag);
            }
            self.output.push(open);
            if !empty {
                flow(self)?;
            }
            self.output.push(close);
            if empty {
                self.output_empty_node_comment(meta);
            } else if let Some(c) = meta.inline_slot() {
                self.output.push_str("  # ");
                self.output.push_str(&c.text);
            }
            self.output.push('\n');
        } else {
            if empty {
                // Empty block containers spell as `{}`/`[]`; keep the
                // anchor/tag header on that same line. A standalone tag
                // line followed by the braces one indent-shallow (under
                // an explicit-key `?`) is ambiguous and cannot re-parse.
                // An empty container has no block form, so a mapping/sequence
                // *value* reaches here inline (`key: &a {}`) — emit its own
                // anchor/tag even in value context, or it would vanish.
                self.write_indent(indent_width);
                if meta.anchor.is_some() || meta.tag.is_some() {
                    self.write_anchor_tag(&meta.anchor, &meta.tag);
                }
                self.output.push(open);
                self.output.push(close);
                self.output_empty_node_comment(meta);
                self.output.push('\n');
                return Ok(());
            }

            if !in_value_context && (meta.anchor.is_some() || meta.tag.is_some()) {
                self.write_indent(indent_width);
                self.write_anchor_tag(&meta.anchor, &meta.tag);
                self.output.push('\n');
            }

            if self.sort_keys {
                block_sorted(self)?
            } else {
                block(self)?
            }

            if let Some(c) = &meta.comment
                && !c.standalone
            {
                self.write_indent(indent_width);
                self.output.push_str("# ");
                self.output.push_str(&c.text);
                self.output.push('\n');
            }
        }
        Ok(())
    }

    /// Serialize a null node with its anchor, tag and trailing comment.
    /// Extracted from `serialize_node_internal`.
    fn write_null_node(
        &mut self,
        meta: &NodeMeta,
        indent_width: usize,
    ) -> Result<(), SerializeError> {
        self.write_indent(indent_width);
        if meta.anchor.is_some() || meta.tag.is_some() {
            self.write_anchor_tag(&meta.anchor, &meta.tag);
        }
        self.output.push_str("null");
        if let Some(c) = &meta.comment
            && !c.standalone
        {
            self.output.push_str("  # ");
            self.output.push_str(&c.text);
        }
        self.output.push('\n');
        Ok(())
    }

    /// Serialize an alias node (`*name`). Extracted from `serialize_node_internal`.
    fn write_alias_node(&mut self, name: &str, indent_width: usize) -> Result<(), SerializeError> {
        self.write_indent(indent_width);
        self.output.push('*');
        self.output.push_str(name);
        self.output.push('\n');
        Ok(())
    }

    /// Write a scalar value directly to output based on style and chomping.
    /// `remaining` is the remaining width on the current line (0 = don't wrap).
    /// `block_base` is the parent-line indent governing a block scalar's body.
    fn write_scalar(
        &mut self,
        value: &str,
        style: &ScalarStyle,
        block: BlockScalarHeader<'_>,
        remaining: usize,
        block_base: usize,
    ) {
        match style {
            ScalarStyle::Plain => self.write_plain_scalar(value, remaining),
            // Single-quoted scalars cannot represent control characters, Unicode
            // noncharacters, or (losslessly) newlines — a raw `\0` inside `'…'`
            // is not re-parseable. When a value carries such a char (e.g. a
            // JSON5 single-quoted string holding a control char, whose
            // SingleQuoted style the parser preserves for fidelity), downgrade
            // to double-quoted, which can escape them. Found by the dialect
            // no-panic / re-parse fuzz: from_json5 emitted `'\0'` that
            // re-parse rejected.
            ScalarStyle::SingleQuoted
                if value.contains('\n')
                    || value.contains('\u{feff}')
                    || value.chars().any(char::is_control)
                    || value.chars().any(is_yaml_noncharacter) =>
            {
                self.write_double_quoted_scalar(value)
            }
            ScalarStyle::SingleQuoted => self.write_single_quoted_scalar(value),
            ScalarStyle::DoubleQuoted => self.write_double_quoted_scalar(value),
            ScalarStyle::Literal => self.write_literal_scalar(value, block, block_base),
            ScalarStyle::Folded => self.write_folded_scalar(value, block, block_base),
        }
    }

    /// Write one `key: value` pair of a block mapping, including indentation,
    /// the `?` marker for complex keys, and the value emission (block value on
    /// a new line, or inline).
    pub(crate) fn write_mapping_pair(
        &mut self,
        key: &CustomNode,
        value: &CustomNode,
        indent_width: usize,
        depth: usize,
    ) -> Result<(), SerializeError> {
        // Check if key is a complex key (mapping or sequence)
        let is_complex_key = matches!(
            key,
            CustomNode::Mapping { .. } | CustomNode::Sequence { .. }
        );

        if is_complex_key {
            // Complex key: use ? indicator. A standalone note on the key
            // must precede the `?` marker — serializing the key body
            // mid-line would emit the comment *after* `? ` (spelling
            // `? # a` + an orphaned key node that cannot re-parse). Emit
            // the note ourselves, then strip it from the cloned body.
            let stripped;
            let key = if let Some(comment) = key.leading_comment() {
                self.write_indent(indent_width);
                self.output.push_str("# ");
                self.output.push_str(&comment.text);
                self.output.push('\n');
                stripped = strip_leading_comment(key);
                &stripped
            } else {
                key
            };
            self.write_indent(indent_width);
            self.output.push('?');
            self.output.push('\n');
            // The key body lives on its own lines one step deeper than the
            // `?` marker. Starting it mid-line (`? A: …`) pinned the node's
            // first column to the `? ` width while every following line used
            // indent_width, splitting multi-pair / nested keys across two
            // incompatible indents that could not re-parse.
            let body_indent = indent_width + self.indent_mapping;
            self.serialize_node_internal(key, body_indent, body_indent, false, depth + 1)?;
        } else {
            // Simple key
            // Handle standalone comments before the key
            if let Some(comment) = key.leading_comment() {
                self.write_indent(indent_width);
                self.output.push_str("# ");
                self.output.push_str(&comment.text);
                self.output.push('\n');
            }
            self.write_indent(indent_width);
            self.write_scalar_for_key(key, false);
        }

        if is_complex_key {
            // The key body always ended its line (`serialize_node_internal`
            // terminates with `\n`), so the value marker needs its own line
            // aligned with the `?` indicator — pushing `:` bare would land it
            // at column 0 and close any enclosing collection.
            self.write_indent(indent_width);
        }
        self.output.push(':');

        // Check if value needs to be on next line
        if (matches!(
            value,
            CustomNode::Mapping {
                flow_style: false,
                ..
            } | CustomNode::Sequence {
                flow_style: false,
                ..
            }
        ) && !Self::is_empty_container(value))
            || is_complex_key
            || (value.leading_comment().is_some() && !Self::is_empty_container(value))
        {
            // Write a block container's anchor/tag after the colon: only
            // block-style Mapping / Sequence suppress their own header in
            // value context (`write_container_node`'s non-flow branch), so
            // only those need the parent to pre-emit it. A flow container
            // (`!tag [..]`) always writes its own header on its line even in
            // value context — pre-emitting for it duplicated the tag
            // (`a: !a` … `!a [0]`) and the text could not re-parse. Scalar /
            // Null nodes likewise still write their own anchor+tag on the
            // node line — pre-emitting for those duplicated the header
            // (`A: !a` … `!a null`) and a standalone comment landed
            // between the two halves. An empty block container reaches the
            // next-line branch only under a complex key, and now writes its
            // own anchor/tag in `write_container_node`, so pre-emitting would
            // duplicate it (`: !a` … `!a {}`).
            if matches!(
                value,
                CustomNode::Mapping {
                    flow_style: false,
                    ..
                } | CustomNode::Sequence {
                    flow_style: false,
                    ..
                }
            ) && !Self::is_empty_container(value)
            {
                if let Some(anchor_name) = value.anchor() {
                    self.output.push_str(" &");
                    self.output.push_str(anchor_name);
                }
                if let Some(t) = value.tag() {
                    self.output.push(' ');
                    self.output.push_str(&t.to_string());
                }
            }
            self.output.push('\n');
            let child_indent = indent_width + self.indent_mapping;
            self.serialize_node_internal(value, child_indent, child_indent, true, depth + 1)?;
        } else {
            self.output.push(' ');
            // Inline value: the header shares the `key:` line, so a block
            // body must be measured from the pair indent, not from zero.
            self.serialize_node_internal(value, 0, indent_width, true, depth + 1)?;
        }

        Ok(())
    }

    /// Write one `- ` item of a block sequence, keeping the compact inline
    /// form for plain mappings on the dash line.
    pub(crate) fn write_sequence_item(
        &mut self,
        item: &CustomNode,
        indent_width: usize,
        depth: usize,
    ) -> Result<(), SerializeError> {
        self.write_indent(indent_width);
        self.output.push_str("- ");

        if is_compact_item(item) {
            // Compact form: `- key: value` with subsequent keys
            // indented to align under the first key. Only when the
            // mapping carries no metadata and every key/value can
            // share the dash line.
            let CustomNode::Mapping { pairs, .. } = item else {
                return Err(SerializeError::Internal("is_compact_item on non-mapping"));
            };
            for (pi, (key, value)) in pairs.iter().enumerate() {
                if pi > 0 {
                    self.write_indent(indent_width + self.indent_sequence);
                }
                self.write_scalar_for_key(key, false);
                self.output.push(':');
                self.output.push(' ');
                // Compact key column is dash indent + `- ` (== indent_sequence
                // for the default 2-step); a block body hangs off that line.
                let key_base = indent_width + self.indent_sequence;
                self.serialize_node_internal(value, 0, key_base, true, depth + 1)?;
            }
        } else if matches!(
            item,
            CustomNode::Mapping {
                flow_style: false,
                ..
            } | CustomNode::Sequence {
                flow_style: false,
                ..
            }
        ) || item.leading_comment().is_some()
        {
            self.output.push('\n');
            let child_indent = indent_width + self.indent_sequence;
            self.serialize_node_internal(item, child_indent, child_indent, false, depth + 1)?;
        } else {
            // For simple items (including flow-style containers),
            // they go on the same line as the dash. Don't pass
            // indent_width to avoid extra indentation; a block body still
            // hangs off the dash line, so the base is the dash indent.
            self.serialize_node_internal(item, 0, indent_width, false, depth + 1)?;
        }

        Ok(())
    }

    /// Write a scalar formatted as a mapping key.
    fn write_scalar_for_key(&mut self, node: &CustomNode, flow: bool) {
        match node {
            CustomNode::Scalar {
                value,
                style: ScalarStyle::Plain,
                ..
            } => {
                if flow && flow_plain_unsafe(value) {
                    // `,`/`[`,`]`,`{`,`}` end a plain token inside a flow
                    // collection; quoting is the only lossless escape.
                    self.write_double_quoted_scalar(value);
                } else if is_short_alphanumeric(value) {
                    self.output.push_str(value);
                } else {
                    self.write_plain_scalar(value, 0);
                }
            }
            CustomNode::Scalar {
                value,
                style,
                chomping,
                block_indent,
                ..
            } => {
                // A key line cannot host a block scalar header either: the
                // body would collide with the `:` and the mapping structure.
                // Quote it, same normalization as flow values.
                let style = match style {
                    ScalarStyle::Literal | ScalarStyle::Folded => &ScalarStyle::DoubleQuoted,
                    other => other,
                };
                // `style` is no longer a block style, so the indentation
                // indicator never reaches the writer here.
                self.write_scalar(
                    value,
                    style,
                    BlockScalarHeader {
                        chomping,
                        indent: *block_indent,
                        comment: None,
                    },
                    0,
                    0,
                )
            }
            _ => {
                self.output.push_str("null");
            }
        }
    }

    /// Write a plain scalar, quoting if necessary.
    /// `remaining` is the remaining width on the current line (0 = don't wrap).
    fn write_plain_scalar(&mut self, value: &str, remaining: usize) {
        write_plain_scalar(&mut self.output, value, remaining, self.width);
    }

    /// Write a single-quoted scalar (single quotes escaped by doubling).
    fn write_single_quoted_scalar(&mut self, value: &str) {
        write_single_quoted_scalar(&mut self.output, value);
    }

    /// Write a double-quoted scalar with escape sequences.
    fn write_double_quoted_scalar(&mut self, value: &str) {
        write_double_quoted_scalar(&mut self.output, value);
    }

    /// Emit a block scalar header: the `|`/`>` sigil, the chomping indicator
    /// and — when the source pinned one — the explicit indentation indicator.
    /// Order follows the spec (`c-b-block-header`): chomping precedes
    /// indentation, so a stripped, explicitly indented scalar is `|-2`.
    fn write_block_header(
        &mut self,
        sigil: char,
        block: BlockScalarHeader<'_>,
        chomping: &Chomping,
    ) {
        self.output.push(sigil);
        self.output.push_str(match chomping {
            Chomping::Strip => "-",
            Chomping::Clip => "",
            Chomping::Keep => "+",
        });
        if let Some(indent) = block.indent.filter(|n| (1..=9).contains(n)) {
            self.output.push((b'0' + indent) as char);
        }
        if let Some(text) = block.comment {
            // The comment rides the header line: a comment on the line after
            // `|`/`>` would be absorbed into the block content on re-read
            // (libFuzzer `yaml_roundtrip` crash-cfb3fa83).
            self.output.push_str("  # ");
            self.output.push_str(text);
        }
    }

    /// Write a literal block scalar (`|`) with its block header.
    fn write_literal_scalar(
        &mut self,
        value: &str,
        block: BlockScalarHeader<'_>,
        block_base: usize,
    ) {
        let chomping = effective_chomping(value, block.chomping);
        // Mirror the folded writer: when the first content line itself begins
        // with a blank, granit's auto-indent detection would take that deeper
        // column as the block indent and read the shallower following lines as
        // a dedent (ending the block / erroring on re-parse) — libFuzzer
        // `yaml_roundtrip` crash-e432d4b8 (`|1` whose explicit indicator the AST
        // dropped, leaving a value like ` 1|l\n:t\n`). Force the indicator so
        // detection is skipped and the leading blanks stay content. The same
        // resolved indent feeds `write_base_indent` so header and body agree.
        let auto_width = self.block_width(block_base, block.indent);
        let first_text_line = value.lines().find(|l| !l.is_empty()).unwrap_or("");
        let force_indicator = block.indent.is_none() && first_text_line.starts_with([' ', '\t']);
        let indent = if force_indicator {
            Some((auto_width - block_base) as u8)
        } else {
            block.indent
        };
        let header = BlockScalarHeader {
            chomping: &chomping,
            indent,
            comment: block.comment,
        };
        self.write_block_header('|', header, &chomping);
        self.output.push('\n');
        self.write_base_indent(value, block_base, indent);
    }

    fn write_folded_scalar(
        &mut self,
        value: &str,
        block: BlockScalarHeader<'_>,
        block_base: usize,
    ) {
        let chomping = effective_chomping(value, block.chomping);
        // granit's folded read map (measured against its scanner):
        //   - k blank lines before a NORMAL continuation line re-read as k '\n'
        //     (the break before them folds away);
        //   - a MORE-INDENTED continuation line keeps its own leading break, so
        //     k blanks there re-read as k + 1 - one blank less is needed for a
        //     given run;
        //   - k blanks right after the header are k leading newlines.
        // A first content line that itself begins with a blank would also be
        // eaten by granit's auto-indent detection, so force an explicit
        // indentation indicator in that case (with it set, detection is skipped
        // and the leading blanks stay content). Run-consumed segments mean an
        // empty segment can only ever be the leading one.
        let auto_width = self.block_width(block_base, block.indent);
        let first_text_line = value.lines().find(|l| !l.is_empty()).unwrap_or("");
        let force_indicator = block.indent.is_none() && first_text_line.starts_with([' ', '\t']);
        let width = auto_width;
        let header = BlockScalarHeader {
            chomping: &chomping,
            indent: if force_indicator {
                Some((width - block_base) as u8)
            } else {
                block.indent
            },
            comment: block.comment,
        };
        self.write_block_header('>', header, &chomping);
        self.output.push('\n');
        let mut rest = value;
        let mut started = false;
        loop {
            let nl = match rest.find('\n') {
                Some(k) => k,
                None => {
                    if !rest.is_empty() {
                        self.write_indent(width);
                        self.output.push_str(rest);
                    }
                    break;
                }
            };
            let line = &rest[..nl];
            let mut r = 1usize;
            while rest[nl + r..].starts_with('\n') {
                r += 1;
            }
            let after = &rest[nl + r..];
            let this_more_indented = line.starts_with([' ', '\t']);
            if !line.is_empty() {
                self.write_indent(width);
                self.output.push_str(line);
            }
            // granit's fold rule keys on `leading_blank`/`trailing_blank`: a
            // more-indented line (starting with a blank) both keeps its own
            // leading break un-folded AND sets `leading_blank`, so the break on
            // the line AFTER it is likewise kept. The break we are about to
            // emit sits between the line just written (`this_more_indented`, a
            // more-indented line keeps the break after it) and the next content
            // line (`next_more_indented`, whose leading break is kept). A run of
            // r newlines therefore needs no blank padding (just r physical
            // newlines) when either neighbour is more-indented; only a fold
            // between two plain text lines costs the extra break (r + 1).
            // Leading runs have no prior text line and trailing runs defer to
            // `write_scalar_node`.
            let had_text = started || !line.is_empty();
            if !line.is_empty() {
                started = true;
            }
            let next_more_indented = after.starts_with([' ', '\t']);
            let newlines = if after.is_empty() {
                r - 1
            } else if !had_text || this_more_indented || next_more_indented {
                r
            } else {
                r + 1
            };
            for _ in 0..newlines {
                self.output.push('\n');
            }
            if after.is_empty() {
                break;
            }
            rest = after;
        }
    }

    /// Content column of a block body: the explicit indicator wins, else one
    /// indent step below the header line - the same rule `write_base_indent`
    /// uses for literal blocks (parsed block content is stored de-indented,
    /// so sniffing the value for leading spaces would scan the whole body for
    /// nothing). Keep the two writers in lockstep.
    fn block_width(&self, block_base: usize, explicit_indent: Option<u8>) -> usize {
        match explicit_indent.filter(|n| (1..=9).contains(n)) {
            Some(n) => block_base + n as usize,
            None => block_base + self.indent_size,
        }
    }

    /// Write each line of the block scalar content with base indentation appended.
    /// Writes directly to output (no intermediate Vec or join); indentation goes
    /// through the memoized `write_indent` cache instead of a fresh `repeat()`
    /// allocation per block scalar. The body sits one step deeper than the line
    /// carrying the `|`/`>` header, wherever that line lives (top level, a
    /// nested pair, after a dash).
    ///
    /// `explicit_indent` is the source's indentation indicator, which is
    /// relative to the *parent* line, not to `self.indent_size`. It has to win:
    /// the indicator is already on the wire, and if the body did not land at
    /// exactly `block_base + n` the reader would derive a different content
    /// indent and silently change the value. A body whose first line is deeper
    /// than the rest (`|2` over `[1, 2, 3]` / `  `) is the case that needs it —
    /// auto-detection alone would see the deeper first line and misread the
    /// shallower one as a dedent. Values carrying an out-of-range indicator
    /// (only reachable from hand-built ASTs) fall back to `indent_size`, whose
    /// number the header above did not print, so the two stay consistent.
    fn write_base_indent(&mut self, value: &str, block_base: usize, explicit_indent: Option<u8>) {
        let width = match explicit_indent.filter(|n| (1..=9).contains(n)) {
            Some(n) => block_base + n as usize,
            None => block_base + self.indent_size,
        };
        let mut first = true;
        for line in value.lines() {
            if !first {
                self.output.push('\n');
            }
            if !line.is_empty() {
                self.write_indent(width);
                self.output.push_str(line);
            }
            first = false;
        }
    }

    /// 在 flow 上下文中序列化值（不追加换行符）。
    fn serialize_flow_value(
        &mut self,
        node: &CustomNode,
        depth: usize,
    ) -> Result<(), SerializeError> {
        if depth >= self.max_depth {
            return Err(SerializeError::MaxDepthExceeded(DepthError(self.max_depth)));
        }

        match node {
            CustomNode::Scalar {
                value,
                style,
                meta,
                chomping,
                block_indent,
                ..
            } => {
                if meta.anchor.is_some() || meta.tag.is_some() {
                    self.write_anchor_tag(&meta.anchor, &meta.tag);
                }
                // A block scalar (`|` / `>`) cannot live inside a flow
                // collection — its body needs a dedicated, deeper-indented
                // region that flow syntax has no room for. Downgrade to a
                // double-quoted scalar so the emitted text always re-parses
                // (the value survives; only the lost style is normalized,
                // exactly like ruamel/PyYAML do for block scalars in flow).
                let style = match style {
                    ScalarStyle::Literal | ScalarStyle::Folded => &ScalarStyle::DoubleQuoted,
                    // Flow indicators end a plain token mid-scalar, not just
                    // at its start — `needs_double_quoted` only guards the
                    // block-context spelling.
                    ScalarStyle::Plain if flow_plain_unsafe(value) => &ScalarStyle::DoubleQuoted,
                    other => other,
                };
                self.write_scalar(
                    value,
                    style,
                    BlockScalarHeader {
                        chomping,
                        indent: *block_indent,
                        comment: None,
                    },
                    self.width,
                    0,
                );
            }
            CustomNode::Null { meta, .. } => {
                if meta.anchor.is_some() || meta.tag.is_some() {
                    self.write_anchor_tag(&meta.anchor, &meta.tag);
                }
                self.output.push_str("null");
            }
            CustomNode::Mapping { pairs, meta, .. } => {
                if meta.anchor.is_some() || meta.tag.is_some() {
                    self.write_anchor_tag(&meta.anchor, &meta.tag);
                }
                self.output.push('{');
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.write_scalar_for_key(key, true);
                    self.output.push_str(": ");
                    self.serialize_flow_value(value, depth + 1)?;
                }
                self.output.push('}');
            }
            CustomNode::Sequence { items, meta, .. } => {
                if meta.anchor.is_some() || meta.tag.is_some() {
                    self.write_anchor_tag(&meta.anchor, &meta.tag);
                }
                self.output.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.serialize_flow_value(item, depth + 1)?;
                }
                self.output.push(']');
            }
            CustomNode::Alias { name } => {
                self.output.push('*');
                self.output.push_str(name);
            }
        }
        Ok(())
    }
}

// Shared scalar-formatting helpers used by both [`Serializer`] (the AST path)
// and the Python-side `DirectWriter` fast path. Operating on a caller-owned
// `String` keeps them crate-agnostic and stops the two mirror implementations
// from drifting. Output is byte-identical to the previous serializer methods.
//
// `remaining` is the remaining width on the current line (0 disables the
// first-line wrap); `width` is the full wrap width used for continuations.

/// Append `value` to `out` as a single-quoted YAML scalar (quotes doubled).
pub fn write_single_quoted_scalar(out: &mut String, value: &str) {
    out.push('\'');
    for c in value.chars() {
        if c == '\'' {
            out.push_str("''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
}

/// Whether `c` is a Unicode noncharacter (U+FFFE/U+FFFF and the plane-end
/// twins U+1FFFE…U+10FFFF). granit-parser rejects these even inside quoted
/// scalars, so they must always be escaped.
// `is_yaml_noncharacter` now lives in the shared `pyrs-schema` crate (used by
// both the scalar resolvers and the YAML emitter); re-exported here so
// `crate::serializer::is_yaml_noncharacter` callers keep resolving.
pub use pyrs_schema::is_yaml_noncharacter;

/// Append `value` to `out` as a double-quoted YAML scalar, escaping control
/// and special characters.
pub fn write_double_quoted_scalar(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            '\x08' => out.push_str("\\b"),
            '\x0C' => out.push_str("\\f"),
            '\x1B' => out.push_str("\\e"),
            '/' => out.push_str("\\/"),
            c if c.is_control() || c == '\u{feff}' || is_yaml_noncharacter(c) => {
                // U+FEFF (BOM / ZWNBSP) is category Cf, so `is_control` misses
                // it, yet granit rejects a raw BOM inside a document ("a BOM must
                // not appear inside a document"). Escaping it as \uFEFF keeps the
                // value while keeping the byte off the stream. `\u` escapes are 4
                // hex digits (BMP only); chars above U+FFFF use the 8-digit `\U`.
                let u = c as u32;
                if u > 0xFFFF {
                    out.push_str(&format!("\\U{:08x}", u));
                } else {
                    out.push_str(&format!("\\u{:04x}", u));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Whether a plain scalar must be rendered double-quoted because it would be
/// ambiguous or invalid as an unquoted token.
fn needs_double_quoted(value: &str) -> bool {
    // Empty scalars must always be quoted: an empty plain token parses back as
    // null, not as an empty string.
    if value.is_empty() {
        return true;
    }
    // A token with edge whitespace can never survive plain emission: the
    // parser strips it (`" "` reparses as an empty — or in flow, broken —
    // token), so quote it even when the raw text would schema-resolve to a
    // non-string (e.g. `" "` → null). The schema promise is moot once
    // re-parsing cannot recover the token at all.
    if value.starts_with([' ', '\t']) || value.ends_with([' ', '\t']) {
        return true;
    }
    // A plain scalar that resolves to a non-string type (int/float/bool/null)
    // is emitted unquoted: its loaded type under the core schema equals
    // `resolve_core_type(text)`, so plain emission always reproduces that type
    // on re-parse. Quoting a value like `-1` would instead load back as the
    // string "-1", because quoted scalars are never schema-resolved (YAML 1.2).
    if core_type_is_non_string(value) {
        return false;
    }
    // Genuine strings: quote only when raw emission would be ambiguous or
    // invalid YAML (YAML indicator characters at the token start, an embedded
    // colon or hash, or a newline).
    value.contains(':')
        || value.contains('#')
        || value.contains('\n')
        || value.contains('\u{00a0}')
        || value.contains('\u{feff}')
        || value.chars().any(char::is_control)
        || value.chars().any(is_yaml_noncharacter)
        || value.starts_with('-')
        || value.starts_with('{')
        || value.starts_with('}')
        || value.starts_with('[')
        || value.starts_with(']')
        || value.starts_with('*')
        || value.starts_with('&')
        || value.starts_with('!')
        || value.starts_with('?')
        || value.starts_with('%')
        || value.starts_with('@')
        || value.starts_with('`')
        || value.starts_with('\'')
        || value.starts_with('"')
        || value.starts_with('|')
        || value.starts_with('>')
        || value.starts_with(',')
}

/// Clone of a node with its effective leading (standalone) comment removed
/// from *both* storage slots (`decor.leading_comment` and a standalone
/// `comment`), so `serialize_node_internal` will not re-emit a note the
/// caller has already written above a `?` marker.
fn strip_leading_comment(node: &CustomNode) -> CustomNode {
    let mut stripped = node.clone();
    let meta = match &mut stripped {
        CustomNode::Scalar { meta, .. }
        | CustomNode::Mapping { meta, .. }
        | CustomNode::Sequence { meta, .. }
        | CustomNode::Null { meta, .. } => meta,
        CustomNode::Alias { .. } => return stripped,
    };
    if let Some(decor) = &mut meta.decor {
        decor.leading_comment = None;
    }
    if meta.comment.as_ref().is_some_and(|c| c.standalone) {
        meta.comment = None;
    }
    stripped
}

/// Whether a plain scalar would break when emitted inside a flow
/// collection. `needs_double_quoted` already rejects leading indicators and
/// embedded `:`/`#`; the extra flow-context danger is an *embedded* `,` `[`
/// `]` `{` `}`, which terminates the plain token per YAML 12.3.3 and corrupts
/// the surrounding flow structure.
fn flow_plain_unsafe(value: &str) -> bool {
    value.contains(',')
        || value.contains('[')
        || value.contains(']')
        || value.contains('{')
        || value.contains('}')
}

/// Append `value` to `out` as a plain scalar, double-quoting it if required
/// and wrapping to `width` when it overflows `remaining`.
pub fn write_plain_scalar(out: &mut String, value: &str, remaining: usize, width: usize) {
    if value.len() <= 8 && value.bytes().all(|b| b.is_ascii_alphanumeric()) {
        out.push_str(value);
        return;
    }
    if needs_double_quoted(value) {
        if value.contains('\\')
            && !value.contains('\n')
            && !value.contains('\u{feff}')
            && !value.chars().any(char::is_control)
            && !value.chars().any(is_yaml_noncharacter)
        {
            // granit-parser mishandles `\\<escape-letter>` inside double-quoted
            // scalars (e.g. `\\0` collapses to NUL). Single-quoted scalars keep
            // backslashes literal, so prefer them whenever the value contains a
            // backslash (and no newline, which single-quoted cannot represent,
            // and no control/noncharacter, which single-quoted cannot escape).
            write_single_quoted_scalar(out, value);
        } else {
            write_double_quoted_scalar(out, value);
        }
    } else if remaining > 0 && value.len() > remaining {
        let safe_remaining = value.floor_char_boundary(remaining);
        match value[..safe_remaining].rfind(' ') {
            Some(split) => {
                out.push_str(&value[..split]);
                let rest = value[split..].trim();
                if !rest.is_empty() {
                    // Continuation lines must be indented past the block
                    // indentation; aligning to the value's own column is always
                    // safe (and matches the value start column for inline
                    // contexts like `- item` or `key: value`).
                    let line_start = out.rfind('\n').map_or(0, |i| i + 1);
                    let cont_indent = (out.len() - line_start).max(2);
                    out.push('\n');
                    wrap_plain_scalar(out, rest, width, cont_indent);
                }
            }
            None => {
                // No whitespace to fold across: a mid-token line break would
                // re-parse as an inserted space and change the value. Emit the
                // whole value as one (potentially long) lossless line.
                out.push_str(value);
            }
        }
    } else {
        out.push_str(value);
    }
}

/// Wrap `value` to `width` with continuation lines indented `cont_indent`
/// columns. Used by [`write_plain_scalar`] when a value overflows the current
/// line.
pub fn wrap_plain_scalar(out: &mut String, value: &str, width: usize, cont_indent: usize) {
    let wrap_indent_str = " ".repeat(cont_indent);
    let mut remaining_rest = value;
    while !remaining_rest.is_empty() {
        out.push_str(&wrap_indent_str);
        if remaining_rest.len() <= width.saturating_sub(wrap_indent_str.len()) {
            out.push_str(remaining_rest);
            break;
        }
        let avail = width.saturating_sub(wrap_indent_str.len());
        if avail == 0 {
            out.push_str(remaining_rest);
            break;
        }
        let safe_avail = remaining_rest.floor_char_boundary(avail);
        match remaining_rest[..safe_avail].rfind(' ') {
            Some(split) => {
                out.push_str(&remaining_rest[..split]);
                out.push('\n');
                remaining_rest = remaining_rest[split..].trim();
            }
            None => {
                // No whitespace to fold across; another folded line would
                // re-parse as an inserted space and change the value. Emit the
                // remainder as one (potentially long) lossless line.
                out.push_str(remaining_rest);
                break;
            }
        }
    }
}

/// Whether a key or scalar value is a short, purely alphanumeric token that
/// can be emitted without any quoting or wrapping.
pub fn is_short_alphanumeric(value: &str) -> bool {
    value.len() <= 8 && value.bytes().all(|b| b.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Tag;
    use indexmap::IndexMap;
    use std::sync::Arc;

    /// libFuzzer `yaml_roundtrip` find: granit delivers double-quoted scalars
    /// already decoded, and the receiver's own unescape used to run a *second*
    /// decode over them — `a: "\\n"` (two literal chars `\` `n`) collapsed to
    /// LF, and the fuzz repro `!-# \\f"\t0:!` lost a backslash per round-trip.
    #[test]
    fn double_quoted_escapes_decode_exactly_once() {
        let node = crate::parser::parse(r#"a: "\\n""#, pyrs_schema::types::Schema::Core).unwrap();
        let CustomNode::Mapping { pairs, .. } = &node else {
            panic!()
        };
        let value = pairs.values().next().unwrap();
        assert!(
            matches!(value, CustomNode::Scalar { value, .. } if value.as_ref() == r"\n"),
            "double-decode regressed: {value:?}"
        );
        // And the full serialize loop is stable on the original fuzz input.
        let input = "!-# \\f\"\t0:!";
        let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core).unwrap();
        let once = to_yaml(&node);
        let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core).unwrap();
        assert_eq!(once, to_yaml(&again), "first output: {once:?}");
    }

    /// libFuzzer `yaml_roundtrip` (12 bytes `&&&&:<LF>#&&&:&`): the raw anchor
    /// scanner took the `:` (followed by end-of-line, i.e. the value indicator)
    /// as name material and re-harvested phantom anchors from the `#&&&:&`
    /// comment, so the emitted `&&&&: v` re-parsed as anchor `&&&` plus an
    /// indicator — drifting one character per serialize round. Pinned here at
    /// the full parse -> to_yaml -> re-parse -> to_yaml loop the fuzz target
    /// exercises, not just the `extract_anchors` unit.
    #[test]
    fn anchor_value_indicator_roundtrip_is_stable() {
        let input = "&&&&:\n#&&&:&";
        let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core).unwrap();
        let once = to_yaml(&node);
        let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
            .unwrap_or_else(|e| panic!("fmt output failed to re-parse: {e}\n---\n{once}\n---"));
        assert_eq!(
            once,
            to_yaml(&again),
            "serialization not idempotent: {once:?}"
        );
    }

    /// libFuzzer `yaml_roundtrip` (42 bytes `&"X-::::…:<CR>ba`): an anchor name
    /// ending in `:` was emitted as a plain `&name ` token, so the trailing
    /// colon merged with the following space into a value indicator and each
    /// serialize round dropped exactly one colon (drift). The serializer now
    /// wraps unsafe names in quotes; the emitted form must re-parse and stay
    /// byte-identical across rounds.
    #[test]
    fn anchor_name_with_trailing_colon_roundtrips() {
        let input = format!("&\"X-{}{}ba", ":".repeat(35), "\r");
        let node = crate::parser::parse(&input, pyrs_schema::types::Schema::Core).unwrap();
        let once = to_yaml(&node);
        let twice =
            to_yaml(&crate::parser::parse(&once, pyrs_schema::types::Schema::Core).unwrap());
        let thrice =
            to_yaml(&crate::parser::parse(&twice, pyrs_schema::types::Schema::Core).unwrap());
        assert_eq!(
            once, twice,
            "not idempotent (round1->2): {once:?} vs {twice:?}"
        );
        assert_eq!(twice, thrice, "not idempotent (round2->3): {twice:?}");
        // And the anchor name is genuinely preserved, not silently emptied.
        assert!(once.contains("X-"), "anchor lost: {once:?}");
    }

    /// libFuzzer `yaml_roundtrip` (49 bytes, crash-c18cb1fd; the 43-byte
    /// crash-cfb3fa83 rides the same writer rules): a Clip-chomped block whose
    /// value carries a trailing blank line round-trips only with the Keep
    /// indicator — granit's Clip read strips trailing newlines — and a block
    /// scalar's inline comment must ride the header line, never a line of its
    /// own (it would be absorbed as block content). Both closures hold by
    /// construction now; assert the exact crash inputs re-serialize stable.
    #[test]
    fn block_scalar_emission_is_closed_under_reparse() {
        for input in [
            "yaml: |2\n  filineam  first\n  lineaml: |-ml: |2\n  ",
            "base: &b\n  x: 2  f,1\n:&hcild:\n  <<: *b\n  y: |  # inlEEE\n",
            "y: |2\n  a\n  b\n  ",
            "y: |\n  a\n  ",
            "y: |1\n a\n ",
        ] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            let twice = to_yaml(&again);
            assert_eq!(
                once, twice,
                "not idempotent for {input:?}: {once:?} vs {twice:?}"
            );
        }
    }

    /// libFuzzer `yaml_roundtrip` (14 bytes, crash-490c4beb): a folded scalar
    /// whose value carries a run of newlines between text lines. granit's
    /// folded read turns k blank lines into exactly k newlines, but the old
    /// line-splitting writer emitted one blank too few, so every run shrank a
    /// newline per round (4 -> 3 -> 2 -> ...). The fold-aware writer now closes
    /// every run length by construction; pin the crash input plus runs 1..=5.
    #[test]
    fn folded_newline_runs_roundtrip() {
        let input = ">\r,2C\u{f6bf}\r\r\r\r\r,";
        let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core).unwrap();
        let once = to_yaml(&node);
        let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
            .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
        assert_eq!(once, to_yaml(&again), "crash input drift: {once:?}");
        for m in 1..=5usize {
            for value in [
                format!("a{}b\n", "\n".repeat(m)),
                format!("{}b\n", "\n".repeat(m)),
                format!("a{} b\n", "\n".repeat(m)),
                format!("{} b\n", "\n".repeat(m)),
            ] {
                let node = CustomNode::Scalar {
                    value: Arc::from(value.as_str()),
                    style: ScalarStyle::Folded,
                    chomping: Chomping::Clip,
                    block_indent: None,
                    meta: Default::default(),
                };
                let once = to_yaml(&node);
                let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                    .unwrap_or_else(|e| panic!("{value:?}: must re-parse: {e}\n{once:?}"));
                assert_eq!(
                    once,
                    to_yaml(&again),
                    "folded run drifts for {value:?}: {once:?}"
                );
            }
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-b7a2285e): a folded scalar whose value
    /// has a MORE-INDENTED continuation line (leading tab/space) flanked by
    /// plain text lines. granit's fold rule keys on `leading_blank`: a
    /// more-indented line keeps the break *before* it and also suppresses the
    /// fold of the break *after* it, so an r-newline run touching a
    /// more-indented neighbour needs exactly r physical newlines — not r + 1.
    /// The writer previously padded every run to r + 1, so each round-trip
    /// gained a blank line (the ONCE/TWICE drift). Pin the crash input plus
    /// synthesized normal↔more-indented shapes and assert full value fidelity
    /// (re-parse preserves the scalar value), not just byte idempotency.
    #[test]
    fn folded_more_indented_continuation_roundtrip() {
        // crash-b7a2285e exact input.
        let crash = ">\n'\"\"\n\t<\t\t\t(\n(\t\t(\n([";
        for input in [crash, ">\na\n\tb\n", ">\n \tq\n r\n"] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(once, to_yaml(&again), "crash input drift: {once:?}");
            let scalar_value = |n: &CustomNode| match n {
                CustomNode::Scalar { value, .. } => value.as_ref().to_string(),
                other => panic!("expected scalar, got {other:?}"),
            };
            assert_eq!(
                scalar_value(&node),
                scalar_value(&again),
                "value not preserved for {input:?}: {once:?}"
            );
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-e432d4b8): a literal (`|`) block
    /// scalar whose first content line begins with a blank but a later line is
    /// shallower. The AST drops the source's explicit `|1` indicator, so the
    /// de-indented value (e.g. ` 1|l\n:t\n`) re-emitted with auto-detect lets
    /// granit read the deeper first line as the block indent and treat the
    /// shallower line as a dedent — the output no longer re-parses. The literal
    /// writer now forces an indentation indicator (mirroring the folded writer)
    /// so detection is skipped. Pin the crash input plus blank-first-line shapes
    /// with full value fidelity.
    #[test]
    fn literal_leading_blank_line_roundtrip() {
        for input in [
            "yaml: |1\n  1|l\n :t  ts  lins  lines\n\n",
            "k: |1\n   x\n  y\n",
            "k: |1\n  a\n  b\n c\n",
        ] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(
                once,
                to_yaml(&again),
                "literal drift for {input:?}: {once:?}"
            );
        }
    }

    /// libFuzzer `yaml_roundtrip` (crash-d0e84310): a block-style *empty*
    /// mapping/sequence used as a mapping value. An empty collection has no
    /// block form, so emitting `key:` then `  {}` on the next line re-reads as
    /// a FLOW mapping (flow_style flips), and the second round inlined it —
    /// ONCE `key:\n  {}` vs TWICE `key: {}` drift. Empty containers must always
    /// serialize inline. Pin the crash input plus empty-value shapes with tag.
    #[test]
    fn empty_container_value_is_inlined() {
        for input in [
            "base: &b\n  \n: 1xchi\n  \n!: 1xchild:\n  <<: *b\n: y: 2  # inline\n",
            "k:\n  {}\n",
            "k: !a {}\n",
            "? {}\n: !a {}\n",
            "a:\n  b: {}\n  c: []\n",
        ] {
            let node = crate::parser::parse(input, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("{input:?} must parse: {e}"));
            let once = to_yaml(&node);
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            assert_eq!(once, to_yaml(&again), "drift for {input:?}: {once:?}");
        }
    }

    /// A `U+FEFF` (BOM / ZWNBSP) is category Cf, so the double-quoted escaper's
    /// `is_control` guard missed it and emitted it raw — but granit rejects a
    /// raw BOM appearing inside a document ("a BOM must not appear inside a
    /// document"), so any scalar carrying a BOM round-tripped to unparseable
    /// text (the double-quoted-scalar half of libFuzzer `yaml_roundtrip`
    /// crash-f4c74685). The escaper now emits it as `\uFEFF`; re-parse restores
    /// the exact value with no raw BOM byte in the stream.
    #[test]
    fn bom_in_scalar_is_escaped_and_round_trips() {
        for (value, style) in [
            ("a\u{feff}b", ScalarStyle::DoubleQuoted),
            ("\u{feff}lead", ScalarStyle::Plain),
            ("mid\u{feff}", ScalarStyle::SingleQuoted),
        ] {
            let node = CustomNode::Scalar {
                value: Arc::from(value),
                style,
                chomping: Chomping::Clip,
                block_indent: None,
                meta: Default::default(),
            };
            let once = to_yaml(&node);
            assert!(
                !once.contains('\u{feff}'),
                "raw BOM leaked into output: {once:?}"
            );
            let again = crate::parser::parse(&once, pyrs_schema::types::Schema::Core)
                .unwrap_or_else(|e| panic!("output must re-parse: {e}\n---\n{once}\n---"));
            match &again {
                CustomNode::Scalar { value: v, .. } => {
                    assert_eq!(v.as_ref(), value, "value changed: {once:?}")
                }
                _ => panic!("expected scalar, got {again:?}"),
            }
            assert_eq!(once, to_yaml(&again), "not idempotent: {once:?}");
        }
    }

    /// Print a unified diff when two YAML strings differ, then panic.
    macro_rules! assert_yaml_eq {
        ($expected:expr, $actual:expr) => {{
            let expected = $expected;
            let actual = $actual;
            let expected_str: &str = expected.as_ref();
            let actual_str: &str = actual.as_ref();
            if expected_str != actual_str {
                let diff = similar::TextDiff::from_lines(expected_str, actual_str);
                let out = diff
                    .unified_diff()
                    .context_radius(3)
                    .header("expected", "actual")
                    .to_string();
                panic!(
                    "YAML mismatch:\n\
                     expected ({} bytes):\n{expected_str:?}\n\
                     actual   ({} bytes):\n{actual_str:?}\n\n\
                     diff:\n{out}",
                    expected_str.len(),
                    actual_str.len()
                );
            }
        }};
    }

    /// Test fixture: core-schema parse with the standard options.
    fn parse_core(yaml: &str) -> CustomNode {
        crate::parser::parse_with_options(
            yaml,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap()
    }

    #[test]
    fn test_pair_helper_matches_full_serialize() {
        let yaml = "a: 1\nb:\n  c: 2\n";
        let ast = parse_core(yaml);
        let full = crate::serializer::to_yaml(&ast);
        let mut s = Serializer::new(&SerializeOptions::default());
        let CustomNode::Mapping { pairs, .. } = &ast else {
            panic!()
        };
        for (k, v) in pairs.iter() {
            s.write_mapping_pair(k, v, 0, 0).unwrap();
        }
        assert_yaml_eq!(&full, &s.output);
    }

    #[test]
    fn test_item_helper_matches_full_serialize() {
        // mixed items: simple scalar + compact mapping (multi-key alignment) + block container
        let yaml = "- a\n- b: c\n  d: 1\n- - 1\n  - 2\n";
        let ast = parse_core(yaml);
        let full = crate::serializer::to_yaml(&ast);
        let mut s = Serializer::new(&SerializeOptions::default());
        let CustomNode::Sequence { items, .. } = &ast else {
            panic!()
        };
        for item in items.iter() {
            s.write_sequence_item(item, 0, 0).unwrap();
        }
        assert_yaml_eq!(&full, &s.output);
    }

    #[test]
    fn test_item_helper_preserves_compact_dash() {
        let yaml = "- host: a\n";
        let ast = parse_core(yaml);
        let mut s = Serializer::new(&SerializeOptions::default());
        let CustomNode::Sequence { items, .. } = &ast else {
            panic!()
        };
        s.write_sequence_item(&items[0], 0, 0).unwrap();
        assert_yaml_eq!(&s.output, "- host: a\n"); // P2: dash prefix must survive
    }

    #[test]
    fn test_serialize_plain_scalar() {
        let node = CustomNode::plain_scalar("hello");
        assert_yaml_eq!(to_yaml(&node), "hello\n");
    }

    #[test]
    fn test_serialize_scalar_with_comment() {
        let node = CustomNode::Scalar {
            value: Arc::from("value"),
            style: ScalarStyle::Plain,
            meta: NodeMeta {
                comment: Some(crate::ast::Comment {
                    text: Arc::from("a comment"),
                    standalone: false,
                }),
                ..Default::default()
            },
            chomping: Chomping::Clip,
            block_indent: None,
        };
        assert_yaml_eq!(to_yaml(&node), "value  # a comment\n");
    }

    #[test]
    fn test_single_quoted_control_char_downgrades_to_double_quoted() {
        // Regression for the dialect re-parse fuzz (from_json5): a JSON5
        // single-quoted string holding a control char keeps SingleQuoted
        // style for fidelity, but `'\0'` is not re-parseable YAML. The
        // serializer must downgrade such values to double-quoted, which
        // escapes them losslessly.
        for value in ["\0", "a\u{1f}b", "line\nbreak", "\u{fffe}"] {
            let node = CustomNode::Scalar {
                value: Arc::from(value),
                style: ScalarStyle::SingleQuoted,
                meta: NodeMeta::default(),
                chomping: Chomping::Clip,
                block_indent: None,
            };
            let dumped = to_yaml(&node);
            let reparsed = parse_core(&dumped);
            let CustomNode::Scalar { value: got, .. } = reparsed else {
                panic!("re-parse of {dumped:?} did not yield a scalar")
            };
            assert_eq!(&*got, value, "round-trip via {dumped:?}");
        }
    }

    #[test]
    fn test_serialize_scalar_with_tag() {
        let node = CustomNode::Scalar {
            value: Arc::from("42"),
            style: ScalarStyle::Plain,
            meta: NodeMeta {
                tag: Some(Tag::primary("int")),
                ..Default::default()
            },
            chomping: Chomping::Clip,
            block_indent: None,
        };
        assert_yaml_eq!(to_yaml(&node), "!!int 42\n");
    }

    #[test]
    fn test_serialize_mapping() {
        let key = CustomNode::plain_scalar("key");
        let value = CustomNode::plain_scalar("value");

        let mut pairs = IndexMap::new();
        pairs.insert(key, value);

        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };

        assert_yaml_eq!(to_yaml(&node), "key: value\n");
    }

    #[test]
    fn test_serialize_complex_key() {
        let key = CustomNode::Sequence {
            items: vec![
                CustomNode::plain_scalar("key1"),
                CustomNode::plain_scalar("key2"),
            ],
            flow_style: false,
            meta: Default::default(),
        };
        let value = CustomNode::plain_scalar("value");

        let mut pairs = IndexMap::new();
        pairs.insert(key, value);

        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };

        let output = to_yaml(&node);
        assert!(output.contains("?\n"));
        // The explicit-key spelling must re-parse.
        let reparsed = crate::parser::parse_with_options(
            &output,
            true,
            crate::parser::yaml::Schema::Core,
            1000,
            false,
        );
        assert!(reparsed.is_ok(), "{output:?} -> {reparsed:?}");
    }

    /// Serialize `node` and assert the emitted text re-parses; returns the
    /// text so callers can also pin the exact spelling.
    fn assert_reparses(node: &CustomNode) -> String {
        let out = to_yaml(node);
        let reparsed = crate::parser::parse_with_options(
            &out,
            true,
            crate::parser::yaml::Schema::Core,
            1000,
            false,
        );
        assert!(
            reparsed.is_ok(),
            "serialized output failed to re-parse: {out:?} -> {reparsed:?}"
        );
        out
    }

    fn literal_scalar(value: &str) -> CustomNode {
        let mut node = CustomNode::quoted_scalar(value);
        node.set_scalar_style(ScalarStyle::Literal);
        node
    }

    /// A literal scalar pinned to an explicit indentation indicator, as
    /// `|2` in the source would produce.
    fn literal_scalar_with_indent(value: &str, indent: u8) -> CustomNode {
        let mut node = literal_scalar(value);
        if let CustomNode::Scalar { block_indent, .. } = &mut node {
            *block_indent = Some(indent);
        }
        node
    }

    /// The `|2` header has to survive a round trip together with the body
    /// indentation it pins. Without it the reader re-detects the content
    /// indent from the *first* body line, so a body whose first line is
    /// deeper than the rest (`4RWC.yaml`) would come back as a dedent.
    ///
    /// The indicator is relative to the line carrying the header, so a
    /// top-level `yaml: |2` puts the body at column 2 — which then leaves the
    /// second line's own 2 leading spaces as content.
    #[test]
    fn explicit_block_indent_indicator_round_trips() {
        let mut pairs = IndexMap::new();
        pairs.insert(
            CustomNode::plain_scalar("yaml"),
            literal_scalar_with_indent("[1, 2, 3]  \n  \n", 2),
        );
        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&node);
        assert_yaml_eq!("yaml: |2\n  [1, 2, 3]  \n    \n", &out);

        // The indicator is the only thing that makes the shallower second
        // line part of the body rather than a dedent, so the re-parsed value
        // must be byte-identical to the original.
        let reparsed = crate::parser::parse_with_options(
            &out,
            true,
            crate::parser::yaml::Schema::Core,
            1000,
            false,
        )
        .expect("re-parse");
        let CustomNode::Mapping { pairs, .. } = &reparsed else {
            panic!("unexpected shape: {reparsed:?}");
        };
        let Some(CustomNode::Scalar { value, .. }) = pairs.get(&CustomNode::plain_scalar("yaml"))
        else {
            panic!("missing `yaml` key: {out:?}");
        };
        assert_eq!(value.as_ref(), "[1, 2, 3]  \n  \n");
    }

    /// Chomping precedes indentation in a block header (`c-b-block-header`),
    /// so the two indicators have to be emitted in that order.
    #[test]
    fn block_indent_indicator_follows_chomping_indicator() {
        let mut node = literal_scalar_with_indent("x\ny", 2);
        if let CustomNode::Scalar { chomping, .. } = &mut node {
            *chomping = Chomping::Strip;
        }
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar("a"), node);
        let mapping = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&mapping);
        assert_yaml_eq!("a: |-2\n  x\n  y\n", &out);
    }

    /// The pinned number must drive the body indent, not `indent_size`: at
    /// `a:\n  b: |-4` the header sits on the `b:` line (column 2), so the body
    /// lands at column 6. Deriving it from `indent_size` would emit a `|-4`
    /// header over a column-4 body, and the reader would believe the header.
    #[test]
    fn explicit_block_indent_is_relative_to_parent_line() {
        let mut inner = IndexMap::new();
        inner.insert(
            CustomNode::plain_scalar("b"),
            literal_scalar_with_indent("x\ny", 4),
        );
        let mut outer = IndexMap::new();
        outer.insert(
            CustomNode::plain_scalar("a"),
            CustomNode::Mapping {
                pairs: inner,
                flow_style: false,
                meta: Default::default(),
            },
        );
        let node = CustomNode::Mapping {
            pairs: outer,
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&node);
        assert_yaml_eq!("a:\n  b: |4\n      x\n      y\n", &out);
    }

    /// No indicator in, no indicator out: the writer must not invent one
    /// (it would pin a value the source never expressed).
    #[test]
    fn block_scalar_without_indicator_keeps_derived_indent() {
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar("b"), literal_scalar("x\ny"));
        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&node);
        assert_yaml_eq!("b: |\n  x\n  y\n", &out);
    }

    #[test]
    fn nested_literal_block_scalar_body_follows_parent_indent() {
        let mut inner = IndexMap::new();
        inner.insert(CustomNode::plain_scalar("b"), literal_scalar("x\ny"));
        let mut outer = IndexMap::new();
        outer.insert(
            CustomNode::plain_scalar("a"),
            CustomNode::Mapping {
                pairs: inner,
                flow_style: false,
                meta: Default::default(),
            },
        );
        let node = CustomNode::Mapping {
            pairs: outer,
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&node);
        assert_yaml_eq!("a:\n  b: |\n    x\n    y\n", &out);
    }

    #[test]
    fn seq_item_literal_block_scalar_body_hangs_off_dash() {
        let seq = CustomNode::Sequence {
            items: vec![literal_scalar("x\ny")],
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&seq);
        assert_yaml_eq!("- |\n  x\n  y\n", &out);
    }

    #[test]
    fn compact_dash_literal_block_scalar_body_indents_past_key() {
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar("b"), literal_scalar("x\ny"));
        let seq = CustomNode::Sequence {
            items: vec![CustomNode::Mapping {
                pairs,
                flow_style: false,
                meta: Default::default(),
            }],
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&seq);
        assert_yaml_eq!("- b: |\n    x\n    y\n", &out);
    }

    #[test]
    fn literal_block_scalar_in_flow_demotes_to_double_quoted() {
        let seq = CustomNode::Sequence {
            items: vec![literal_scalar("x")],
            flow_style: true,
            meta: Default::default(),
        };
        let out = assert_reparses(&seq);
        assert_eq!(&out, "[\"x\"]\n");
    }

    #[test]
    fn plain_scalar_with_edge_spaces_is_quoted() {
        let mut pairs = IndexMap::new();
        pairs.insert(
            CustomNode::plain_scalar("k"),
            CustomNode::plain_scalar(" %"),
        );
        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
        let out = assert_reparses(&node);
        assert_eq!(&out, "k: \" %\"\n");
    }

    #[test]
    fn plain_scalar_with_comma_in_flow_is_quoted() {
        let seq = CustomNode::Sequence {
            items: vec![CustomNode::plain_scalar("a,b")],
            flow_style: true,
            meta: Default::default(),
        };
        let out = assert_reparses(&seq);
        assert_eq!(&out, "[\"a,b\"]\n");
    }

    #[test]
    fn tagged_empty_block_container_puts_header_on_braces_line() {
        let seq = CustomNode::Sequence {
            items: vec![],
            flow_style: false,
            meta: NodeMeta {
                tag: Some(Tag {
                    handle: "!".into(),
                    suffix: "a".into(),
                }),
                ..Default::default()
            },
        };
        let out = assert_reparses(&seq);
        assert_eq!(&out, "!a []\n");
    }

    #[test]
    fn seq_item_value_with_standalone_comment_is_not_compacted() {
        let mut pairs = IndexMap::new();
        pairs.insert(
            CustomNode::plain_scalar("0"),
            CustomNode::Null {
                meta: NodeMeta {
                    comment: Some(crate::ast::Comment {
                        text: "note".into(),
                        standalone: true,
                    }),
                    ..Default::default()
                },
            },
        );
        let seq = CustomNode::Sequence {
            items: vec![CustomNode::Mapping {
                pairs,
                flow_style: false,
                meta: Default::default(),
            }],
            flow_style: false,
            meta: Default::default(),
        };
        assert_reparses(&seq);
    }

    #[test]
    fn test_serialize_max_depth_exceeded() {
        let inner = CustomNode::plain_scalar("leaf");
        let mut current = inner;
        for _ in 0..200 {
            let mut m = IndexMap::<CustomNode, CustomNode>::new();
            m.insert(CustomNode::plain_scalar("a"), current);
            current = CustomNode::plain_mapping(m);
        }
        let options = SerializeOptions {
            indent_size: 2,
            explicit_start: false,
            explicit_end: false,
            sort_keys: false,
            max_depth: 50,
            ..Default::default()
        };
        let result = to_yaml_with_options(&current, &options);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(SerializeError::MaxDepthExceeded(DepthError(50)))
        ));
    }

    #[test]
    fn test_serialize_indent_mapping() {
        let mut inner = IndexMap::new();
        inner.insert(CustomNode::plain_scalar("b"), CustomNode::plain_scalar("1"));
        let inner_map = CustomNode::Mapping {
            pairs: inner,
            flow_style: false,
            meta: Default::default(),
        };
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar("a"), inner_map);
        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
        let options = SerializeOptions {
            indent_mapping: 4,
            ..Default::default()
        };
        assert_eq!(
            to_yaml_with_options(&node, &options).unwrap(),
            "a:\n    b: 1\n"
        );
    }

    #[test]
    fn test_serialize_indent_sequence() {
        let inner_seq = CustomNode::Sequence {
            items: vec![CustomNode::plain_scalar("1"), CustomNode::plain_scalar("2")],
            flow_style: false,
            meta: Default::default(),
        };
        let node = CustomNode::Sequence {
            items: vec![inner_seq],
            flow_style: false,
            meta: Default::default(),
        };
        let options = SerializeOptions {
            indent_sequence: 4,
            ..Default::default()
        };
        assert_eq!(
            to_yaml_with_options(&node, &options).unwrap(),
            "- \n    - 1\n    - 2\n"
        );
    }

    #[test]
    fn test_serialize_indent_offset() {
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar("a"), CustomNode::plain_scalar("1"));
        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };
        let options = SerializeOptions {
            indent_offset: 2,
            ..Default::default()
        };
        assert_yaml_eq!(to_yaml_with_options(&node, &options).unwrap(), "  a: 1\n");
    }

    #[test]
    fn test_standalone_comment_before_simple_key() {
        let yaml = "a: 1\n# c1\nb: 2\n";
        let ast = crate::parser::parse_with_options(
            yaml,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap();
        assert_yaml_eq!(crate::serializer::to_yaml(&ast), "a: 1\n# c1\nb: 2\n");
    }

    #[test]
    fn test_standalone_comment_before_nested_key() {
        let yaml = "top:\n  x: 1\n  # c2\n  y: 2\n";
        let ast = crate::parser::parse_with_options(
            yaml,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap();
        assert_yaml_eq!(
            crate::serializer::to_yaml(&ast),
            "top:\n  x: 1\n  # c2\n  y: 2\n"
        );
    }

    #[test]
    fn test_wrap_plain_scalar_does_not_split_multibyte() {
        // A value longer than the wrap width with a 4-byte char straddling the
        // continuation slice (byte 78). `wrap_plain_scalar` must floor the
        // slice to a char boundary instead of panicking, and must never split
        // inside the multi-byte character.
        let value =
            format!("x {}y", "y".repeat(75)) + &char::from_u32(0x10a09b).unwrap().to_string();
        let mut out = String::new();
        wrap_plain_scalar(&mut out, &value, 80, 2);
        assert!(out.contains('\u{10a09b}'), "multibyte char lost: {out:?}");
        assert!(out.contains("yyy"), "wrapped output incomplete: {out:?}");
    }
}
