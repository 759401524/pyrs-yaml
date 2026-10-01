//! # pyrs-ast
//!
//! Format-agnostic foundation for the pyrs-yaml engine family: the `CustomNode`
//! AST (with comment / anchor / tag fidelity slots) and the shared structured
//! error types. Depends only on `indexmap` + `thiserror` — no parser, i18n, or
//! regex dependencies — so it is the leaf every format crate builds on and the
//! base for `no_std` support and independent ecosystem reuse.
//!
//! `#![no_std]` by default; `alloc` covers the owned node fields (`String`
//! tags/anchors, `Vec` children, `Arc<str>` comments). Enable the `std`
//! feature to get `std::error::Error` impls for the error types (needed for
//! `Box<dyn Error>` in std-only consumers); `pyrs-schema` is already no_std
//! with zero dependencies. CI gate: the `no-std-check` job builds both crates
//! for `thumbv7em-none-eabi`.

#![no_std]

extern crate alloc;
// `#![no_std]` drops `std` from the extern prelude, so the std-only
// `RandomState` node-map hasher needs it named explicitly.
#[cfg(feature = "std")]
extern crate std;

pub mod ast;
pub mod error;
