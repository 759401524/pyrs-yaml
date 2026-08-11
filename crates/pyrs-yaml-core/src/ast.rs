use indexmap::IndexMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

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
    pub text: Arc<str>,
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

/// Metadata shared by all content-bearing node variants (not Alias).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NodeMeta {
    /// Comment attached to the node.
    pub comment: Option<Comment>,
    /// Anchor name for this node (e.g., "my_anchor").
    pub anchor: Option<String>,
    /// YAML tag (e.g., !!str, !custom).
    pub tag: Option<Tag>,
    /// Byte range of this node in the original source text.
    pub source_range: Option<Range<usize>>,
}

impl Hash for NodeMeta {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.comment.hash(state);
        self.anchor.hash(state);
        self.tag.hash(state);
    }
}

/// Custom AST node with full metadata for round-trip support
#[derive(Debug, Clone, Eq)]
pub enum CustomNode {
    Scalar {
        value: Arc<str>,
        style: ScalarStyle,
        /// Chomping indicator for block scalars (|+, |-, >+, >-)
        chomping: Chomping,
        /// Shared metadata (comment, anchor, tag, source_range)
        meta: NodeMeta,
    },
    Mapping {
        pairs: IndexMap<CustomNode, CustomNode>,
        /// Whether this mapping uses flow style ({key: value}) vs block style
        flow_style: bool,
        /// Shared metadata (comment, anchor, tag, source_range)
        meta: NodeMeta,
    },
    Sequence {
        items: Vec<CustomNode>,
        /// Whether this sequence uses flow style (\[item\]) vs block style
        flow_style: bool,
        /// Shared metadata (comment, anchor, tag, source_range)
        meta: NodeMeta,
    },
    Null {
        /// Shared metadata (comment, anchor, tag, source_range)
        meta: NodeMeta,
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
                chomping,
                meta,
            } => {
                state.write_u8(0);
                value.hash(state);
                style.hash(state);
                meta.hash(state);
                chomping.hash(state);
            }
            CustomNode::Mapping {
                pairs,
                flow_style,
                meta,
            } => {
                state.write_u8(1);
                for (k, v) in pairs {
                    k.hash(state);
                    v.hash(state);
                }
                meta.hash(state);
                flow_style.hash(state);
            }
            CustomNode::Sequence {
                items,
                flow_style,
                meta,
            } => {
                state.write_u8(2);
                for item in items {
                    item.hash(state);
                }
                meta.hash(state);
                flow_style.hash(state);
            }
            CustomNode::Null { meta } => {
                state.write_u8(3);
                meta.hash(state);
            }
            CustomNode::Alias { name } => {
                state.write_u8(4);
                name.hash(state);
            }
        }
    }
}

/// Depth-first pre-order traversal yielding paths as `&[&CustomNode]`.
///
/// # Example
///
/// ```rust
/// use pyrs_yaml_core::ast::CustomNode;
/// use pyrs_yaml_core::ast::walk;
///
/// let node = CustomNode::plain_scalar("hello");
/// let paths = walk(&node);
/// assert_eq!(paths.len(), 1);
/// ```
pub fn walk(node: &CustomNode) -> Vec<Vec<&CustomNode>> {
    traverse(node, |_| true)
}

/// Depth-first pre-order traversal yielding only scalar/null paths.
///
/// # Example
///
/// ```rust
/// use pyrs_yaml_core::ast::CustomNode;
/// use pyrs_yaml_core::ast::scalars;
///
/// let node = CustomNode::plain_scalar("hello");
/// let paths = scalars(&node);
/// assert_eq!(paths.len(), 1);
/// ```
pub fn scalars(node: &CustomNode) -> Vec<Vec<&CustomNode>> {
    traverse(node, |n| {
        matches!(n, CustomNode::Scalar { .. } | CustomNode::Null { .. })
    })
}

