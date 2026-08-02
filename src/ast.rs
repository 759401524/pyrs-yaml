use indexmap::IndexMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Range;

/// Scalar style preservation for round-trip support
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarStyle {
    /// Plain scalar (no quotes)
    Plain,
    /// Single-quoted scalar
    SingleQuoted,
    /// Double-quoted scalar
    DoubleQuoted,
    /// Literal block scalar (|)
    Literal,
    /// Folded block scalar (>)
    Folded,
}

/// Chomping indicator for block scalars (YAML 1.2)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Chomping {
    /// Strip chomping (-): final newline is stripped
    Strip,
    /// Clip chomping (default): single final newline kept
    #[default]
    Clip,
    /// Keep chomping (+): all newlines preserved
    Keep,
}

impl Hash for Chomping {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
    }
}

impl Hash for ScalarStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
    }
}

/// Comment attached to a node
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// The comment text (without # prefix)
    pub text: String,
    /// Whether this is a standalone line comment (true) or line-end comment (false)
    pub standalone: bool,
}

impl Hash for Comment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
        self.standalone.hash(state);
    }
}

/// YAML tag (e.g., !!str, !!int, !custom)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// The handle (e.g., "!!" or "!") - empty for verbatim tags
    pub handle: String,
    /// The suffix (e.g., "str", "int", "null")
    pub suffix: String,
}

impl Tag {
    /// Create a local tag (!suffix)
    pub fn local(suffix: &str) -> Self {
        Self {
            handle: "!".to_string(),
            suffix: suffix.to_string(),
        }
    }

    /// Create a primary tag (!!suffix)
    pub fn primary(suffix: &str) -> Self {
        Self {
            handle: "!!".to_string(),
            suffix: suffix.to_string(),
        }
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.handle == "!" && self.suffix.is_empty() {
            write!(f, "!")
        } else if self.handle == "!!" {
            write!(f, "!!{}", self.suffix)
        } else if self.handle == "!" {
            write!(f, "!{}", self.suffix)
        } else {
            write!(f, "{}{}", self.handle, self.suffix)
        }
    }
}

impl Hash for Tag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
        self.suffix.hash(state);
    }
}

/// Custom AST node with full metadata for round-trip support
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomNode {
    Scalar {
        value: String,
        style: ScalarStyle,
        comment: Option<Comment>,
        anchor: Option<String>,
        tag: Option<Tag>,
        /// Chomping indicator for block scalars (|+, |-, >+, >-)
        chomping: Chomping,
        /// Byte range of this node in the original source text
        source_range: Option<Range<usize>>,
    },
    Mapping {
        pairs: IndexMap<CustomNode, CustomNode>,
        comment: Option<Comment>,
        anchor: Option<String>,
        tag: Option<Tag>,
        /// Whether this mapping uses flow style ({key: value}) vs block style
        flow_style: bool,
        /// Byte range of this node in the original source text
        source_range: Option<Range<usize>>,
    },
    Sequence {
        items: Vec<CustomNode>,
        comment: Option<Comment>,
        anchor: Option<String>,
        tag: Option<Tag>,
        /// Whether this sequence uses flow style (\[item\]) vs block style
        flow_style: bool,
        /// Byte range of this node in the original source text
        source_range: Option<Range<usize>>,
    },
    Null {
        comment: Option<Comment>,
        anchor: Option<String>,
        tag: Option<Tag>,
        /// Byte range of this node in the original source text
        source_range: Option<Range<usize>>,
    },
    /// Alias reference (*alias)
    Alias { name: String },
}

impl Hash for CustomNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            CustomNode::Scalar {
                value,
                style,
                comment,
                anchor,
                tag,
                chomping,
                source_range,
            } => {
                state.write_u8(0);
                value.hash(state);
                style.hash(state);
                comment.hash(state);
                anchor.hash(state);
                tag.hash(state);
                chomping.hash(state);
                source_range.hash(state);
            }
            CustomNode::Mapping {
                pairs,
                comment,
                anchor,
                tag,
                flow_style,
                source_range,
            } => {
                state.write_u8(1);
                for (k, v) in pairs {
                    k.hash(state);
                    v.hash(state);
                }
                comment.hash(state);
                anchor.hash(state);
                tag.hash(state);
                flow_style.hash(state);
                source_range.hash(state);
            }
            CustomNode::Sequence {
                items,
                comment,
                anchor,
                tag,
                flow_style,
                source_range,
            } => {
                state.write_u8(2);
                for item in items {
                    item.hash(state);
                }
                comment.hash(state);
                anchor.hash(state);
                tag.hash(state);
                flow_style.hash(state);
                source_range.hash(state);
            }
            CustomNode::Null {
                comment,
                anchor,
                tag,
                source_range,
            } => {
                state.write_u8(3);
                comment.hash(state);
                anchor.hash(state);
                tag.hash(state);
                source_range.hash(state);
            }
            CustomNode::Alias { name } => {
                state.write_u8(4);
                name.hash(state);
            }
        }
    }
}

