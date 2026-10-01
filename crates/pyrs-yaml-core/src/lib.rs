//! # pyrs-yaml-core
//!
//! Core multi-format engine (YAML / TOML / JSON / JSONC / JSON5) — pure Rust,
//! no Python dependencies. The format-agnostic AST and shared error types live
//! in the `pyrs-ast` crate; the native JSON-family engine lives in `pyrs-json`
//! and the TOML engine in `pyrs-toml`. All are re-exported here so
//! `crate::ast` / `crate::error` / `crate::json` / `crate::toml` paths
//! resolve unchanged across the engine.

pub use pyrs_ast::{ast, error};

pub mod editing;
pub mod i18n;
pub use pyrs_json as json;
pub mod parser;
pub mod serializer;
pub mod splice;
pub use pyrs_toml as toml;

#[cfg(test)]
mod integration;

#[cfg(test)]
mod fmt_pbt;
#[cfg(test)]
mod pbt;

rust_i18n::i18n!("src/i18n/locales");
