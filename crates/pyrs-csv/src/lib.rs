//! CSV/TSV engine — the hub's tabular spoke.
//!
//! One engine, two dialects: a TSV file is a CSV file with a tab where the
//! comma was. The delimiter is an option, not a second format.
//!
//! ```
//! use pyrs_csv::{from_csv, to_csv_text};
//! let node = from_csv("name,port\nweb-1,8080\n").unwrap();
//! assert_eq!(to_csv_text(&node).unwrap(), "name,port\nweb-1,8080\n");
//! ```
//!
//! # A table, not a document
//!
//! CSV has no document model: a file is one table. The header row names the
//! columns and every data record becomes a mapping, so the shared AST holds
//! a sequence of mappings. Without a header (`CsvOptions::header`), every
//! record becomes a sequence of cells instead.
//!
//! # Inference decides the type, the spelling travels
//!
//! A cell is typed against the YAML core schema — the *same* resolver the
//! YAML hub uses, so a CSV cell and a YAML scalar of the same text resolve
//! to the same type and there is never a second, drifting set of rules. The
//! AST keeps the cell's **source text** and resolves the type at read time,
//! which is what lets a cell reading `007` stay `007` on the way out and
//! still be an integer on the way in. A CSV → YAML → CSV cycle therefore
//! reproduces the original bytes.
//!
//! | CSV cell | AST | reads back as |
//! |----------|-----|---------------|
//! | `007` | plain scalar `007` | the integer 7, spelled `007` |
//! | `true` | plain scalar `true` | the boolean true |
//! | (empty) | null | null |
//! | `x,y` | plain scalar `x,y` | the string `x,y` |
//!
//! # Two refusals, no repairs
//!
//! A record whose field count differs from the header's (`E-CSV-002`) and a
//! header that names the same column twice (`E-CSV-003`) are refused with a
//! position. Neither is padded, truncated or merged, because every repair
//! loses a value the caller cannot see losing. A quoted field that is never
//! closed (`E-CSV-001`) is refused too — the field could have swallowed the
//! rest of the file, and a reader that guessed where it ended would turn a
//! typo into a table.
//!
//! # Exchange, not an editing substrate
//!
//! Round-trip *editing* — comments, styles, splices — stays YAML-only by
//! design. CSV has no comment syntax, one file holds one table, and there
//! is no splice engine here. `pyrs-csv` sits beside `pyrs-json` and
//! `pyrs-toml` and does not depend on `pyrs-yaml-core`: YAML stays the only
//! editable representation.
//!
//! # `no_std`
//!
//! The crate is `#![no_std]` and uses `alloc`, so it builds for a target
//! with no standard library at all (`cargo build --no-default-features
//! --target thumbv7em-none-eabi -p pyrs-csv`). The `std` feature is on by
//! default for the host test harness.

#![no_std]

// `format!`/`vec!` live in `alloc`'s macro exports, which a `no_std` crate
// only sees through `#[macro_use]` — the std prelude that carries them for
// a normal crate is deliberately absent here.
#[macro_use]
extern crate alloc;
// `#![no_std]` drops `std` from the extern prelude, so the host test harness
// needs it named explicitly when the `std` feature is on.
#[cfg(feature = "std")]
extern crate std;

pub mod error;
pub mod parser;
pub mod writer;

pub use error::CsvError;
pub use parser::{CsvOptions, Delimiter, LineTerminator, from_csv, from_csv_with_options};
pub use writer::{to_csv_text, to_csv_text_with_options};
