use crate::ast::{Chomping, CustomNode, ScalarStyle, Tag};

/// Serialization options
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
pub fn to_yaml(node: &CustomNode) -> String {
    to_yaml_with_options(node, &SerializeOptions::default()).expect("serialization failed")
}

/// Serialize with custom options
pub fn to_yaml_with_options(
    node: &CustomNode,
    options: &SerializeOptions,
) -> Result<String, String> {
    let mut serializer = Serializer::new(options);
    if options.explicit_start {
        serializer.output.push_str("---\n");
    }
    serializer.serialize_node_internal(node, options.indent_offset, false, false, 0)?;
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
        _is_last: bool,
        in_value_context: bool,
        depth: usize,
    ) -> Result<(), String> {
        if depth >= self.max_depth {
            return Err(format!("max depth exceeded (max={})", self.max_depth));
        }

        // Handle standalone comments first
        if let Some(comment) = node.comment() {
            if comment.standalone {
                self.write_indent(indent_width);
                self.output.push_str("# ");
                self.output.push_str(&comment.text);
                self.output.push('\n');
            }
        }

        match node {
            CustomNode::Scalar {
                value,
                style,
                comment,
                anchor,
                tag,
                chomping,
                ..
            } => {
                self.write_indent(indent_width);
                if anchor.is_some() || tag.is_some() {
                    self.write_anchor_tag(anchor, tag);
                }

                self.write_scalar(value, style, chomping, self.width);
                if let Some(c) = comment {
                    if !c.standalone {
                        self.output.push_str("  # ");
                        self.output.push_str(&c.text);
                    }
                }

                self.output.push('\n');
            }
            CustomNode::Mapping {
                pairs,
                comment,
                anchor,
                tag,
                flow_style,
                ..
            } => {
                if *flow_style {
                    // Flow style: { key: value, key2: value2 }
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
                    if let Some(c) = comment {
                        if !c.standalone {
                            self.output.push_str("  # ");
                            self.output.push_str(&c.text);
                        }
                    }
                    self.output.push('\n');
                } else {
                    // Block style (original logic)
                    // Write anchor and tag if present (but not if in value context - they were already output)
                    if !in_value_context && (anchor.is_some() || tag.is_some()) {
                        self.write_indent(indent_width);
                        self.write_anchor_tag(anchor, tag);
                        self.output.push('\n');
                    }

                    if self.sort_keys {
                        let mut pairs_vec: Vec<(&CustomNode, &CustomNode)> = pairs.iter().collect();
                        pairs_vec.sort_by(|a, b| {
                            let ka = match a.0 {
                                CustomNode::Scalar { value, .. } => value.as_str(),
                                _ => "",
                            };
                            let kb = match b.0 {
                                CustomNode::Scalar { value, .. } => value.as_str(),
                                _ => "",
                            };
                            ka.cmp(kb)
                        });

                        for (i, (key, value)) in pairs_vec.iter().copied().enumerate() {
                            self.write_mapping_pair(
                                key,
                                value,
                                indent_width,
                                i == pairs_vec.len() - 1,
                                depth,
                            )?;
                        }
                    } else {
                        for (i, (key, value)) in pairs.iter().enumerate() {
                            self.write_mapping_pair(
                                key,
                                value,
                                indent_width,
                                i == pairs.len() - 1,
                                depth,
                            )?;
                        }
                    }

                    // Write mapping comment
                    if let Some(c) = comment {
                        if !c.standalone {
                            self.write_indent(indent_width);
                            self.output.push_str("# ");
                            self.output.push_str(&c.text);
                            self.output.push('\n');
                        }
                    }
                } // end block style
            }
            CustomNode::Sequence {
                items,
                comment,
                anchor,
                tag,
                flow_style,
                ..
            } => {
                if *flow_style {
                    // Flow style: [ item1, item2 ]
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
                    if let Some(c) = comment {
                        if !c.standalone {
                            self.output.push_str("  # ");
                            self.output.push_str(&c.text);
                        }
                    }
                    self.output.push('\n');
                } else {
                    // Block style (original logic)
                    // Write anchor and tag if present (but not if in value context)
                    if !in_value_context && (anchor.is_some() || tag.is_some()) {
                        self.write_indent(indent_width);
                        self.write_anchor_tag(anchor, tag);
                        self.output.push('\n');
                    }

                    for (i, item) in items.iter().enumerate() {
                        self.write_sequence_item(item, indent_width, i == items.len() - 1, depth)?;
                    }

                    // Write sequence comment
                    if let Some(c) = comment {
                        if !c.standalone {
                            self.write_indent(indent_width);
                            self.output.push_str("# ");
                            self.output.push_str(&c.text);
                            self.output.push('\n');
                        }
                    }
                } // end block style
            }
            CustomNode::Null {
                comment,
                anchor,
                tag,
                ..
            } => {
                self.write_indent(indent_width);
                if anchor.is_some() || tag.is_some() {
                    self.write_anchor_tag(anchor, tag);
                }

                self.output.push_str("null");
                if let Some(c) = comment {
                    if !c.standalone {
                        self.output.push_str("  # ");
                        self.output.push_str(&c.text);
                    }
                }

                self.output.push('\n');
            }
            CustomNode::Alias { name } => {
                self.write_indent(indent_width);
                self.output.push('*');
                self.output.push_str(name);
                self.output.push('\n');
            }
        }
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
    /// a new line, or inline). `is_last` is true when this is the final pair.
    pub(crate) fn write_mapping_pair(
        &mut self,
        key: &CustomNode,
        value: &CustomNode,
        indent_width: usize,
        is_last: bool,
        depth: usize,
    ) -> Result<(), String> {
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
            self.serialize_node_internal(key, indent_width, false, false, depth + 1)?;
        } else {
            // Simple key
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
                is_last,
                true,
                depth + 1,
            )?;
        } else {
            self.output.push(' ');
            self.serialize_node_internal(value, 0, is_last, true, depth + 1)?;
        }

        Ok(())
    }

    /// Write one `- ` item of a block sequence, keeping the compact inline
    /// form for plain mappings on the dash line. `is_last` is true when this
    /// is the final item.
    pub(crate) fn write_sequence_item(
        &mut self,
        item: &CustomNode,
        indent_width: usize,
        is_last: bool,
        depth: usize,
    ) -> Result<(), String> {
        self.write_indent(indent_width);
        self.output.push_str("- ");

        match item {
            // Compact form: `- key: value` with subsequent keys
            // indented to align under the first key. Only when the
            // mapping carries no metadata and every key/value can
            // share the dash line.
            CustomNode::Mapping {
                pairs,
                comment: None,
                anchor: None,
                tag: None,
                flow_style: false,
                ..
            } if !pairs.is_empty()
                && pairs.iter().all(|(k, v)| {
                    !matches!(k, CustomNode::Mapping { .. } | CustomNode::Sequence { .. })
                        && inlineable_value(v)
                }) =>
            {
                for (pi, (key, value)) in pairs.iter().enumerate() {
                    if pi > 0 {
                        self.write_indent(indent_width + self.indent_sequence);
                    }
                    self.write_scalar_for_key(key);
                    self.output.push(':');
                    self.output.push(' ');
                    self.serialize_node_internal(
                        value,
                        0,
                        is_last && pi == pairs.len() - 1,
                        true,
                        depth + 1,
                    )?;
                }
            }
            CustomNode::Mapping {
                flow_style: false, ..
            }
            | CustomNode::Sequence {
                flow_style: false, ..
            } => {
                self.output.push('\n');
                self.serialize_node_internal(
                    item,
                    indent_width + self.indent_sequence,
                    is_last,
                    false,
                    depth + 1,
                )?;
            }
            _ => {
                // For simple items (including flow-style containers),
                // they go on the same line as the dash. Don't pass
                // indent_width to avoid extra indentation.
                self.serialize_node_internal(item, 0, is_last, false, depth + 1)?;
            }
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
                if value.len() <= 8 && value.bytes().all(|b| b.is_ascii_alphanumeric()) {
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
        if value.len() <= 8 && value.bytes().all(|b| b.is_ascii_alphanumeric()) {
            self.output.push_str(value);
            return;
        }
        if value.is_empty()
            || value.contains(':')
            || value.contains('#')
            || value.starts_with('-')
            || value.starts_with('{')
            || value.starts_with('[')
            || value.starts_with('*')
            || value.starts_with('&')
            || value.starts_with('!')
            || value.starts_with('%')
            || value.starts_with('@')
            || value.starts_with('`')
            || value.contains('\n')
        {
            self.write_double_quoted_scalar(value);
        } else {
            // Width-based wrapping: if remaining > 0 and value is too long, wrap
            if remaining > 0 && value.len() > remaining {
                let split = value[..remaining].rfind(' ').unwrap_or(remaining);
                self.output.push_str(&value[..split]);
                let rest = value[split..].trim();
                if !rest.is_empty() {
                    self.output.push('\n');
                    let wrap_indent_str = " ".repeat(2);
                    let max_line = self.width;
                    let mut remaining_rest = rest;
                    while !remaining_rest.is_empty() {
                        self.output.push_str(&wrap_indent_str);
                        if remaining_rest.len() <= max_line.saturating_sub(wrap_indent_str.len()) {
                            self.output.push_str(remaining_rest);
                            break;
                        }
                        let avail = max_line.saturating_sub(wrap_indent_str.len());
                        if avail == 0 {
                            self.output.push_str(remaining_rest);
                            break;
                        }
                        let split_rest = remaining_rest[..avail].rfind(' ').unwrap_or(avail);
                        self.output.push_str(&remaining_rest[..split_rest]);
                        self.output.push('\n');
                        remaining_rest = remaining_rest[split_rest..].trim();
                    }
                }
            } else {
                self.output.push_str(value);
            }
        }
    }

    /// Write a single-quoted scalar (single quotes escaped by doubling).
    fn write_single_quoted_scalar(&mut self, value: &str) {
        self.output.push('\'');
        for c in value.chars() {
            if c == '\'' {
                self.output.push_str("''");
            } else {
                self.output.push(c);
            }
        }
        self.output.push('\'');
    }

    /// Write a double-quoted scalar with escape sequences.
    fn write_double_quoted_scalar(&mut self, value: &str) {
        self.output.push('"');
        for c in value.chars() {
            match c {
                '\\' => self.output.push_str("\\\\"),
                '"' => self.output.push_str("\\\""),
                '\n' => self.output.push_str("\\n"),
                '\r' => self.output.push_str("\\r"),
                '\t' => self.output.push_str("\\t"),
                '\0' => self.output.push_str("\\0"),
                '\x08' => self.output.push_str("\\b"),
                '\x0C' => self.output.push_str("\\f"),
                '\x1B' => self.output.push_str("\\e"),
                '/' => self.output.push_str("\\/"),
                _ if c.is_control() => {
                    self.output.push_str(&format!("\\u{:04x}", c as u32));
                }
                _ => self.output.push(c),
            }
        }
        self.output.push('"');
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
    fn serialize_flow_value(&mut self, node: &CustomNode, depth: usize) -> Result<(), String> {
        if depth >= self.max_depth {
            return Err(format!("max depth exceeded (max={})", self.max_depth));
        }

        match node {
            CustomNode::Scalar {
                value,
                style,
                anchor,
                tag,
                ..
            } => {
                if anchor.is_some() || tag.is_some() {
                    self.write_anchor_tag(anchor, tag);
                }
                self.write_scalar(value, style, &Chomping::Clip, self.width);
            }
            CustomNode::Null { anchor, tag, .. } => {
                if anchor.is_some() || tag.is_some() {
                    self.write_anchor_tag(anchor, tag);
                }
                self.output.push_str("null");
            }
            CustomNode::Mapping {
                pairs, anchor, tag, ..
            } => {
                if anchor.is_some() || tag.is_some() {
                    self.write_anchor_tag(anchor, tag);
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
            CustomNode::Sequence {
                items, anchor, tag, ..
            } => {
                if anchor.is_some() || tag.is_some() {
                    self.write_anchor_tag(anchor, tag);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Tag;
    use indexmap::IndexMap;

    #[test]
    fn test_pair_helper_matches_full_serialize() {
        let yaml = "a: 1\nb:\n  c: 2\n";
        let (ast, _) = crate::parser::parse_with_options(
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
        for (i, (k, v)) in pairs.iter().enumerate() {
            s.write_mapping_pair(k, v, 0, i == pairs.len() - 1, 0)
                .unwrap();
        }
        assert_eq!(full, s.output);
    }

    #[test]
    fn test_item_helper_matches_full_serialize() {
        // mixed items: simple scalar + compact mapping (multi-key alignment) + block container
        let yaml = "- a\n- b: c\n  d: 1\n- - 1\n  - 2\n";
        let (ast, _) = crate::parser::parse_with_options(
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
        for (i, item) in items.iter().enumerate() {
            s.write_sequence_item(item, 0, i == items.len() - 1, 0)
                .unwrap();
        }
        assert_eq!(full, s.output);
    }

    #[test]
    fn test_item_helper_preserves_compact_dash() {
        let yaml = "- host: a\n";
        let (ast, _) = crate::parser::parse_with_options(
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
        s.write_sequence_item(&items[0], 0, true, 0).unwrap();
        assert_eq!(s.output, "- host: a\n"); // P2: dash prefix must survive
    }

    #[test]
    fn test_serialize_plain_scalar() {
        let node = CustomNode::Scalar {
            value: "hello".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
            source_range: None,
        };
        assert_eq!(to_yaml(&node), "hello\n");
    }

    #[test]
    fn test_serialize_scalar_with_comment() {
        let node = CustomNode::Scalar {
            value: "value".to_string(),
            style: ScalarStyle::Plain,
            comment: Some(crate::ast::Comment {
                text: "a comment".to_string(),
                standalone: false,
            }),
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
            source_range: None,
        };
        assert_eq!(to_yaml(&node), "value  # a comment\n");
    }

    #[test]
    fn test_serialize_scalar_with_tag() {
        let node = CustomNode::Scalar {
            value: "42".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: Some(Tag::primary("int")),
            chomping: Chomping::Clip,
            source_range: None,
        };
        assert_eq!(to_yaml(&node), "!!int 42\n");
    }

    #[test]
    fn test_serialize_mapping() {
        let key = CustomNode::Scalar {
            value: "key".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
            source_range: None,
        };
        let value = CustomNode::Scalar {
            value: "value".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
            source_range: None,
        };

        let mut pairs = IndexMap::new();
        pairs.insert(key, value);

        let node = CustomNode::Mapping {
            pairs,
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
            source_range: None,
        };

        assert_eq!(to_yaml(&node), "key: value\n");
    }

    #[test]
    fn test_serialize_complex_key() {
        let key = CustomNode::Sequence {
            items: vec![
                CustomNode::Scalar {
                    value: "key1".to_string(),
                    style: ScalarStyle::Plain,
                    comment: None,
                    anchor: None,
                    tag: None,
                    chomping: Chomping::Clip,
                    source_range: None,
                },
                CustomNode::Scalar {
                    value: "key2".to_string(),
                    style: ScalarStyle::Plain,
                    comment: None,
                    anchor: None,
                    tag: None,
                    chomping: Chomping::Clip,
                    source_range: None,
                },
            ],
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
            source_range: None,
        };
        let value = CustomNode::Scalar {
            value: "value".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
            source_range: None,
        };

        let mut pairs = IndexMap::new();
        pairs.insert(key, value);

        let node = CustomNode::Mapping {
            pairs,
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
            source_range: None,
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
        assert!(result.unwrap_err().contains("max depth exceeded"));
    }

    #[test]
    fn test_serialize_indent_mapping() {
        let mut inner = IndexMap::new();
        inner.insert(CustomNode::plain_scalar("b"), CustomNode::plain_scalar("1"));
        let inner_map = CustomNode::Mapping {
            pairs: inner,
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
            source_range: None,
        };
        let mut pairs = IndexMap::new();
        pairs.insert(CustomNode::plain_scalar("a"), inner_map);
        let node = CustomNode::Mapping {
            pairs,
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
            source_range: None,
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
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
            source_range: None,
        };
        let node = CustomNode::Sequence {
            items: vec![inner_seq],
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
            source_range: None,
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
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
            source_range: None,
        };
        let options = SerializeOptions {
            indent_offset: 2,
            ..Default::default()
        };
        assert_eq!(to_yaml_with_options(&node, &options).unwrap(), "  a: 1\n");
    }
}