impl CustomNode {
    /// 创建一个无元数据的普通标量节点。
    ///
    /// 使用 `ScalarStyle::Plain` 和 `Chomping::Clip` 默认值，不附加注释、锚点或标签。
    ///
    /// # Arguments
    /// * `value` - 标量文本值，支持任何实现了 `Into<String>` 的类型。
    ///
    /// # Returns
    /// 返回一个 `CustomNode::Scalar` 变体，所有元数据字段为 `None`。
    ///
    /// # Examples
    /// ```ignore
    /// use pyrs_yaml::ast::CustomNode;
    ///
    /// let node = CustomNode::plain_scalar("hello");
    /// assert_eq!(node.comment(), None);
    /// ```
    pub fn plain_scalar(value: impl Into<String>) -> Self {
        CustomNode::Scalar {
            value: value.into(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
            source_range: None,
        }
    }

    /// 创建一个单引号风格的标量节点（用于表示负数等需要引号的值）。
    ///
    /// 使用 `ScalarStyle::SingleQuoted` 和默认元数据。
    ///
    /// # Arguments
    /// * `value` - 标量文本值。
    ///
    /// # Returns
    /// 返回一个 `CustomNode::Scalar` 变体，style 为 `SingleQuoted`。
    ///
    /// # Examples
    /// ```ignore
    /// use pyrs_yaml::ast::CustomNode;
    ///
    /// // YAML 输出: '-100'
    /// let node = CustomNode::quoted_scalar("-100");
    /// ```
    pub fn quoted_scalar(value: impl Into<String>) -> Self {
        CustomNode::Scalar {
            value: value.into(),
            style: ScalarStyle::SingleQuoted,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
            source_range: None,
        }
    }

    /// 创建一个无元数据的块风格映射节点。
    ///
    /// 键值对顺序由 `IndexMap` 保证，不会进行排序或重新排列。
    ///
    /// # Arguments
    /// * `pairs` - 保持插入顺序的键值对映射。
    ///
    /// # Returns
    /// 返回一个 `CustomNode::Mapping` 变体，`flow_style` 为 `false`。
    ///
    /// # Examples
    /// ```ignore
    /// use pyrs_yaml::ast::CustomNode;
    /// use indexmap::IndexMap;
    ///
    /// let mut pairs = IndexMap::new();
    /// pairs.insert(CustomNode::plain_scalar("key"), CustomNode::plain_scalar("value"));
    /// let node = CustomNode::plain_mapping(pairs);
    /// ```
    pub fn plain_mapping(pairs: IndexMap<CustomNode, CustomNode>) -> Self {
        CustomNode::Mapping {
            pairs,
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
            source_range: None,
        }
    }

    /// 创建一个无元数据的块风格序列节点。
    ///
    /// # Arguments
    /// * `items` - 序列中的子节点列表。
    ///
    /// # Returns
    /// 返回一个 `CustomNode::Sequence` 变体，`flow_style` 为 `false`。
    ///
    /// # Examples
    /// ```ignore
    /// use pyrs_yaml::ast::CustomNode;
    ///
    /// let items = vec![CustomNode::plain_scalar("a"), CustomNode::plain_scalar("b")];
    /// let node = CustomNode::plain_sequence(items);
    /// ```
    pub fn plain_sequence(items: Vec<CustomNode>) -> Self {
        CustomNode::Sequence {
            items,
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
            source_range: None,
        }
    }

    /// 创建一个无元数据的空值节点。
    ///
    /// # Returns
    /// 返回一个 `CustomNode::Null` 变体，所有元数据字段为 `None`。
    ///
    /// # Examples
    /// ```ignore
    /// use pyrs_yaml::ast::CustomNode;
    ///
    /// let node = CustomNode::plain_null();
    /// assert!(matches!(node, CustomNode::Null { .. }));
    /// ```
    pub fn plain_null() -> Self {
        CustomNode::Null {
            comment: None,
            anchor: None,
            tag: None,
            source_range: None,
        }
    }

    /// 获取节点附加的注释。
    ///
    /// # Returns
    /// 返回 `Some(&Comment)` 如果节点有注释，否则返回 `None`。
    /// `Alias` 变体始终返回 `None`。
    ///
    /// # Examples
    /// ```ignore
    /// use pyrs_yaml::ast::{CustomNode, Comment};
    ///
    /// let mut node = CustomNode::plain_scalar("value");
    /// assert_eq!(node.comment(), None);
    /// node.set_comment(Comment { text: "note".into(), standalone: false });
    /// assert!(node.comment().is_some());
    /// ```
    pub fn comment(&self) -> Option<&Comment> {
        match self {
            CustomNode::Scalar { comment, .. }
            | CustomNode::Mapping { comment, .. }
            | CustomNode::Sequence { comment, .. }
            | CustomNode::Null { comment, .. } => comment.as_ref(),
            CustomNode::Alias { .. } => None,
        }
    }

    /// 设置节点的注释。
    ///
    /// # Arguments
    /// * `new_comment` - 要附加的注释对象。
    ///
    /// 对 `Alias` 变体调用此方法是空操作。
    ///
    /// # Examples
    /// ```ignore
    /// use pyrs_yaml::ast::{CustomNode, Comment};
    ///
    /// let mut node = CustomNode::plain_scalar("hello");
    /// node.set_comment(Comment { text: "greeting".into(), standalone: false });
    /// assert_eq!(node.comment().unwrap().text, "greeting");
    /// ```
    pub fn set_comment(&mut self, new_comment: Comment) {
        match self {
            CustomNode::Scalar { comment, .. }
            | CustomNode::Mapping { comment, .. }
            | CustomNode::Sequence { comment, .. }
            | CustomNode::Null { comment, .. } => {
                *comment = Some(new_comment);
            }
            CustomNode::Alias { .. } => {}
        }
    }

    /// 获取节点在原始源文本中的字节区间。
    ///
    /// # Returns
    /// 返回 `Some(&Range<usize>)` 表示节点覆盖的 `[start, end)` 字节范围，
    /// 或 `None` 表示没有范围信息（例如编程式构造的节点或 `Alias` 变体）。
    pub fn source_range(&self) -> Option<&Range<usize>> {
        match self {
            CustomNode::Scalar { source_range, .. }
            | CustomNode::Mapping { source_range, .. }
            | CustomNode::Sequence { source_range, .. }
            | CustomNode::Null { source_range, .. } => source_range.as_ref(),
            CustomNode::Alias { .. } => None,
        }
    }

    /// 设置节点在原始源文本中的字节区间。
    ///
    /// 对 `Alias` 变体调用此方法是空操作（别名永远不携带源区间）。
    pub fn set_source_range(&mut self, r: Range<usize>) {
        match self {
            CustomNode::Scalar { source_range, .. }
            | CustomNode::Mapping { source_range, .. }
            | CustomNode::Sequence { source_range, .. }
            | CustomNode::Null { source_range, .. } => {
                *source_range = Some(r);
            }
            CustomNode::Alias { .. } => {}
        }
    }

    /// 获取节点的锚点名称。
    ///
    /// # Returns
    /// 返回 `Some(&str)` 锚点名称，或 `None` 表示无锚点。
    /// `Alias` 变体始终返回 `None`。
    ///
    /// # Examples
    /// ```ignore
    /// use pyrs_yaml::ast::CustomNode;
    ///
    /// let node = CustomNode::Scalar {
    ///     value: "42".into(),
    ///     style: Default::default(),
    ///     comment: None,
    ///     anchor: Some("my_anchor".into()),
    ///     tag: None,
    ///     chomping: Default::default(),
    ///     source_range: None,
    /// };
    /// assert_eq!(node.anchor(), Some("my_anchor"));
    /// ```
    pub fn anchor(&self) -> Option<&str> {
        match self {
            CustomNode::Scalar { anchor, .. }
            | CustomNode::Mapping { anchor, .. }
            | CustomNode::Sequence { anchor, .. }
            | CustomNode::Null { anchor, .. } => anchor.as_deref(),
            CustomNode::Alias { .. } => None,
        }
    }

    /// 获取节点的 YAML 标签。
    ///
    /// # Returns
    /// 返回 `Some(&Tag)` 如果节点有标签，否则返回 `None`。
    /// `Alias` 变体始终返回 `None`。
    ///
    /// # Examples
    /// ```ignore
    /// use pyrs_yaml::ast::{CustomNode, Tag};
    ///
    /// let node = CustomNode::Scalar {
    ///     value: "42".into(),
    ///     style: Default::default(),
    ///     comment: None,
    ///     anchor: None,
    ///     tag: Some(Tag::primary("int")),
    ///     chomping: Default::default(),
    ///     source_range: None,
    /// };
    /// assert_eq!(node.tag().unwrap().suffix, "int");
    /// ```
    pub fn tag(&self) -> Option<&Tag> {
        match self {
            CustomNode::Scalar { tag, .. }
            | CustomNode::Mapping { tag, .. }
            | CustomNode::Sequence { tag, .. }
            | CustomNode::Null { tag, .. } => tag.as_ref(),
            CustomNode::Alias { .. } => None,
        }
    }

    /// 设置节点的 YAML 标签。
    ///
    /// # Arguments
    /// * `new_tag` - 要附加的标签对象。
    ///
    /// 对 `Alias` 变体调用此方法是空操作。
    ///
    /// # Examples
    /// ```ignore
    /// use pyrs_yaml::ast::{CustomNode, Tag};
    ///
    /// let mut node = CustomNode::plain_scalar("42");
    /// node.set_tag(Tag::primary("int"));
    /// assert_eq!(node.tag().unwrap().suffix, "int");
    /// ```
    pub fn set_tag(&mut self, new_tag: Tag) {
        match self {
            CustomNode::Scalar { tag, .. }
            | CustomNode::Mapping { tag, .. }
            | CustomNode::Sequence { tag, .. }
            | CustomNode::Null { tag, .. } => {
                *tag = Some(new_tag);
            }
            CustomNode::Alias { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_creation() {
        let node = CustomNode::Scalar {
            value: "hello".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
            source_range: None,
        };
        assert_eq!(node.comment(), None);
        assert_eq!(node.anchor(), None);
        assert_eq!(node.tag(), None);
    }

    #[test]
    fn test_scalar_with_tag() {
        let node = CustomNode::Scalar {
            value: "42".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: Some(Tag::primary("int")),
            chomping: Chomping::Clip,
            source_range: None,
        };
        assert_eq!(node.tag().unwrap().suffix, "int");
    }

    #[test]
    fn test_scalar_with_comment() {
        let node = CustomNode::Scalar {
            value: "world".to_string(),
            style: ScalarStyle::DoubleQuoted,
            comment: Some(Comment {
                text: "a greeting".to_string(),
                standalone: false,
            }),
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
            source_range: None,
        };
        assert_eq!(node.comment().unwrap().text, "a greeting");
        assert!(!node.comment().unwrap().standalone);
    }

    #[test]
    fn test_mapping_preserves_order() {
        let key1 = CustomNode::Scalar {
            value: "b".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
            source_range: None,
        };
        let key2 = CustomNode::Scalar {
            value: "a".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
            source_range: None,
        };
        let val = CustomNode::Scalar {
            value: "1".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
            source_range: None,
        };

        let mut pairs = IndexMap::new();
        pairs.insert(key1.clone(), val.clone());
        pairs.insert(key2.clone(), val.clone());

        let mapping = CustomNode::Mapping {
            pairs,
            comment: None,
            anchor: None,
            tag: None,
            flow_style: false,
            source_range: None,
        };

        // Verify order is preserved (insertion order)
        if let CustomNode::Mapping { pairs, .. } = &mapping {
            let keys: Vec<&CustomNode> = pairs.keys().collect();
            assert_eq!(keys[0].clone(), key1);
            assert_eq!(keys[1].clone(), key2);
        }
    }

    #[test]
    fn test_tag_formatting() {
        assert_eq!(Tag::primary("str").to_string(), "!!str");
        assert_eq!(Tag::local("custom").to_string(), "!custom");
        assert_eq!(
            Tag {
                handle: "!".to_string(),
                suffix: "".to_string()
            }
            .to_string(),
            "!"
        );
    }
}
