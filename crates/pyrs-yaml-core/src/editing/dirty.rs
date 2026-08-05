//! Edit operation types for splice-based document modification.
//!
//! This module defines the core types used by the editing primitives and
//! splice assembly layer. It is pure Rust with no Python/PyO3 dependencies.

use std::ops::Range;

/// Describes the kind of edit operation to apply.
///
/// ```
/// use pyrs_yaml_core::editing::DirtyKind;
/// let insert = DirtyKind::Insert { at: 5, text: "hello".into() };
/// let region = DirtyKind::Region { range: 0..5, indent: 2, text: "abc".into() };
/// let delete = DirtyKind::Delete { range: 3..7 };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum DirtyKind {
    /// Replace a range of original text with new text.
    Region {
        range: Range<usize>,
        indent: usize,
        text: String,
    },
    /// Insert text at a byte offset.
    Insert { at: usize, text: String },
    /// Delete a range of original text.
    Delete { range: Range<usize> },
}

/// A single edit unit with eligibility information.
///
/// ```
/// use pyrs_yaml_core::editing::{DirtyKind, DirtyUnit};
/// let unit = DirtyUnit { kind: DirtyKind::Insert { at: 0, text: "".into() }, eligible: true };
/// assert!(unit.eligible);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DirtyUnit {
    pub kind: DirtyKind,
    pub eligible: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_kind_insert() {
        let kind = DirtyKind::Insert {
            at: 5,
            text: "hello".into(),
        };
        match kind {
            DirtyKind::Insert { at, text } => {
                assert_eq!(at, 5);
                assert_eq!(text, "hello");
            }
            _ => panic!("expected Insert"),
        }
    }

    #[test]
    fn test_dirty_kind_delete() {
        let kind = DirtyKind::Delete { range: 3..7 };
        match kind {
            DirtyKind::Delete { range } => {
                assert_eq!(range, 3..7);
            }
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn test_dirty_kind_region() {
        let kind = DirtyKind::Region {
            range: 0..5,
            indent: 2,
            text: "abc".into(),
        };
        match kind {
            DirtyKind::Region {
                range,
                indent,
                text,
            } => {
                assert_eq!(range, 0..5);
                assert_eq!(indent, 2);
                assert_eq!(text, "abc");
            }
            _ => panic!("expected Region"),
        }
    }

    #[test]
    fn test_dirty_unit_eligible() {
        let unit = DirtyUnit {
            kind: DirtyKind::Insert {
                at: 0,
                text: "".into(),
            },
            eligible: true,
        };
        assert!(unit.eligible);
    }

    #[test]
    fn test_dirty_unit_ineligible() {
        let unit = DirtyUnit {
            kind: DirtyKind::Delete { range: 0..1 },
            eligible: false,
        };
        assert!(!unit.eligible);
    }
}
