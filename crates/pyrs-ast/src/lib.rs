//! # pyrs-ast
//!
//! Format-agnostic foundation for the pyrs-yaml engine family: the `CustomNode`
//! AST (with comment / anchor / tag fidelity slots) and the shared structured
//! error types. Depends only on `indexmap` + `thiserror` — no parser, i18n, or
//! regex dependencies — so it is the leaf every format crate builds on and the
//! base for future `no_std` support and independent ecosystem reuse.

pub mod ast;
pub mod error;
