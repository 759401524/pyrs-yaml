//! # pyrs-yaml-core
//!
//! Native YAML 1.2 engine (parser, round-trip serializer, comment-preserving
//! editing, splices) — pure Rust, no Python dependencies. Sibling format
//! engines are independent crates: `pyrs-json` (JSON/JSONC/JSON5) and
//! `pyrs-toml` (TOML 1.0/1.1). The format-agnostic AST and shared error
//! types live in `pyrs-ast` and are re-exported here so `crate::ast` /
//! `crate::error` paths resolve unchanged across the engine.

pub use pyrs_ast::{ast, error};

pub mod bench_inputs;
pub mod editing;
pub mod i18n;
pub mod parser;
pub mod serializer;
pub mod splice;

#[cfg(test)]
mod integration;

#[cfg(test)]
mod fmt_pbt;
#[cfg(test)]
mod pbt;

rust_i18n::i18n!("src/i18n/locales");
