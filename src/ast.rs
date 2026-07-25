use indexmap::IndexMap;
use std::fmt;
use std::hash::{Hash, Hasher};

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
    },
    Mapping {
        pairs: IndexMap<CustomNode, CustomNode>,
        comment: Option<Comment>,
        anchor: Option<String>,
        tag: Option<Tag>,
    },
    Sequence {
        items: Vec<CustomNode>,
        comment: Option<Comment>,
        anchor: Option<String>,
        tag: Option<Tag>,
    },
    Null {
        comment: Option<Comment>,
        anchor: Option<String>,
        tag: Option<Tag>,
    },
    /// Alias reference (*alias)
    Alias {
        name: String,
    },
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
            } => {
                state.write_u8(0);
                value.hash(state);
                style.hash(state);
                comment.hash(state);
                anchor.hash(state);
                tag.hash(state);
                chomping.hash(state);
            }
            CustomNode::Mapping {
                pairs,
                comment,
                anchor,
                tag,
            } => {
                state.write_u8(1);
                for (k, v) in pairs {
                    k.hash(state);
                    v.hash(state);
                }
                comment.hash(state);
                anchor.hash(state);
                tag.hash(state);
            }
            CustomNode::Sequence {
                items,
                comment,
                anchor,
                tag,
            } => {
                state.write_u8(2);
                for item in items {
                    item.hash(state);
                }
                comment.hash(state);
                anchor.hash(state);
                tag.hash(state);
            }
            CustomNode::Null {
                comment,
                anchor,
                tag,
            } => {
                state.write_u8(3);
                comment.hash(state);
                anchor.hash(state);
                tag.hash(state);
            }
            CustomNode::Alias { name } => {
                state.write_u8(4);
                name.hash(state);
            }
        }
    }
}

impl CustomNode {
    /// Get the comment attached to this node
    pub fn comment(&self) -> Option<&Comment> {
        match self {
            CustomNode::Scalar { comment, .. }
            | CustomNode::Mapping { comment, .. }
            | CustomNode::Sequence { comment, .. }
            | CustomNode::Null { comment, .. } => comment.as_ref(),
            CustomNode::Alias { .. } => None,
        }
    }

    /// Set the comment on this node
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

    /// Get the anchor name if present
    pub fn anchor(&self) -> Option<&str> {
        match self {
            CustomNode::Scalar { anchor, .. }
            | CustomNode::Mapping { anchor, .. }
            | CustomNode::Sequence { anchor, .. }
            | CustomNode::Null { anchor, .. } => anchor.as_deref(),
            CustomNode::Alias { .. } => None,
        }
    }

    /// Get the tag if present
    pub fn tag(&self) -> Option<&Tag> {
        match self {
            CustomNode::Scalar { tag, .. }
            | CustomNode::Mapping { tag, .. }
            | CustomNode::Sequence { tag, .. }
            | CustomNode::Null { tag, .. } => tag.as_ref(),
            CustomNode::Alias { .. } => None,
        }
    }

    /// Set the tag on this node
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
        };
        let key2 = CustomNode::Scalar {
            value: "a".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
        };
        let val = CustomNode::Scalar {
            value: "1".to_string(),
            style: ScalarStyle::Plain,
            comment: None,
            anchor: None,
            tag: None,
            chomping: Chomping::Clip,
        };

        let mut pairs = IndexMap::new();
        pairs.insert(key1.clone(), val.clone());
        pairs.insert(key2.clone(), val.clone());

        let mapping = CustomNode::Mapping {
            pairs,
            comment: None,
            anchor: None,
            tag: None,
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
        assert_eq!(Tag { handle: "!".to_string(), suffix: "".to_string() }.to_string(), "!");
    }
}
