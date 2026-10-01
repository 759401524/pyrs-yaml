//! # pyrs-schema
//!
//! Format-agnostic scalar-type vocabulary (`YamlType`, `YamlSchema`, `Schema`,
//! the `SchemaResolver` extension trait) and the built-in plain-scalar
//! resolution rules (YAML 1.2 Core, JSON, JSON5, YAML 1.1, failsafe) plus the
//! scalar-quoting predicates the serializers share.
//!
//! `no_std` + dependency-free — the generic "which type is this scalar"
//! infrastructure every format engine reuses (the JSON/TOML writers resolve
//! plain scalars through it). Kept separate from `pyrs-ast` (the node model)
//! and from the YAML-specific schema *registry* / schema-language layers,
//! which live in the engine crate.
//!
//! The crate is `#![no_std]` by default and only pulls in `alloc` for the
//! owned parts of the scalar model (`String` formatting, `Arc`-shared scalar
//! strings). `cargo build -p pyrs-schema --target thumbv7em-none-eabi`
//! is the CI gate that keeps it honest.

#![no_std]

extern crate alloc;

pub mod schema;
pub mod types;

/// True for Unicode noncharacters (…FFFE / …FFFF planes) that granit-parser and
/// the YAML emitter reject even inside quoted scalars. Shared by the scalar
/// resolvers here and the YAML serializer.
pub fn is_yaml_noncharacter(c: char) -> bool {
    (c as u32) & 0xFFFE == 0xFFFE
}
