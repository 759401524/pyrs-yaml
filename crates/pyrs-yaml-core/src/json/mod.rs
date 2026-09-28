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
//!
//! The default [`from_json`] is strict RFC 8259. Pass a
//! [`JsonParseOptions`] (or call [`from_jsonc`]) to accept JSONC-style
//! `//` and `/* ... */` comments — handy for parsing TypeScript
//! `tsconfig.json`, VS Code `settings.json`, and similar dialects.
//! Comments are stripped on read; they are not currently re-emitted on
//! write.

mod parser;
mod writer;

pub use parser::{
    DEFAULT_MAX_DEPTH, JsonParseOptions, from_json, from_json_with_max_depth,
    from_json_with_options, from_json5, from_jsonc,
};
pub use writer::{
    key_text, to_json_text, to_json_text_pretty, to_json5_text, to_json5_text_pretty,
    to_jsonc_text, to_jsonc_text_pretty,
};
