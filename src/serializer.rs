use crate::ast::{Chomping, CustomNode, ScalarStyle};

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

#[allow(dead_code)]
struct Serializer {
    output: String,
    indent_size: usize,
    sort_keys: bool,
}

impl Serializer {
    fn new(options: &SerializeOptions) -> Self {
        Self {
            output: String::new(),
            indent_size: options.indent_size,
            sort_keys: options.sort_keys,
        }
    }

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

                // Write anchor if present
                if let Some(anchor_name) = anchor {
                    self.output.push('&');
                    self.output.push_str(anchor_name);
                    self.output.push(' ');
                }

                // Write tag if present
                if let Some(t) = tag {
                    self.output.push_str(&t.to_string());
                    self.output.push(' ');
                }

                // Write scalar with appropriate style
                let formatted = self.format_scalar(value, style, chomping);
                self.output.push_str(&formatted);

                // Write line-end comment
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
                            self.output.push_str(&self.format_scalar_for_key(key));
                            self.output.push_str(": ");
                            self.serialize_flow_value(value);
                        }
                    }
                    self.output.push('}');
                    // Write comment if present
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
                    self.write_indent(indent_level);
                    if let Some(anchor_name) = anchor {
                        self.output.push('&');
                        self.output.push_str(anchor_name);
                        self.output.push(' ');
                    }
                    if let Some(t) = tag {
                        self.output.push_str(&t.to_string());
                        self.output.push(' ');
                    }
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
                        self.output.push_str(&self.format_scalar_for_key(key));
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
                    // Write comment if present
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
                        self.write_indent(indent_level);
                        if let Some(anchor_name) = anchor {
                            self.output.push('&');
                            self.output.push_str(anchor_name);
                            self.output.push(' ');
                        }
                        if let Some(t) = tag {
                            self.output.push_str(&t.to_string());
                            self.output.push(' ');
                        }
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

                if let Some(anchor_name) = anchor {
                    self.output.push('&');
                    self.output.push_str(anchor_name);
                    self.output.push(' ');
                }

                if let Some(t) = tag {
                    self.output.push_str(&t.to_string());
                    self.output.push(' ');
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
                self.write_indent(indent_level);
                self.output.push('*');
                self.output.push_str(name);
                self.output.push('\n');
            }
        }
    }

    fn format_scalar(&self, value: &str, style: &ScalarStyle, chomping: &Chomping) -> String {
        match style {
            ScalarStyle::Plain => self.format_plain_scalar(value),
            ScalarStyle::SingleQuoted => self.format_single_quoted_scalar(value),
            ScalarStyle::DoubleQuoted => self.format_double_quoted_scalar(value),
            ScalarStyle::Literal => self.format_literal_scalar(value, chomping),
            ScalarStyle::Folded => self.format_folded_scalar(value, chomping),
        }
    }

    fn format_scalar_for_key(&self, node: &CustomNode) -> String {
        match node {
            CustomNode::Scalar { value, style, chomping, .. } => {
                match style {
                    ScalarStyle::Plain => self.format_plain_scalar(value),
                    ScalarStyle::SingleQuoted => self.format_single_quoted_scalar(value),
                    ScalarStyle::DoubleQuoted => self.format_double_quoted_scalar(value),
                    ScalarStyle::Literal => self.format_literal_scalar(value, chomping),
                    ScalarStyle::Folded => self.format_folded_scalar(value, chomping),
                }
            }
            _ => "null".to_string(),
        }
    }

    fn format_plain_scalar(&self, value: &str) -> String {
        // Check if the value needs quoting
        // Note: true/false/null/~ are valid plain scalars in YAML
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
            self.format_double_quoted_scalar(value)
        } else {
            value.to_string()
        }
    }

    fn format_single_quoted_scalar(&self, value: &str) -> String {
        // Escape single quotes by doubling them
        let escaped = value.replace('\'', "''");
        format!("'{}'", escaped)
    }

    fn format_double_quoted_scalar(&self, value: &str) -> String {
        let mut escaped = String::new();
        for c in value.chars() {
            match c {
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                '\0' => escaped.push_str("\\0"),
                '\x08' => escaped.push_str("\\b"),
                '\x0C' => escaped.push_str("\\f"),
                '\x1B' => escaped.push_str("\\e"),
                '/' => escaped.push_str("\\/"),
                _ if c.is_control() => {
                    // Unicode escape for control characters
                    escaped.push_str(&format!("\\u{:04x}", c as u32));
                }
                _ => escaped.push(c),
            }
        }
        format!("\"{}\"", escaped)
    }

    fn format_literal_scalar(&self, value: &str, chomping: &Chomping) -> String {
        let indicator = match chomping {
            Chomping::Strip => "|-",
            Chomping::Clip => "|",
            Chomping::Keep => "|+",
        };
        format!("{}\n{}", indicator, self.add_base_indent(value, 0))
    }

    fn format_folded_scalar(&self, value: &str, chomping: &Chomping) -> String {
        let indicator = match chomping {
            Chomping::Strip => ">-",
            Chomping::Clip => ">",
            Chomping::Keep => ">+",
        };
        format!("{}\n{}", indicator, self.add_base_indent(value, 0))
    }

    fn add_base_indent(&self, value: &str, base_indent: usize) -> String {
        let indent = " ".repeat(base_indent + self.indent_size);
        value
            .lines()
            .map(|line| {
                if line.is_empty() {
                    String::new()
                } else {
                    format!("{}{}", indent, line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn write_indent(&mut self, level: usize) {
        self.output.push_str(&" ".repeat(level * self.indent_size));
    }

    fn needs_newline_for_value(&self, node: &CustomNode) -> bool {
        match node {
            CustomNode::Mapping { flow_style, .. } => !flow_style,
            CustomNode::Sequence { flow_style, .. } => !flow_style,
            _ => false,
        }
    }

    fn needs_newline_for_sequence_item(&self, node: &CustomNode) -> bool {
        matches!(
            node,
            CustomNode::Mapping { .. } | CustomNode::Sequence { .. }
        )
    }

    /// Serialize a value in flow context (no trailing newline)
    fn serialize_flow_value(&mut self, node: &CustomNode) {
        match node {
            CustomNode::Scalar { value, style, anchor, tag, .. } => {
                if let Some(anchor_name) = anchor {
                    self.output.push('&');
                    self.output.push_str(anchor_name);
                    self.output.push(' ');
                }
                if let Some(t) = tag {
                    self.output.push_str(&t.to_string());
                    self.output.push(' ');
                }
                let formatted = match style {
                    ScalarStyle::Plain => self.format_plain_scalar(value),
                    ScalarStyle::SingleQuoted => self.format_single_quoted_scalar(value),
                    ScalarStyle::DoubleQuoted => self.format_double_quoted_scalar(value),
                    ScalarStyle::Literal => self.format_literal_scalar(value, &Chomping::Clip),
                    ScalarStyle::Folded => self.format_folded_scalar(value, &Chomping::Clip),
                };
                self.output.push_str(&formatted);
            }
            CustomNode::Null { anchor, tag, .. } => {
                if let Some(anchor_name) = anchor {
                    self.output.push('&');
                    self.output.push_str(anchor_name);
                    self.output.push(' ');
                }
                if let Some(t) = tag {
                    self.output.push_str(&t.to_string());
                    self.output.push(' ');
                }
                self.output.push_str("null");
            }
            CustomNode::Mapping { pairs, anchor, tag, .. } => {
                if let Some(anchor_name) = anchor {
                    self.output.push('&');
                    self.output.push_str(anchor_name);
                    self.output.push(' ');
                }
                if let Some(t) = tag {
                    self.output.push_str(&t.to_string());
                    self.output.push(' ');
                }
                self.output.push('{');
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(&self.format_scalar_for_key(key));
                    self.output.push_str(": ");
                    self.serialize_flow_value(value);
                }
                self.output.push('}');
            }
            CustomNode::Sequence { items, anchor, tag, .. } => {
                if let Some(anchor_name) = anchor {
                    self.output.push('&');
                    self.output.push_str(anchor_name);
                    self.output.push(' ');
                }
                if let Some(t) = tag {
                    self.output.push_str(&t.to_string());
                    self.output.push(' ');
                }
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
