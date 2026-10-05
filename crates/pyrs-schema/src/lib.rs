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

/// True when `c` may appear inside YAML document content.
///
/// This is YAML 1.2's `c-printable` set - tab, LF, CR, `#x20-#x7E`, `#x85`,
/// `#xA0-#xD7FF`, `#xE000-#xFFFD`, `#x10000-#x10FFFF` - minus the Unicode
/// noncharacters (the plane-end `…FFFE`/`…FFFF` twins), which granit rejects
/// outright. `#xD800-#xDFFF` cannot occur because `char` never holds a surrogate.
///
/// U+FEFF is then excluded on top: it *is* printable, but YAML restricts it to
/// the stream's own leading byte-order mark, so it can never sit inside a body.
/// Callers that must round-trip text they do not control - notably YAML comments,
/// which have no escape syntax at all - use this to drop what the reader would
/// reject rather than emit output that no longer parses.
pub fn is_yaml_document_char(c: char) -> bool {
    if c == '\u{FEFF}' {
        return false;
    }
    matches!(
        c as u32,
        0x09
        | 0x0A
        | 0x0D
        | 0x20..=0x7E
        | 0x85
        | 0xA0..=0xD7FF
        | 0xE000..=0xFFFD
        | 0x1_0000..=0x10_FFFF
    ) && !is_yaml_noncharacter(c)
}

/// True for the characters YAML may consume as *separation* only: `s-space`
/// (SP, TAB) plus the break set (CR, LF) and `z` (NUL) — exactly granit's
/// `is_blank` ∪ `is_breakz`.
///
/// This is deliberately **not** `char::is_whitespace`, which is Unicode-based
/// and also matches NBSP (U+00A0), U+0085, U+2028/U+2029 and the other
/// separators. YAML gives those no special meaning: a lone NBSP is an ordinary
/// plain-scalar character and may sit inside an anchor name. Any scan that asks
/// "is this text blank" therefore has to use this predicate — `str::trim`
/// silently turns a NBSP-only document into nothing (libFuzzer
/// `yaml_roundtrip` crash-512814, 5 bytes BOM + NBSP, where the empty-document
/// fast path reported `null` for a scalar).
#[must_use]
pub fn is_yaml_blank(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n' | '\0')
}

/// True when `text` holds nothing but separation, by [`is_yaml_blank`] — i.e.
/// what the reader would see as an empty document or an empty comment body.
#[must_use]
pub fn is_yaml_blank_only(text: &str) -> bool {
    text.chars().all(is_yaml_blank)
}
