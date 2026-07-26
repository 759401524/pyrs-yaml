use crate::ast::{Chomping, Comment, CustomNode, ScalarStyle, Tag};

/// Serialization options
pub struct SerializeOptions {
    pub indent_size: usize,
    pub explicit_start: bool,
    pub explicit_end: bool,
    pub sort_keys: bool,
}

impl Default for SerializeOptions {
    fn default() -> Self {
        Self {
            indent_size: 2,
            explicit_start: false,
            explicit_end: false,
            sort_keys: false,
        }
    }
}

/// Serialize a CustomNode AST back to YAML string
pub fn to_yaml(node: &CustomNode) -> String {
    to_yaml_with_options(node, &SerializeOptions::default())
}

/// Serialize with custom options
pub fn to_yaml_with_options(node: &CustomNode, options: &SerializeOptions) -> String {
    let mut serializer = Serializer::new(options);
    if options.explicit_start {
        serializer.output.push_str("---\n");
    }
    serializer.serialize_node_internal(node, 0, false, false);
    if options.explicit_end {
        serializer.output.push_str("...\n");
    }
    serializer.output
}

struct Serializer {
    output: String,
    indent_size: usize,
    sort_keys: bool,
    /// Pre-computed indent strings for levels 0..max_cached (avoids `repeat()` on every call)
    indent_cache: Vec<String>,
    /// Max depth we've cached; grown on demand
    max_cached: usize,
}

impl Serializer {
    fn new(options: &SerializeOptions) -> Self {
        let mut cache = Vec::with_capacity(64);
        cache.push(String::new()); // level 0 = empty
        Self {
            output: String::new(),
            indent_size: options.indent_size,
            sort_keys: options.sort_keys,
            indent_cache: cache,
            max_cached: 0,
        }
    }

    /// Ensure indent_cache has an entry for the given level, then write it to output.
    /// Does not return a reference to avoid overlapping borrows.
    fn write_indent(&mut self, level: usize) {
        if level > self.max_cached {
            let base_indent = " ".repeat(self.indent_size);
            let mut last = &self.indent_cache[self.max_cached];
            for _ in self.max_cached + 1..=level {
                let next = format!("{}{}", last, base_indent);
                self.indent_cache.push(next);
                last = self.indent_cache.last().unwrap();
            }
            self.max_cached = level;
        }
        self.output.push_str(&self.indent_cache[level]);
    }

    /// Ensure indent_cache has an entry for the given level, then return a reference to it.
    fn get_indent(&mut self, level: usize) -> &str {
        if level > self.max_cached {
            // Grow cache to the requested level
            let base_indent = " ".repeat(self.indent_size);
            let mut last = &self.indent_cache[self.max_cached];
            for _ in self.max_cached + 1..=level {
                let next = format!("{}{}", last, base_indent);
                self.indent_cache.push(next);
                last = self.indent_cache.last().unwrap();
            }
            self.max_cached = level;
        }
        &self.indent_cache[level]
    }

