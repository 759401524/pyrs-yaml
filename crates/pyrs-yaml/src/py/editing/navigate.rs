//! AST navigation for edit path resolution.
//!
//! Re-exported from the core `crate::editing` module for backward compatibility.

pub(crate) use crate::editing::{
    key_eq, mapping_get_mut, mapping_key_index, navigate, navigate_mut, normalize_index,
    parse_path_segments, NavigateError, Segment,
};
