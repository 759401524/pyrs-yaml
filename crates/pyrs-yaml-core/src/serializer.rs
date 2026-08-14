use crate::ast::{Chomping, CustomNode, NodeMeta, ScalarStyle, Tag};
use crate::error::{DepthError, SerializeError};
use crate::parser::yaml::schema::core_type_is_non_string;
use indexmap::IndexMap;

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
    serializer.serialize_node_internal(node, options.indent_offset, false, 0)?;
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
            self.output.push('&');
            self.output.push_str(anchor_name);
            self.output.push(' ');
        }
        if let Some(t) = tag {
            self.output.push_str(&t.to_string());
            self.output.push(' ');
        }
    }

    /// 核心递归序列化方法，处理所有节点类型的缩进和格式化。
    fn serialize_node_internal(
        &mut self,
        node: &CustomNode,
        indent_width: usize,
        in_value_context: bool,
        depth: usize,
    ) -> Result<(), SerializeError> {
        if depth >= self.max_depth {
            return Err(SerializeError::MaxDepthExceeded(DepthError(self.max_depth)));
        }

        // Handle standalone comments first
        if let Some(comment) = node.comment()
            && comment.standalone
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
                ..
            } => self.write_scalar_node(value, style, meta, chomping, indent_width)?,
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
        chomping: &Chomping,
        indent_width: usize,
    ) -> Result<(), SerializeError> {
        self.write_indent(indent_width);
        if meta.anchor.is_some() || meta.tag.is_some() {
            self.write_anchor_tag(&meta.anchor, &meta.tag);
        }
        self.write_scalar(value, style, chomping, self.width);
        if let Some(c) = &meta.comment
            && !c.standalone
        {
            self.output.push_str("  # ");
            self.output.push_str(&c.text);
        }
        self.output.push('\n');
        Ok(())
    }

    /// Serialize a mapping node in either flow (`{ ... }`) or block style,
    /// including anchor/tag and trailing comment. Extracted from
    /// `serialize_node_internal`.
    fn write_mapping_node(
        &mut self,
        pairs: &IndexMap<CustomNode, CustomNode>,
        meta: &NodeMeta,
        flow_style: bool,
        indent_width: usize,
        depth: usize,
        in_value_context: bool,
    ) -> Result<(), SerializeError> {
        if flow_style {
            self.output.push('{');
            if !pairs.is_empty() {
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.write_scalar_for_key(key);
                    self.output.push_str(": ");
                    self.serialize_flow_value(value, depth + 1)?;
                }
            }
            self.output.push('}');
            if let Some(c) = &meta.comment
                && !c.standalone
            {
                self.output.push_str("  # ");
                self.output.push_str(&c.text);
            }
            self.output.push('\n');
        } else {
            if !in_value_context && (meta.anchor.is_some() || meta.tag.is_some()) {
                self.write_indent(indent_width);
                self.write_anchor_tag(&meta.anchor, &meta.tag);
                self.output.push('\n');
            }

            if pairs.is_empty() {
                self.write_indent(indent_width);
                self.output.push_str("{}\n");
                return Ok(());
            }

            if self.sort_keys {
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
                    self.write_mapping_pair(key, value, indent_width, depth)?;
                }
            } else {
                for (key, value) in pairs.iter() {
                    self.write_mapping_pair(key, value, indent_width, depth)?;
                }
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

    /// Serialize a sequence node in either flow (`[ ... ]`) or block style,
    /// including anchor/tag and trailing comment. Extracted from
    /// `serialize_node_internal`.
    fn write_sequence_node(
        &mut self,
        items: &[CustomNode],
        meta: &NodeMeta,
        flow_style: bool,
        indent_width: usize,
        depth: usize,
        in_value_context: bool,
    ) -> Result<(), SerializeError> {
        if flow_style {
            self.output.push('[');
            if !items.is_empty() {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.serialize_flow_value(item, depth + 1)?;
                }
            }
            self.output.push(']');
            if let Some(c) = &meta.comment
                && !c.standalone
            {
                self.output.push_str("  # ");
                self.output.push_str(&c.text);
            }
            self.output.push('\n');
        } else {
            if !in_value_context && (meta.anchor.is_some() || meta.tag.is_some()) {
                self.write_indent(indent_width);
                self.write_anchor_tag(&meta.anchor, &meta.tag);
                self.output.push('\n');
            }

            if items.is_empty() {
                self.write_indent(indent_width);
                self.output.push_str("[]\n");
                return Ok(());
            }

            for item in items.iter() {
                self.write_sequence_item(item, indent_width, depth)?;
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
    fn write_scalar(
        &mut self,
        value: &str,
        style: &ScalarStyle,
        chomping: &Chomping,
        remaining: usize,
    ) {
        match style {
            ScalarStyle::Plain => self.write_plain_scalar(value, remaining),
            ScalarStyle::SingleQuoted => self.write_single_quoted_scalar(value),
            ScalarStyle::DoubleQuoted => self.write_double_quoted_scalar(value),
            ScalarStyle::Literal => self.write_literal_scalar(value, chomping),
            ScalarStyle::Folded => self.write_folded_scalar(value, chomping),
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
            // Complex key: use ? indicator
            self.write_indent(indent_width);
            self.output.push_str("? ");
            // For complex keys, we need to serialize at the same indent level
            // but the key content should be indented relative to the ?
            self.serialize_node_internal(key, indent_width, false, depth + 1)?;
        } else {
            // Simple key
            // Handle standalone comments before the key
            if let Some(comment) = key.comment()
                && comment.standalone
            {
                self.write_indent(indent_width);
                self.output.push_str("# ");
                self.output.push_str(&comment.text);
                self.output.push('\n');
            }
            self.write_indent(indent_width);
            self.write_scalar_for_key(key);
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
        )) || is_complex_key
        {
            // If the value node has an anchor or tag, write it after the colon
            if let Some(anchor_name) = value.anchor() {
                self.output.push_str(" &");
                self.output.push_str(anchor_name);
            }
            if let Some(t) = value.tag() {
                self.output.push(' ');
                self.output.push_str(&t.to_string());
            }
            self.output.push('\n');
            self.serialize_node_internal(
                value,
                indent_width + self.indent_mapping,
                true,
                depth + 1,
            )?;
        } else {
            self.output.push(' ');
            self.serialize_node_internal(value, 0, true, depth + 1)?;
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
                self.write_scalar_for_key(key);
                self.output.push(':');
                self.output.push(' ');
                self.serialize_node_internal(value, 0, true, depth + 1)?;
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
        ) {
            self.output.push('\n');
            self.serialize_node_internal(
                item,
                indent_width + self.indent_sequence,
                false,
                depth + 1,
            )?;
        } else {
            // For simple items (including flow-style containers),
            // they go on the same line as the dash. Don't pass
            // indent_width to avoid extra indentation.
            self.serialize_node_internal(item, 0, false, depth + 1)?;
        }

        Ok(())
    }

    /// Write a scalar formatted as a mapping key.
    fn write_scalar_for_key(&mut self, node: &CustomNode) {
        match node {
            CustomNode::Scalar {
                value,
                style: ScalarStyle::Plain,
                ..
            } => {
                if is_short_alphanumeric(value) {
                    self.output.push_str(value);
                } else {
                    self.write_plain_scalar(value, 0);
                }
            }
            CustomNode::Scalar {
                value,
                style,
                chomping,
                ..
            } => self.write_scalar(value, style, chomping, 0),
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

    /// Write a literal block scalar (`|`) with chomping indicator.
    fn write_literal_scalar(&mut self, value: &str, chomping: &Chomping) {
        let indicator = match chomping {
            Chomping::Strip => "|-",
            Chomping::Clip => "|",
            Chomping::Keep => "|+",
        };
        self.output.push_str(indicator);
        self.output.push('\n');
        self.write_base_indent(value);
    }

    /// Write a folded block scalar (`>`) with chomping indicator.
    fn write_folded_scalar(&mut self, value: &str, chomping: &Chomping) {
        let indicator = match chomping {
            Chomping::Strip => ">-",
            Chomping::Clip => ">",
            Chomping::Keep => ">+",
        };
        self.output.push_str(indicator);
        self.output.push('\n');
        self.write_base_indent(value);
    }

    /// Write each line of the block scalar content with base indentation appended.
    /// Writes directly to output (no intermediate Vec or join).
    fn write_base_indent(&mut self, value: &str) {
        let indent = " ".repeat(self.indent_size);
        let mut first = true;
        for line in value.lines() {
            if !first {
                self.output.push('\n');
            }
            if !line.is_empty() {
                self.output.push_str(&indent);
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
                value, style, meta, ..
            } => {
                if meta.anchor.is_some() || meta.tag.is_some() {
                    self.write_anchor_tag(&meta.anchor, &meta.tag);
                }
                self.write_scalar(value, style, &Chomping::Clip, self.width);
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
                    self.write_scalar_for_key(key);
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
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
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

/// Append `value` to `out` as a plain scalar, double-quoting it if required
/// and wrapping to `width` when it overflows `remaining`.
pub fn write_plain_scalar(out: &mut String, value: &str, remaining: usize, width: usize) {
    if value.len() <= 8 && value.bytes().all(|b| b.is_ascii_alphanumeric()) {
        out.push_str(value);
        return;
    }
    if needs_double_quoted(value) {
        if value.contains('\\') && !value.contains('\n') {
            // granit-parser mishandles `\\<escape-letter>` inside double-quoted
            // scalars (e.g. `\\0` collapses to NUL). Single-quoted scalars keep
            // backslashes literal, so prefer them whenever the value contains a
            // backslash (and no newline, which single-quoted cannot represent).
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
                    out.push('\n');
                    wrap_plain_scalar(out, rest, width);
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

/// Wrap `value` to `width` with a fixed 2-space continuation indent. Used by
/// [`write_plain_scalar`] when a value overflows the current line.
pub fn wrap_plain_scalar(out: &mut String, value: &str, width: usize) {
    let wrap_indent_str = " ".repeat(2);
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
        match remaining_rest[..avail].rfind(' ') {
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

    #[test]
    fn test_pair_helper_matches_full_serialize() {
        let yaml = "a: 1\nb:\n  c: 2\n";
        let ast = crate::parser::parse_with_options(
            yaml,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap();
        let full = crate::serializer::to_yaml(&ast);
        let mut s = Serializer::new(&SerializeOptions::default());
        let CustomNode::Mapping { pairs, .. } = &ast else {
            panic!()
        };
        for (k, v) in pairs.iter() {
            s.write_mapping_pair(k, v, 0, 0).unwrap();
        }
        assert_eq!(full, s.output);
    }

    #[test]
    fn test_item_helper_matches_full_serialize() {
        // mixed items: simple scalar + compact mapping (multi-key alignment) + block container
        let yaml = "- a\n- b: c\n  d: 1\n- - 1\n  - 2\n";
        let ast = crate::parser::parse_with_options(
            yaml,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap();
        let full = crate::serializer::to_yaml(&ast);
        let mut s = Serializer::new(&SerializeOptions::default());
        let CustomNode::Sequence { items, .. } = &ast else {
            panic!()
        };
        for item in items.iter() {
            s.write_sequence_item(item, 0, 0).unwrap();
        }
        assert_eq!(full, s.output);
    }

    #[test]
    fn test_item_helper_preserves_compact_dash() {
        let yaml = "- host: a\n";
        let ast = crate::parser::parse_with_options(
            yaml,
            true,
            crate::parser::yaml::YamlSchema::Core,
            1000,
            false,
        )
        .unwrap();
        let mut s = Serializer::new(&SerializeOptions::default());
        let CustomNode::Sequence { items, .. } = &ast else {
            panic!()
        };
        s.write_sequence_item(&items[0], 0, 0).unwrap();
        assert_eq!(s.output, "- host: a\n"); // P2: dash prefix must survive
    }

    #[test]
    fn test_serialize_plain_scalar() {
        let node = CustomNode::Scalar {
            value: Arc::from("hello"),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: Default::default(),
        };
        assert_eq!(to_yaml(&node), "hello\n");
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
        };
        assert_eq!(to_yaml(&node), "value  # a comment\n");
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
        };
        assert_eq!(to_yaml(&node), "!!int 42\n");
    }

    #[test]
    fn test_serialize_mapping() {
        let key = CustomNode::Scalar {
            value: Arc::from("key"),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: Default::default(),
        };
        let value = CustomNode::Scalar {
            value: Arc::from("value"),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: Default::default(),
        };

        let mut pairs = IndexMap::new();
        pairs.insert(key, value);

        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };

        assert_eq!(to_yaml(&node), "key: value\n");
    }

    #[test]
    fn test_serialize_complex_key() {
        let key = CustomNode::Sequence {
            items: vec![
                CustomNode::Scalar {
                    value: Arc::from("key1"),
                    style: ScalarStyle::Plain,
                    chomping: Chomping::Clip,
                    meta: Default::default(),
                },
                CustomNode::Scalar {
                    value: Arc::from("key2"),
                    style: ScalarStyle::Plain,
                    chomping: Chomping::Clip,
                    meta: Default::default(),
                },
            ],
            flow_style: false,
            meta: Default::default(),
        };
        let value = CustomNode::Scalar {
            value: Arc::from("value"),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: Default::default(),
        };

        let mut pairs = IndexMap::new();
        pairs.insert(key, value);

        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
        };

        let output = to_yaml(&node);
        assert!(output.contains("? "));
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
        assert_eq!(to_yaml_with_options(&node, &options).unwrap(), "  a: 1\n");
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
        assert_eq!(crate::serializer::to_yaml(&ast), "a: 1\n# c1\nb: 2\n");
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
        assert_eq!(
            crate::serializer::to_yaml(&ast),
            "top:\n  x: 1\n  # c2\n  y: 2\n"
        );
    }
}
