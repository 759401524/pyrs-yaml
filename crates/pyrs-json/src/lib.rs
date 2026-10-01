//! Native JSON engine (`granit-parser` house style, RFC 8259 grammar).
//!
//! Replaces the previous `serde_json`-backed paths with a byte-level
//! parser producing the shared `CustomNode` AST (with exact error
//! positions and verbatim number spelling) and a serializer projecting
//! AST nodes onto JSON text with stable rejection reasons.
//!
//! ```
//! use pyrs_json::{from_json, to_json_text};
//! let node = from_json(r#"{"a": [1, 2.5, true]}"#).unwrap();
//! assert_eq!(to_json_text(&node).unwrap(), r#"{"a":[1,2.5,true]}"#);
//! ```
//!
//! The default [`from_json`] is strict RFC 8259. Pass a
//! [`JsonParseOptions`] (or call [`from_jsonc`]) to accept JSONC-style
//! `//` and `/* ... */` comments — handy for parsing TypeScript
//! `tsconfig.json`, VS Code `settings.json`, and similar dialects.
//! Comments are carried on the AST (leading notes ride the
//! `leading_comment` slot, inline ones the `comment` slot) and are
//! re-emitted by [`to_jsonc_text`] / [`to_json5_text`] (#122); the
//! plain [`to_json_text`] writer drops them, as strict JSON has no
//! comment syntax.

#![no_std]

// `format!`/`vec!` live in `alloc`'s macro exports, which a `no_std` crate
// only sees through `#[macro_use]` — the std prelude that carries them for
// a normal crate is deliberately absent here.
#[macro_use]
extern crate alloc;
// `#![no_std]` drops `std` from the extern prelude, so the std-only
// `RandomState` node-map hasher (and the host test harness) need it named
// explicitly when the `std` feature is on.
#[cfg(feature = "std")]
extern crate std;

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
