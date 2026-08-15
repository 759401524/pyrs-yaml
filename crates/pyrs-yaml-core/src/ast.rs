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

    /// Create a verbatim tag (!<suffix>)
    pub fn verbatim(suffix: &str) -> Self {
        Self {
            handle: String::new(),
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
        } else if self.handle.is_empty() && self.suffix != "!" {
            write!(f, "!<{}>", self.suffix)
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

    /// Set the node's anchor name. No-op on `Alias`.
    ///
    /// # Examples
    /// ```rust
    /// use pyrs_yaml_core::ast::CustomNode;
    ///
    /// let mut node = CustomNode::plain_scalar("val");
    /// node.set_anchor("myanchor");
    /// assert_eq!(node.anchor(), Some("myanchor"));
    /// ```
    pub fn set_anchor(&mut self, name: impl Into<String>) {
        if let Some(meta) = self.meta_mut() {
            meta.anchor = Some(name.into());
        }
    }

    /// Remove the node's anchor. No-op on `Alias`.
    pub fn remove_anchor(&mut self) {
        if let Some(meta) = self.meta_mut() {
            meta.anchor = None;
        }
    }

    /// Remove the node's comment. No-op on `Alias`.
    pub fn remove_comment(&mut self) {
        if let Some(meta) = self.meta_mut() {
            meta.comment = None;
        }
    }

    /// Remove the node's tag. No-op on `Alias`.
    pub fn remove_tag(&mut self) {
        if let Some(meta) = self.meta_mut() {
            meta.tag = None;
        }
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
pub(crate) mod proptest_strategies {
    use super::*;
    use proptest::prelude::*;
    use proptest::strategy::BoxedStrategy;

    pub fn arb_scalar_style() -> impl Strategy<Value = ScalarStyle> {
        prop_oneof![
            Just(ScalarStyle::Plain),
            Just(ScalarStyle::SingleQuoted),
            Just(ScalarStyle::DoubleQuoted),
            Just(ScalarStyle::Literal),
            Just(ScalarStyle::Folded),
        ]
    }

    pub fn arb_chomping() -> impl Strategy<Value = Chomping> {
        prop_oneof![
            Just(Chomping::Strip),
            Just(Chomping::Clip),
            Just(Chomping::Keep),
        ]
    }

    pub fn arb_tag() -> impl Strategy<Value = Tag> {
        (prop_oneof![Just("!"), Just("!!")], "[a-zA-Z][a-zA-Z0-9_-]*").prop_map(
            |(handle, suffix)| Tag {
                handle: handle.to_string(),
                suffix,
            },
        )
    }

    pub fn arb_comment() -> impl Strategy<Value = Comment> {
        (
            proptest::string::string_regex("[ -~]+").expect("regex"),
            any::<bool>(),
        )
            .prop_map(|(text, standalone)| Comment {
                text: Arc::from(text),
                standalone,
            })
    }

    pub fn arb_node_meta() -> impl Strategy<Value = NodeMeta> {
        (
            proptest::option::of(arb_comment()),
            proptest::option::of("[a-zA-Z_][a-zA-Z0-9_-]*"),
            proptest::option::of(arb_tag()),
        )
            .prop_map(|(comment, anchor, tag)| NodeMeta {
                comment,
                anchor,
                tag,
                source_range: None,
            })
    }

    fn arb_scalar() -> impl Strategy<Value = CustomNode> {
        let meta = arb_node_meta();
        let style = arb_scalar_style();
        let value = proptest::string::string_regex("[ -~]+").expect("regex");
        (style, value, meta)
            .prop_flat_map(|(style, value, meta)| {
                let chomp = match style {
                    ScalarStyle::Literal | ScalarStyle::Folded => arb_chomping().boxed(),
                    _ => Just(Chomping::Clip).boxed(),
                };
                (Just(style), Just(value), Just(meta), chomp)
            })
            .prop_map(|(style, value, meta, chomping)| CustomNode::Scalar {
                value: Arc::from(value),
                style,
                chomping,
                meta,
            })
    }

    fn arb_mapping(inner: BoxedStrategy<CustomNode>) -> impl Strategy<Value = CustomNode> {
        (
            prop::collection::vec((inner.clone(), inner), 0..8),
            any::<bool>(),
            arb_node_meta(),
        )
            .prop_map(|(pairs, flow_style, meta)| {
                let mut map = IndexMap::new();
                for (k, v) in pairs {
                    map.insert(k, v);
                }
                CustomNode::Mapping {
                    pairs: map,
                    flow_style,
                    meta,
                }
            })
    }

    fn arb_sequence(inner: BoxedStrategy<CustomNode>) -> impl Strategy<Value = CustomNode> {
        (
            prop::collection::vec(inner, 0..8),
            any::<bool>(),
            arb_node_meta(),
        )
            .prop_map(|(items, flow_style, meta)| CustomNode::Sequence {
                items,
                flow_style,
                meta,
            })
    }

    fn arb_null() -> impl Strategy<Value = CustomNode> {
        arb_node_meta().prop_map(|meta| CustomNode::Null { meta })
    }

    #[allow(dead_code)]
    fn arb_alias() -> impl Strategy<Value = CustomNode> {
        "[a-zA-Z_][a-zA-Z0-9_-]*".prop_map(|name| CustomNode::Alias { name })
    }

    /// Full strategy including Alias (for non-round-trip tests).
    #[allow(dead_code)]
    pub fn arb_custom_node_full() -> impl Strategy<Value = CustomNode> {
        let leaf = prop_oneof![
            arb_scalar().boxed(),
            arb_null().boxed(),
            arb_alias().boxed()
        ];
        leaf.prop_recursive(6, 128, 8, |inner| {
            prop_oneof![
                arb_mapping(inner.clone()).boxed(),
                arb_sequence(inner).boxed()
            ]
            .boxed()
        })
    }

    /// Strategy excluding Alias (for round-trip tests).
    pub fn arb_custom_node() -> impl Strategy<Value = CustomNode> {
        let leaf = prop_oneof![arb_scalar().boxed(), arb_null().boxed()];
        leaf.prop_recursive(6, 128, 8, |inner| {
            prop_oneof![
                arb_mapping(inner.clone()).boxed(),
                arb_sequence(inner).boxed()
            ]
            .boxed()
        })
    }

    /// Compare two CustomNode trees ignoring source_range and
    /// chomping-on-non-block-scalars.
    pub fn nodes_equal_ignore_meta(a: &CustomNode, b: &CustomNode) -> bool {
        use CustomNode::*;
        match (a, b) {
            (
                Scalar {
                    value: va,
                    style: sa,
                    chomping: ca,
                    meta: ma,
                },
                Scalar {
                    value: vb,
                    style: sb,
                    chomping: cb,
                    meta: mb,
                },
            ) => {
                let style_ok = sa == sb
                    || matches!(
                        (sa, sb),
                        (ScalarStyle::Literal, ScalarStyle::Plain)
                            | (ScalarStyle::Folded, ScalarStyle::Plain)
                            | (ScalarStyle::Plain, ScalarStyle::SingleQuoted)
                            | (ScalarStyle::Plain, ScalarStyle::DoubleQuoted)
                    );
                va == vb
                    && style_ok
                    && (ca == cb
                        || (!matches!(sa, ScalarStyle::Literal | ScalarStyle::Folded)
                            && *ca == Chomping::Clip))
                    && meta_equal_ignore_source(ma, mb)
            }
            (
                Mapping {
                    pairs: pa,
                    flow_style: fa,
                    meta: ma,
                },
                Mapping {
                    pairs: pb,
                    flow_style: fb,
                    meta: mb,
                },
            ) => {
                let style_ok = fa == fb || (pa.is_empty() && pb.is_empty());
                let tag_ok = meta_equal_ignore_source(ma, mb)
                    || (pa.is_empty() && pb.is_empty() && meta_allow_loss_on_empty(ma, mb));
                style_ok
                    && pa.len() == pb.len()
                    && pa.iter().zip(pb.iter()).all(|((ka, va), (kb, vb))| {
                        nodes_equal_ignore_meta(ka, kb) && nodes_equal_ignore_meta(va, vb)
                    })
                    && tag_ok
            }
            (
                Sequence {
                    items: ia,
                    flow_style: fa,
                    meta: ma,
                },
                Sequence {
                    items: ib,
                    flow_style: fb,
                    meta: mb,
                },
            ) => {
                let style_ok = fa == fb || (ia.is_empty() && ib.is_empty());
                let tag_ok = meta_equal_ignore_source(ma, mb)
                    || (ia.is_empty() && ib.is_empty() && meta_allow_loss_on_empty(ma, mb));
                style_ok
                    && ia.len() == ib.len()
                    && ia
                        .iter()
                        .zip(ib.iter())
                        .all(|(a, b)| nodes_equal_ignore_meta(a, b))
                    && tag_ok
            }
            (Null { meta: ma }, Null { meta: mb }) => meta_equal_ignore_source(ma, mb),
            (Alias { name: na }, Alias { name: nb }) => na == nb,
            _ => false,
        }
    }

    fn meta_equal_ignore_source(a: &NodeMeta, b: &NodeMeta) -> bool {
        a.comment == b.comment && a.anchor == b.anchor && a.tag == b.tag
    }

    /// Allow tag/anchor loss on empty containers (serializer normalizes them
    /// to `{}`/`[]` which lose metadata).
    fn meta_allow_loss_on_empty(a: &NodeMeta, b: &NodeMeta) -> bool {
        a.comment == b.comment
            && (a.anchor == b.anchor || b.anchor.is_none())
            && (a.tag == b.tag || b.tag.is_none())
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
        assert_eq!(
            Tag::verbatim("tag:yaml.org,2002:str").to_string(),
            "!<tag:yaml.org,2002:str>"
        );
        assert_eq!(
            Tag {
                handle: String::new(),
                suffix: "!".to_string()
            }
            .to_string(),
            "!"
        );
    }
}
