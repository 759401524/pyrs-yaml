//! The third reader of the TOML IR: values out, no `CustomNode` in between.
//!
//! `from_toml` projects the scanner's intermediate representation onto the shared AST, and `events`
//! reports the same IR as a stream. Both are the right shape for a caller that edits or round-trips.
//! A caller that only wants the values - `load_toml` - pays for neither: it builds the AST it will
//! never read back, then walks it into Python. This reader hands the same projected values to the
//! caller's own sink, so the tree becomes Python without the node model in the middle.
//!
//! What it deliberately does not carry: notes, byte offsets, anchors and blank-line decoration -
//! everything only an editor asks for, and the measured reason the event stream could not serve as
//! this reader (`docs/dev/perf.md`: reporting costs about what the AST costs, so a fast route must
//! not report). What it does carry is the projection both other readers already use
//! ([`crate::scalar_projection`]), so a scalar means the same thing to all three: `0b1010` still
//! arrives tagged, a multi-line string still arrives `Literal`, a date still arrives `!!timestamp`.

use crate::parser::{TomlDialect, parse_ir};
use crate::{CowTable, ScalarProjection, TomlTable, TomlValue, scalar_projection, toml_key_style};
use alloc::sync::Arc;
use pyrs_ast::ast::ScalarStyle;

/// One table's members, in the order the reader will visit them.
///
/// A table opened by a header, a dotted key's sub-table and an inline table are the same shape to a
/// caller that only reads values: a mapping. The distinction the AST keeps (`Explicit` versus
/// `Implicit`) is a re-opening rule, so it stays in the parser.
pub trait TomlSink {
    /// The failure a sink reports, propagated out of [`read_into`].
    type Error;

    /// A mapping opens. Its members follow as `key`/value pairs, then [`TomlSink::close`].
    fn open_table(&mut self) -> Result<(), Self::Error>;

    /// A mapping member's name. Always followed by exactly one value.
    fn key(&mut self, name: Arc<str>, style: ScalarStyle) -> Result<(), Self::Error>;

    /// A scalar, in the projection the AST route would have stored.
    fn scalar(&mut self, value: ScalarProjection) -> Result<(), Self::Error>;

    /// A sequence opens - an array's value or an array-of-tables. Its elements follow, each with no
    /// key of its own, then [`TomlSink::close_array`].
    fn open_array(&mut self) -> Result<(), Self::Error>;

    /// The innermost open mapping closes.
    fn close_table(&mut self) -> Result<(), Self::Error>;

    /// The innermost open sequence closes.
    fn close_array(&mut self) -> Result<(), Self::Error>;
}

/// Why a read did not produce a value.
#[derive(Debug)]
pub enum TomlReadError<E> {
    /// The document is not TOML. The sink never saw an event, exactly as on the AST route.
    Parse(pyrs_ast::error::ParseError),
    /// The document parsed; the sink refused what it was handed.
    Sink(E),
}

/// Read a TOML document into `sink`, with the grammar pinned to `dialect`.
///
/// The scan is the one `parse_ir` runs for every other route, and the accepted set is identical:
/// this reader describes the IR that scan produced, so it cannot read a document the AST route
/// refuses, nor refuse one it accepts. The walk borrows the tree and hands over each leaf as the
/// projection the other two readers share.
pub fn read_into<S: TomlSink>(
    src: &str,
    dialect: TomlDialect,
    sink: &mut S,
) -> Result<(), TomlReadError<S::Error>> {
    let (root, _) = parse_ir(src, dialect).map_err(TomlReadError::Parse)?;
    table(&root, sink).map_err(TomlReadError::Sink)
}

/// Read a TOML document into `sink` in the default dialect - the one [`crate::from_toml`] uses.
pub fn read_toml<S: TomlSink>(src: &str, sink: &mut S) -> Result<(), TomlReadError<S::Error>> {
    read_into(src, TomlDialect::default(), sink)
}

fn table<S: TomlSink>(t: &CowTable, sink: &mut S) -> Result<(), S::Error> {
    sink.open_table()?;
    for (name, entry) in t.entries.iter() {
        sink.key(Arc::clone(name), toml_key_style(name))?;
        member(entry, sink)?;
    }
    sink.close_table()
}

fn member<S: TomlSink>(entry: &TomlTable, sink: &mut S) -> Result<(), S::Error> {
    match entry {
        TomlTable::Value(value, _) => emit(value, sink),
        TomlTable::Implicit(t) | TomlTable::Explicit(t) => table(t, sink),
        TomlTable::ArrayOfTables(list) => {
            sink.open_array()?;
            for t in list {
                table(t, sink)?;
            }
            sink.close_array()
        }
    }
}

fn emit<S: TomlSink>(item: &TomlValue, sink: &mut S) -> Result<(), S::Error> {
    match item {
        TomlValue::Array(items) => {
            sink.open_array()?;
            for (item, _) in items.iter() {
                emit(item, sink)?;
            }
            sink.close_array()
        }
        TomlValue::InlineTable(members) => {
            sink.open_table()?;
            for (name, item, _) in members.iter() {
                sink.key(Arc::clone(name), toml_key_style(name))?;
                emit(item, sink)?;
            }
            sink.close_table()
        }
        // A leaf's projection is the same answer the AST route stores and the stream reports; the
        // clone is a reference count per value, not a copy of its text.
        other => sink.scalar(scalar_projection(other.clone())),
    }
}
