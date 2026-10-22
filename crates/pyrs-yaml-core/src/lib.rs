//! # pyrs-yaml-core
//!
//! Native YAML 1.2 engine (parser, round-trip serializer, comment-preserving
//! editing, splices) — pure Rust, no Python dependencies. Sibling format
//! engines are independent crates: `pyrs-json` (JSON/JSONC/JSON5) and
//! `pyrs-toml` (TOML 1.0/1.1). The format-agnostic AST and shared error
//! types live in `pyrs-ast` and are re-exported here so `crate::ast` /
//! `crate::error` paths resolve unchanged across the engine.
//!
//! `#![no_std]`: the parser, serializer, edit and splice layers run on
//! `core` + `alloc` alone, and `granit-parser` is built without std underneath
//! them. The default `std` feature adds surfaces that need the OS rather than
//! being required by the engine: [`i18n`] (rust-i18n owns a process-global
//! locale) and the runtime schema surface (`parser::yaml::registry`, whose
//! global mutable registry is a `RwLock`, plus
//! `parser::yaml::schema_language`, the regex-backed user-schema language). A
//! `no_std` caller selects a schema by constructing the
//! `pyrs_schema::types::Schema` variant directly.

#![no_std]

// `format!`/`vec!` live in `alloc`'s macro exports, which a `no_std` crate
// only sees through `#[macro_use]` — the std prelude that carries them for a
// normal crate is deliberately absent here.
#[macro_use]
extern crate alloc;
// `#![no_std]` drops `std` from the extern prelude, so the `std` feature and
// the host test harness (which builds on a std host even with the feature off)
// need it named explicitly.
#[cfg(any(feature = "std", test))]
extern crate std;
// The `i18n!` expansion below names `Box`/`Vec` at its invocation site, which
// a `no_std` crate no longer has in a prelude.
#[cfg(feature = "std")]
use alloc::{boxed::Box, vec::Vec};

pub use pyrs_ast::{ast, error};

pub mod bench_inputs;
pub mod editing;
#[cfg(feature = "std")]
pub mod file;
#[cfg(feature = "std")]
pub mod i18n;
pub mod parser;
pub mod serializer;
pub mod splice;
pub mod tagged;

#[cfg(test)]
mod integration;

#[cfg(test)]
mod fmt_pbt;
#[cfg(test)]
mod pbt;

#[cfg(feature = "std")]
rust_i18n::i18n!("src/i18n/locales");