    /// 写入锚点（`&name`）和标签（`!!type`）前缀。
    fn write_anchor_tag(&mut self, anchor: &Option<String>, tag: &Option<Tag>) {
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

    /// 写入行内注释（`  # text`），跳过独立行注释。
    fn write_inline_comment(&mut self, comment: &Option<Comment>) {
        if let Some(c) = comment {
            if !c.standalone {
                self.output.push_str("  # ");
                self.output.push_str(&c.text);
            }
        }
    }

    /// 核心递归序列化方法，处理所有节点类型的缩进和格式化。
    fn serialize_node_internal(
        &mut self,
        node: &CustomNode,
        indent_level: usize,
        _is_last: bool,
        in_value_context: bool,
    ) {
        // Handle standalone comments first
        if let Some(comment) = node.comment() {
            if comment.standalone {
                self.write_indent(indent_level);
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
            } => {
                self.write_indent(indent_level);
                self.write_anchor_tag(anchor, tag);

                self.write_scalar(value, style, chomping);
                self.write_inline_comment(comment);

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
                            self.serialize_flow_value(value);
                        }
                    }
                    self.output.push('}');
                    self.write_inline_comment(comment);
                    self.output.push('\n');
                } else {
                    // Block style (original logic)
                    // Write anchor and tag if present (but not if in value context - they were already output)
                    if !in_value_context && (anchor.is_some() || tag.is_some()) {
                        self.write_indent(indent_level);
                        self.write_anchor_tag(anchor, tag);
                        self.output.push('\n');
                    }

                    // Collect pairs, optionally sorted
                    let pairs_vec: Vec<(&CustomNode, &CustomNode)> = if self.sort_keys {
                        let mut v: Vec<(&CustomNode, &CustomNode)> = pairs.iter().collect();
                        v.sort_by(|a, b| {
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
                        v
                    } else {
                        pairs.iter().collect()
                    };

                    for (i, (key, value)) in pairs_vec.iter().enumerate() {
                        // Check if key is a complex key (mapping or sequence)
                        let is_complex_key = matches!(
                            key,
                            CustomNode::Mapping { .. } | CustomNode::Sequence { .. }
                        );

                        if is_complex_key {
                            // Complex key: use ? indicator
                            self.write_indent(indent_level);
                            self.output.push_str("? ");
                            // For complex keys, we need to serialize at the same indent level
                            // but the key content should be indented relative to the ?
                            self.serialize_node_internal(key, indent_level, false, false);
                        } else {
                            // Simple key
                            self.write_indent(indent_level);
                            self.write_scalar_for_key(key);
                        }

                        self.output.push(':');

                        // Check if value needs to be on next line
                        if self.needs_newline_for_value(value) || is_complex_key {
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
                                indent_level + 1,
                                i == pairs_vec.len() - 1,
                                true,
                            );
                        } else {
                            self.output.push(' ');
                            self.serialize_node_internal(value, 0, i == pairs_vec.len() - 1, true);
                        }
                    }

                    // Write mapping comment
                    if let Some(c) = comment {
                        if !c.standalone {
                            self.write_indent(indent_level);
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
                            self.serialize_flow_value(item);
                        }
                    }
                    self.output.push(']');
                    self.write_inline_comment(comment);
                    self.output.push('\n');
                } else {
                    // Block style (original logic)
                    // Write anchor and tag if present (but not if in value context)
                    if !in_value_context && (anchor.is_some() || tag.is_some()) {
                        self.write_indent(indent_level);
                        self.write_anchor_tag(anchor, tag);
                        self.output.push('\n');
                    }

                    for (i, item) in items.iter().enumerate() {
                        self.write_indent(indent_level);
                        self.output.push_str("- ");

                        if self.needs_newline_for_sequence_item(item) {
                            self.output.push('\n');
                            self.serialize_node_internal(
                                item,
                                indent_level + 1,
                                i == items.len() - 1,
                                false,
                            );
                        } else {
                            // For simple items, they go on the same line as the dash
                            // Don't pass indent_level to avoid extra indentation
                            self.serialize_node_internal(item, 0, i == items.len() - 1, false);
                        }
                    }

                    // Write sequence comment
                    if let Some(c) = comment {
                        if !c.standalone {
                            self.write_indent(indent_level);
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
            } => {
                self.write_indent(indent_level);
                self.write_anchor_tag(anchor, tag);

                self.output.push_str("null");
                self.write_inline_comment(comment);

                self.output.push('\n');
            }
            CustomNode::Alias { name } => {
                self.write_indent(indent_level);
                self.output.push('*');
                self.output.push_str(name);
                self.output.push('\n');
            }
        }
    }

    /// Write a scalar value directly to output based on style and chomping.
    fn write_scalar(&mut self, value: &str, style: &ScalarStyle, chomping: &Chomping) {
        match style {
            ScalarStyle::Plain => self.write_plain_scalar(value),
            ScalarStyle::SingleQuoted => self.write_single_quoted_scalar(value),
            ScalarStyle::DoubleQuoted => self.write_double_quoted_scalar(value),
            ScalarStyle::Literal => self.write_literal_scalar(value, chomping),
            ScalarStyle::Folded => self.write_folded_scalar(value, chomping),
        }
    }

    /// Write a scalar formatted as a mapping key.
    fn write_scalar_for_key(&mut self, node: &CustomNode) {
        match node {
            CustomNode::Scalar {
                value,
                style,
                chomping,
                ..
            } => self.write_scalar(value, style, chomping),
            _ => {
                self.output.push_str("null");
            }
        }
    }

    /// Write a plain scalar, quoting if necessary.
    fn write_plain_scalar(&mut self, value: &str) {
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
            self.output.push_str(value);
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
        self.write_base_indent(value, 1);
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
        self.write_base_indent(value, 1);
    }

    /// Write each line of the block scalar content with base indentation appended.
    /// Writes directly to output (no intermediate Vec or join).
    fn write_base_indent(&mut self, value: &str, base_indent: usize) {
        let indent = self.get_indent(base_indent);
        let indent = indent.to_string();
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

    /// 判断值节点是否需要换行显示（非 flow 风格的映射/序列）。
    fn needs_newline_for_value(&self, node: &CustomNode) -> bool {
        match node {
            CustomNode::Mapping { flow_style, .. } => !flow_style,
            CustomNode::Sequence { flow_style, .. } => !flow_style,
            _ => false,
        }
    }

    /// 判断序列项是否需要换行显示（映射或序列类型）。
    fn needs_newline_for_sequence_item(&self, node: &CustomNode) -> bool {
        matches!(
            node,
            CustomNode::Mapping { .. } | CustomNode::Sequence { .. }
        )
    }

    /// 在 flow 上下文中序列化值（不追加换行符）。
    fn serialize_flow_value(&mut self, node: &CustomNode) {
        match node {
            CustomNode::Scalar {
                value,
                style,
                anchor,
                tag,
                ..
            } => {
                self.write_anchor_tag(anchor, tag);
                self.write_scalar(value, style, &Chomping::Clip);
            }
            CustomNode::Null { anchor, tag, .. } => {
                self.write_anchor_tag(anchor, tag);
                self.output.push_str("null");
            }
            CustomNode::Mapping {
                pairs, anchor, tag, ..
            } => {
                self.write_anchor_tag(anchor, tag);
                self.output.push('{');
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.write_scalar_for_key(key);
                    self.output.push_str(": ");
                    self.serialize_flow_value(value);
                }
                self.output.push('}');
            }
            CustomNode::Sequence {
                items, anchor, tag, ..
            } => {
                self.write_anchor_tag(anchor, tag);
                self.output.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.serialize_flow_value(item);
                }
                self.output.push(']');
            }
            CustomNode::Alias { name } => {
                self.output.push('*');
                self.output.push_str(name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Tag;
    use indexmap::IndexMap;

    #[test]
    fn test_serialize_plain_scalar() {
        let node = CustomNode::Scalar {
            value: "hello".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
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
        };
        let value = CustomNode::Scalar {
            value: "value".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
        };

        let mut pairs = IndexMap::new();
        pairs.insert(key, value);

        let node = CustomNode::Mapping {
            pairs,
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
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
                },
                CustomNode::Scalar {
                    value: "key2".to_string(),
                    style: ScalarStyle::Plain,
                    comment: None,
                    anchor: None,
                    tag: None,
                    chomping: Chomping::Clip,
                },
            ],
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
        };
        let value = CustomNode::Scalar {
            value: "value".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
        };

        let mut pairs = IndexMap::new();
        pairs.insert(key, value);

        let node = CustomNode::Mapping {
            pairs,
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
        };

        let output = to_yaml(&node);
        assert!(output.contains("? "));
    }
}
