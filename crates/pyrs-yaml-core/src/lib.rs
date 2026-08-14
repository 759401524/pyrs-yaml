//! # pyrs-yaml-core
//!
//! Core YAML engine — pure Rust, no Python dependencies.
//! Provides AST, parser, serializer, splice, and editing modules.

pub mod ast;
pub mod editing;
pub mod error;
pub mod i18n;
pub mod parser;
pub mod serializer;
pub mod splice;

#[cfg(test)]
mod integration;

#[cfg(test)]
mod pbt;

rust_i18n::i18n!("src/i18n/locales");