/// Depth-first pre-order traversal.
///
/// `filter` decides whether a node is included in the output. Container nodes
/// are always recursed into regardless of `filter`; only the emitted path is
/// gated by it.
fn traverse<F>(node: &CustomNode, mut filter: F) -> Vec<Vec<&CustomNode>>
where
    F: FnMut(&CustomNode) -> bool,
{
    let mut paths = Vec::new();
    fn collect<'a, F>(
        node: &'a CustomNode,
        path: &mut Vec<&'a CustomNode>,
        out: &mut Vec<Vec<&'a CustomNode>>,
        filter: &mut F,
    ) where
        F: FnMut(&CustomNode) -> bool,
    {
        if filter(node) {
            path.push(node);
            out.push(path.clone());
            path.pop();
        }
        match node {
            CustomNode::Mapping { pairs, .. } => {
                for (_, v) in pairs.iter() {
                    collect(v, path, out, filter);
                }
            }
            CustomNode::Sequence { items, .. } => {
                for item in items.iter() {
                    collect(item, path, out, filter);
                }
            }
            _ => {}
        }
    }
    collect(node, &mut Vec::new(), &mut paths, &mut filter);
    paths
}

impl PartialEq for CustomNode {
    /// Equality is structural: `source_range` (byte provenance) is excluded so that
    /// programmatically-built nodes (`source_range: None`) match parsed nodes (`Some(..)`),
    /// and dirty-detection snapshots that differ only in ranges still compare equal.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                CustomNode::Scalar {
                    value: a,
                    style: b,
                    chomping: c,
                    meta: d,
                    ..
                },
                CustomNode::Scalar {
                    value: a2,
                    style: b2,
                    chomping: c2,
                    meta: d2,
                    ..
                },
            ) => {
                a == a2
                    && b == b2
                    && c == c2
                    && d.comment == d2.comment
                    && d.anchor == d2.anchor
                    && d.tag == d2.tag
            }
            (
                CustomNode::Mapping {
                    pairs: a,
                    flow_style: b,
                    meta: c,
                    ..
                },
                CustomNode::Mapping {
                    pairs: a2,
                    flow_style: b2,
                    meta: c2,
                    ..
                },
            ) => {
                a == a2
                    && b == b2
                    && c.comment == c2.comment
                    && c.anchor == c2.anchor
                    && c.tag == c2.tag
            }
            (
                CustomNode::Sequence {
                    items: a,
                    flow_style: b,
                    meta: c,
                    ..
                },
                CustomNode::Sequence {
                    items: a2,
                    flow_style: b2,
                    meta: c2,
                    ..
                },
            ) => {
                a == a2
                    && b == b2
                    && c.comment == c2.comment
                    && c.anchor == c2.anchor
                    && c.tag == c2.tag
            }
            (CustomNode::Null { meta: a, .. }, CustomNode::Null { meta: a2, .. }) => {
                a.comment == a2.comment && a.anchor == a2.anchor && a.tag == a2.tag
            }
            (CustomNode::Alias { name: a }, CustomNode::Alias { name: a2 }) => a == a2,
            _ => false,
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
    /// ```rust
    /// use pyrs_yaml_core::ast::CustomNode;
    ///
    /// let node = CustomNode::plain_scalar("hello");
    /// assert_eq!(node.comment(), None);
    /// ```
    pub fn plain_scalar(value: impl Into<Arc<str>>) -> Self {
        CustomNode::Scalar {
            value: value.into(),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: NodeMeta::default(),
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
    /// ```rust
    /// use pyrs_yaml_core::ast::CustomNode;
    ///
    /// // YAML 输出: '-100'
    /// let node = CustomNode::quoted_scalar("-100");
    /// ```
    pub fn quoted_scalar(value: impl Into<Arc<str>>) -> Self {
        CustomNode::Scalar {
            value: value.into(),
            style: ScalarStyle::SingleQuoted,
            chomping: Chomping::Clip,
            meta: NodeMeta::default(),
        }
    }

    /// 创建一个双引号风格的标量节点（用于表示会隐式解析为非字符串的
    /// 字符串值，如 `"true"`、`"42"`、`"null"`、`"~"`、空串及控制字符，
    /// 裸输出会丢失类型或产生非法 YAML）。
    ///
    /// 使用 `ScalarStyle::DoubleQuoted` 和默认元数据。
    ///
    /// # Arguments
    /// * `value` - 标量文本值。
    ///
    /// # Returns
    /// 返回一个 `CustomNode::Scalar` 变体，style 为 `DoubleQuoted`。
    ///
    /// # Examples
    /// ```rust
    /// use pyrs_yaml_core::ast::CustomNode;
    ///
    /// // YAML 输出: "true"
    /// let node = CustomNode::double_quoted_scalar("true");
    /// ```
    pub fn double_quoted_scalar(value: impl Into<Arc<str>>) -> Self {
        CustomNode::Scalar {
            value: value.into(),
            style: ScalarStyle::DoubleQuoted,
            chomping: Chomping::Clip,
            meta: NodeMeta::default(),
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
    /// ```rust
    /// use pyrs_yaml_core::ast::CustomNode;
    /// use indexmap::IndexMap;
    ///
    /// let mut pairs = IndexMap::new();
    /// pairs.insert(CustomNode::plain_scalar("key"), CustomNode::plain_scalar("value"));
    /// let node = CustomNode::plain_mapping(pairs);
    /// ```
    pub fn plain_mapping(pairs: IndexMap<CustomNode, CustomNode>) -> Self {
        CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: NodeMeta::default(),
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
    /// ```rust
    /// use pyrs_yaml_core::ast::CustomNode;
    ///
    /// let items = vec![CustomNode::plain_scalar("a"), CustomNode::plain_scalar("b")];
    /// let node = CustomNode::plain_sequence(items);
    /// ```
    pub fn plain_sequence(items: Vec<CustomNode>) -> Self {
        CustomNode::Sequence {
            items,
            flow_style: false,
            meta: NodeMeta::default(),
        }
    }

    /// 创建一个无元数据的空值节点。
    ///
    /// # Returns
    /// 返回一个 `CustomNode::Null` 变体，所有元数据字段为 `None`。
    ///
    /// # Examples
    /// ```rust
    /// use pyrs_yaml_core::ast::CustomNode;
    ///
    /// let node = CustomNode::plain_null();
    /// assert!(matches!(node, CustomNode::Null { .. }));
    /// ```
    pub fn plain_null() -> Self {
        CustomNode::Null {
            meta: NodeMeta::default(),
        }
    }

    /// 获取节点附加的注释。
    ///
    /// # Returns
    /// 返回 `Some(&Comment)` 如果节点有注释，否则返回 `None`。
    /// `Alias` 变体始终返回 `None`。
    ///
    /// # Examples
    /// ```rust
    /// use pyrs_yaml_core::ast::{CustomNode, Comment};
    ///
    /// let mut node = CustomNode::plain_scalar("value");
    /// assert_eq!(node.comment(), None);
    /// node.set_comment(Comment { text: "note".into(), standalone: false });
    /// assert!(node.comment().is_some());
    /// ```
    pub fn comment(&self) -> Option<&Comment> {
        self.meta().and_then(|m| m.comment.as_ref())
    }

    /// 设置节点的注释。
    ///
    /// # Arguments
    /// * `new_comment` - 要附加的注释对象。
    ///
    /// 对 `Alias` 变体调用此方法是空操作。
    ///
    /// # Examples
    /// ```rust
    /// use pyrs_yaml_core::ast::{CustomNode, Comment};
    ///
    /// let mut node = CustomNode::plain_scalar("hello");
    /// node.set_comment(Comment { text: "greeting".into(), standalone: false });
    /// assert_eq!(node.comment().unwrap().text.as_ref(), "greeting");
    /// ```
    pub fn set_comment(&mut self, new_comment: Comment) {
        if let Some(meta) = self.meta_mut() {
            meta.comment = Some(new_comment);
        }
    }

    /// 获取节点在原始源文本中的字节区间。
    ///
    /// # Returns
    /// 返回 `Some(&Range<usize>)` 表示节点覆盖的 `[start, end)` 字节范围，
    /// 或 `None` 表示没有范围信息（例如编程式构造的节点或 `Alias` 变体）。
    pub fn source_range(&self) -> Option<&Range<usize>> {
        self.meta().and_then(|m| m.source_range.as_ref())
    }

    /// 设置节点在原始源文本中的字节区间。
    ///
    /// 对 `Alias` 变体调用此方法是空操作（别名永远不携带源区间）。
    pub fn set_source_range(&mut self, r: Range<usize>) {
        if let Some(meta) = self.meta_mut() {
            meta.source_range = Some(r);
        }
    }

    /// 获取节点的锚点名称。
    ///
    /// # Returns
    /// 返回 `Some(&str)` 锚点名称，或 `None` 表示无锚点。
    /// `Alias` 变体始终返回 `None`。
    ///
    /// # Examples
    /// ```rust
    /// use pyrs_yaml_core::ast::{CustomNode, ScalarStyle, Chomping};
    ///
    /// let node = CustomNode::Scalar {
    ///     value: "42".into(),
    ///     style: ScalarStyle::Plain,
    ///     chomping: Chomping::Clip,
    ///     meta: Default::default(),
    /// };
    /// assert_eq!(node.anchor(), None);
    /// ```
    pub fn anchor(&self) -> Option<&str> {
        self.meta().and_then(|m| m.anchor.as_deref())
    }

    /// 获取节点的 YAML 标签。
    ///
    /// # Returns
    /// 返回 `Some(&Tag)` 如果节点有标签，否则返回 `None`。
    /// `Alias` 变体始终返回 `None`。
    ///
    /// # Examples
    /// ```rust
    /// use pyrs_yaml_core::ast::{CustomNode, Tag, ScalarStyle, Chomping};
    ///
    /// let node = CustomNode::Scalar {
    ///     value: "42".into(),
    ///     style: ScalarStyle::Plain,
    ///     chomping: Chomping::Clip,
    ///     meta: Default::default(),
    /// };
    /// assert_eq!(node.tag(), None);
    /// ```
    pub fn tag(&self) -> Option<&Tag> {
        self.meta().and_then(|m| m.tag.as_ref())
    }

    /// 设置节点的 YAML 标签。
    ///
    /// # Arguments
    /// * `new_tag` - 要附加的标签对象。
    ///
    /// 对 `Alias` 变体调用此方法是空操作。
    ///
    /// # Examples
    /// ```rust
    /// use pyrs_yaml_core::ast::{CustomNode, Tag};
    ///
    /// let mut node = CustomNode::plain_scalar("42");
    /// node.set_tag(Tag::primary("int"));
    /// assert_eq!(node.tag().unwrap().suffix, "int");
    /// ```
    pub fn set_tag(&mut self, new_tag: Tag) {
        if let Some(meta) = self.meta_mut() {
            meta.tag = Some(new_tag);
        }
    }

    /// Access the shared metadata of a content-bearing variant.
    fn meta(&self) -> Option<&NodeMeta> {
        match self {
            CustomNode::Scalar { meta, .. }
            | CustomNode::Mapping { meta, .. }
            | CustomNode::Sequence { meta, .. }
            | CustomNode::Null { meta, .. } => Some(meta),
            CustomNode::Alias { .. } => None,
        }
    }

    /// Mutably access the shared metadata of a content-bearing variant.
    fn meta_mut(&mut self) -> Option<&mut NodeMeta> {
        match self {
            CustomNode::Scalar { meta, .. }
            | CustomNode::Mapping { meta, .. }
            | CustomNode::Sequence { meta, .. }
            | CustomNode::Null { meta, .. } => Some(meta),
            CustomNode::Alias { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_creation() {
        let node = CustomNode::Scalar {
            value: Arc::from("hello"),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: Default::default(),
        };
        assert_eq!(node.comment(), None);
        assert_eq!(node.anchor(), None);
        assert_eq!(node.tag(), None);
    }

    #[test]
    fn test_scalar_with_tag() {
        let node = CustomNode::Scalar {
            value: Arc::from("42"),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: NodeMeta {
                tag: Some(Tag::primary("int")),
                ..Default::default()
            },
        };
        assert_eq!(node.tag().unwrap().suffix, "int");
    }

    #[test]
    fn test_scalar_with_comment() {
        let node = CustomNode::Scalar {
            value: Arc::from("world"),
            style: ScalarStyle::DoubleQuoted,
            chomping: Chomping::Clip,
            meta: NodeMeta {
                comment: Some(Comment {
                    text: Arc::from("a greeting"),
                    standalone: false,
                }),
                ..Default::default()
            },
        };
        assert_eq!(node.comment().unwrap().text.as_ref(), "a greeting");
        assert!(!node.comment().unwrap().standalone);
    }

    #[test]
    fn test_mapping_preserves_order() {
        let key1 = CustomNode::Scalar {
            value: Arc::from("b"),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: Default::default(),
        };
        let key2 = CustomNode::Scalar {
            value: Arc::from("a"),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: Default::default(),
        };
        let val = CustomNode::Scalar {
            value: Arc::from("1"),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: Default::default(),
        };

        let mut pairs = IndexMap::new();
        pairs.insert(key1.clone(), val.clone());
        pairs.insert(key2.clone(), val.clone());

        let mapping = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: Default::default(),
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
