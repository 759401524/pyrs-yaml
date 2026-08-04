//! Core editing module — pure Rust, no PyO3 dependencies.
//!
//! This module provides the editing primitives for YAML document mutation.
//! It is independent of Python and can be used from any Rust context
//! (CLI, WASM, C FFI, gRPC, etc.).

pub mod dirty;
pub mod metadata;
pub mod navigate;
pub mod region;

pub use dirty::{DirtyKind, DirtyUnit};
pub use metadata::with_metadata_from;
pub use navigate::{
    key_eq, mapping_get_mut, mapping_key_index, navigate, navigate_mut, normalize_index,
    parse_path_segments, NavigateError, Segment,
};
pub use region::{
    compact_item_ancestor, eligible_path, extend_delete_over_comments, line_aligned, line_end,
    line_indent, line_start, nav_err, node_is_flow, path_nodes, precompute, regenerate_region_text,
    region_unit,
};
