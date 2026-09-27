//! Native JSON engine (`granit-parser` house style, RFC 8259 grammar).
//!
//! Replaces the previous `serde_json`-backed paths with a byte-level
//! parser producing the shared `CustomNode` AST (with exact error
//! positions and verbatim number spelling) and a serializer projecting
//! AST nodes onto JSON text with stable rejection reasons.
//!
//! ```
//! use pyrs_yaml_core::json::{from_json, to_json_text};
//! let node = from_json(r#"{"a": [1, 2.5, true]}"#).unwrap();
//! assert_eq!(to_json_text(&node).unwrap(), r#"{"a":[1,2.5,true]}"#);
//! ```

mod parser;
mod writer;

pub use parser::{DEFAULT_MAX_DEPTH, from_json, from_json_with_max_depth};
pub use writer::{key_text, to_json_text, to_json_text_pretty};
