//! Edit orchestration re-export shell: the splice-plan engine moved down
//! to `pyrs_yaml_core::editing::plan` so the `pyq` CLI shares it; this
//! module keeps the bindings' historical `crate::py::editing::_` paths
//! alive and hosts the Python `SegmentPy` class.

// Re-export (public): benches and tests addressed the engine through this
// path before the core move; the names must keep resolving.
#[allow(unused_imports)]
pub use pyrs_yaml_core::editing::plan::*;

pub mod segment_py;
