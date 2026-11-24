//! Read-only XML -> YAML engine — the hub's XML spoke.
//!
//! One direction only: XML text in, shared [`pyrs_ast::ast::CustomNode`] out.
//! There is no writer, no `to_xml` symbol, and no edit substrate — YAML stays
//! the only editable representation. YAML -> XML is a later slice.
//!
//! ```
//! use pyrs_xml::from_xml;
//! let node = from_xml("<root id=\"7\"><name>Ada</name></root>").unwrap();
//! assert!(matches!(node, pyrs_ast::ast::CustomNode::Mapping { .. }));
//! ```
//!
//! # The key convention
//!
//! Structure that is not an element name carries a `+` prefix: `+@attr`,
//! `+content`, `+c_<n>` (comments), `+p_<n>` (processing instructions). `+` is
//! outside both the YAML plain-scalar indicators and the XML `Name`
//! production, so a reserved key can never collide with a real element name.
//! A text-only element with no attributes short-circuits to a plain scalar
//! instead of a `{+content: ...}` wrapper.
//!
//! # Infoset-preserving, not lexically lossless
//!
//! Element, attribute, comment, PI and character-data structure is kept. The
//! XML declaration, BOM, CDATA delimiters, entity spelling, self-closing vs
//! explicit end tags, attribute quote style and inter-attribute whitespace are
//! not: all six are normalised away by the mapping.
//!
//! # DTD and entities are refused structurally
//!
//! There is no DTD table, no external fetch and no I/O dependency. A
//! `<!DOCTYPE` or `<!ENTITY` is refused at the `<!` classification step,
//! *before* any content is read — a parse-then-filter design would expand a
//! billion-laughs bomb before the filter could see the declaration. Only the
//! five predefined references and numeric character references are decoded;
//! every other `&name;` is a refusal.
//!
//! # `no_std`
//!
//! The crate is `#![no_std]` and uses `alloc`, so it builds for a target with
//! no standard library at all (`cargo build --no-default-features --target
//! thumbv7em-none-eabi -p pyrs-xml`). The `std` feature is on by default for
//! the host test harness.

#![no_std]

#[macro_use]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod error;
pub mod mapping;
pub mod model;
pub mod scanner;

pub use error::XmlError;
pub use mapping::{XmlOptions, from_xml, from_xml_with_options};
// No `writer` module. No `to_xml` symbol. No `pub fn to_*` at all — this keeps
// SERIALIZER_RE (quality_matrix.py) silent for this crate.
