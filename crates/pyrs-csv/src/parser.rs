//! The CSV/TSV reader: a byte-level scanner over RFC 4180 text.
//!
//! The scanner walks the input one byte at a time and builds the shared AST
//! directly - there is no intermediate table representation to keep in step
//! with the one the writer reads back, because there is only ever one.
//!
//! # What the reader keeps
//!
//! A cell's **source text** is what travels: the AST stores the bytes as they
//! were written and the *type* is resolved at read time by the hub, exactly
//! as a YAML scalar behaves. That is what lets a cell reading `007` stay
//! `007` on the way out and still be an integer on the way in.
//!
//! Two consequences are worth stating outright:
//!
//! - A quoted cell and an unquoted cell spell the same value, and CSV gives
//!   a value no type: the quoting is syntax, so it is not carried across.
//!   What *is* carried is the spelling of the text inside the quotes.
//! - An empty cell is null (it has no text to keep). A cell that spells a
//!   null (`null`, `~`) keeps that spelling: its type is null, but a `null`
//!   cell that came back as an *empty* cell would change the file's bytes.
//!
//! # What the reader refuses
//!
//! - A quoted field that is never closed (`E-CSV-001`). There is no silent
//!   recovery: the field could have swallowed the rest of the file.
//! - A record whose field count differs from the header's (`E-CSV-002`). No
//!   silent padding, no silent truncation - both lose a value.
//! - A header that names the same column twice (`E-CSV-003`). A mapping
//!   cannot hold two equal keys without losing one of them.

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use pyrs_ast::ast::{Chomping, CustomNode, NodeMap, NodeMeta, ScalarStyle};
use pyrs_schema::schema::resolve_core_type;
use pyrs_schema::types::YamlType;

use crate::error::CsvError;

/// The field separator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Delimiter {
    /// `,` - the RFC 4180 default.
    #[default]
    Comma,
    /// `\t` - what a TSV file uses.
    Tab,
    /// `;` - what a spreadsheet in a German locale writes.
    Semicolon,
    /// `|` - what a few exports write.
    Pipe,
}

impl Delimiter {
    /// The delimiter's byte.
    pub fn as_byte(self) -> u8 {
        match self {
            Self::Comma => b',',
            Self::Tab => b'\t',
            Self::Semicolon => b';',
            Self::Pipe => b'|',
        }
    }

    /// The delimiter a one-character argument names.
    ///
    /// The Python API takes the separator as a string (`,`, `\t`, `;`, `|`) and
    /// this is where that string becomes the engine's own vocabulary. A
    /// character the engine does not use is refused here rather than being
    /// silently accepted as "some separator", because a separator the writer
    /// cannot spell back would break the round trip.
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            ',' => Some(Self::Comma),
            '\t' => Some(Self::Tab),
            ';' => Some(Self::Semicolon),
            '|' => Some(Self::Pipe),
            _ => None,
        }
    }
}

/// The record separator the writer emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineTerminator {
    /// `\n` - the default, and what the reader accepts beside CRLF.
    #[default]
    Lf,
    /// `\r\n` - what RFC 4180 names and what a spreadsheet writes.
    CrLf,
}

impl LineTerminator {
    /// The terminator's bytes.
    pub fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Lf => b"\n",
            Self::CrLf => b"\r\n",
        }
    }
}

/// How a CSV document is read and written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CsvOptions {
    /// The field separator.
    pub delimiter: Delimiter,
    /// Whether the first record is a header row (default `true`).
    ///
    /// With a header, each data record becomes a mapping keyed by the column
    /// names; without one, each record becomes a sequence of cells.
    pub header: bool,
    /// The quote character (default `"`).
    pub quote: u8,
    /// Whether a cell's text is typed against the YAML core schema (default
    /// `true`).
    ///
    /// With inference on, a cell is a plain scalar and the hub resolves its
    /// type when it reads the document; with inference off, every non-empty
    /// cell is a quoted scalar, so it stays a string and never re-resolves.
    pub infer_types: bool,
    /// The record separator the writer emits (default `\n`).
    pub line_terminator: LineTerminator,
    /// Whether blank records are skipped (default `true`).
    pub skip_blank_lines: bool,
}

impl Default for CsvOptions {
    fn default() -> Self {
        Self {
            delimiter: Delimiter::Comma,
            header: true,
            quote: b'"',
            infer_types: true,
            line_terminator: LineTerminator::Lf,
            skip_blank_lines: true,
        }
    }
}

/// Read a CSV document into the shared AST.
///
/// The header row names the columns, and every data record becomes a
/// mapping. A record whose cell count differs from the header's, and a
/// header that names the same column twice, are refused with a position.
///
/// ```
/// use pyrs_csv::from_csv;
/// let node = from_csv("name,port\nweb-1,8080\n").unwrap();
/// assert!(matches!(node, pyrs_ast::ast::CustomNode::Sequence { .. }));
/// ```
pub fn from_csv(text: &str) -> Result<CustomNode, CsvError> {
    from_csv_with_options(text, &CsvOptions::default())
}

/// Read a CSV document into the shared AST, with the engine's own options.
pub fn from_csv_with_options(text: &str, options: &CsvOptions) -> Result<CustomNode, CsvError> {
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut scanner = Scanner::new(body, options);
    let mut items: Vec<CustomNode> = Vec::new();
    let mut header: Option<Vec<String>> = None;

    while let Some(record) = scanner.read_record()? {
        if record.blank && options.skip_blank_lines {
            continue;
        }
        match &header {
            None => {
                if options.header {
                    check_duplicate_columns(&record.fields, record.line)?;
                    header = Some(record.fields);
                } else {
                    items.push(CustomNode::Sequence {
                        items: record
                            .fields
                            .iter()
                            .map(|text| cell_node(text, options))
                            .collect(),
                        flow_style: false,
                        meta: NodeMeta::default(),
                    });
                }
            }
            Some(names) => {
                if record.fields.len() != names.len() {
                    return Err(CsvError::RaggedRow {
                        line: record.line,
                        expected: names.len(),
                        found: record.fields.len(),
                    });
                }
                let mut pairs = NodeMap::default();
                for (name, text) in names.iter().zip(record.fields.iter()) {
                    pairs.insert(column_key(name), cell_node(text, options));
                }
                items.push(CustomNode::Mapping {
                    pairs,
                    flow_style: false,
                    meta: NodeMeta::default(),
                });
            }
        }
    }

    Ok(CustomNode::Sequence {
        items,
        flow_style: false,
        meta: NodeMeta::default(),
    })
}

/// The AST node one cell becomes.
///
/// The cell's type comes from the YAML core schema - the same resolver the
/// YAML hub uses, so a CSV cell and a YAML scalar of the same text resolve
/// to the same type. The AST keeps the *source text* and lets the type be
/// resolved at read time, which is what lets `007` stay `007` and still be
/// an integer.
///
/// An empty cell is the one cell with no spelling to keep: it is null. A
/// cell that spells a null (`null`, `~`) stays a scalar, because its type
/// is null but its spelling has to travel - a `null` cell that came back as
/// an empty cell would change the file's bytes.
fn cell_node(text: &str, options: &CsvOptions) -> CustomNode {
    match resolve_core_type(text) {
        YamlType::Null if text.is_empty() => CustomNode::Null {
            meta: NodeMeta::default(),
        },
        _ => CustomNode::Scalar {
            value: Arc::from(text),
            // With inference on the cell is a plain scalar and the hub
            // resolves its type; with inference off it is quoted, so it stays
            // a string and never re-resolves.
            style: if options.infer_types {
                ScalarStyle::Plain
            } else {
                ScalarStyle::DoubleQuoted
            },
            chomping: Chomping::Clip,
            block_indent: None,
            meta: NodeMeta::default(),
        },
    }
}

/// The AST node one column name becomes.
///
/// A column name is a name, not a value: it is a string even when it spells
/// something the core schema would type (`007`, `true`). Quoting it is how
/// the AST says so, the same way the TOML engine quotes its keys - and it
/// is what keeps `load_csv`'s dictionaries keyed by the column's text.
fn column_key(name: &str) -> CustomNode {
    let style = if pyrs_schema::schema::plain_text_is_typed(name) {
        ScalarStyle::DoubleQuoted
    } else {
        ScalarStyle::Plain
    };
    CustomNode::Scalar {
        value: Arc::from(name),
        style,
        chomping: Chomping::Clip,
        block_indent: None,
        meta: NodeMeta::default(),
    }
}

/// Refuse a header that names the same column twice.
fn check_duplicate_columns(names: &[String], line: usize) -> Result<(), CsvError> {
    for (index, name) in names.iter().enumerate() {
        if names[..index].iter().any(|seen| seen == name) {
            return Err(CsvError::DuplicateHeader {
                name: name.clone(),
                line,
            });
        }
    }
    Ok(())
}

/// One record as the scanner saw it.
struct Record {
    /// The record's fields, with the quoting already resolved.
    fields: Vec<String>,
    /// The line the record started on.
    line: usize,
    /// Whether the record's own text was empty (a blank line).
    blank: bool,
}

/// The byte-level scanner over the document.
struct Scanner<'a> {
    /// The document, without its byte-order mark.
    text: &'a str,
    /// The scan position, as a byte offset into `text`.
    pos: usize,
    /// The current line, 1-based.
    line: usize,
    /// The current column, 1-based and counted in characters.
    column: usize,
    /// The quote byte.
    quote: u8,
    /// The delimiter byte.
    delimiter: u8,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str, options: &CsvOptions) -> Self {
        Self {
            text,
            pos: 0,
            line: 1,
            column: 1,
            quote: options.quote,
            delimiter: options.delimiter.as_byte(),
        }
    }

    /// The byte at the scan position.
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }

    /// The byte `ahead` positions past the scan position.
    fn peek_ahead(&self, ahead: usize) -> Option<u8> {
        self.text.as_bytes().get(self.pos + ahead).copied()
    }

    /// Advance one byte, keeping the line and column in step.
    ///
    /// The column counts *characters*, not bytes: a UTF-8 continuation byte
    /// is part of the character before it and must not move the column on.
    /// A line break inside a quoted field is a real line break in the file,
    /// so it moves the line too.
    fn bump(&mut self) {
        let Some(byte) = self.peek() else { return };
        self.pos += 1;
        match byte {
            b'\n' => {
                self.line += 1;
                self.column = 1;
            }
            0x80..=0xBF => {}
            _ => self.column += 1,
        }
    }

    /// Whether the scan position sits on a record separator.
    fn at_record_end(&self) -> bool {
        match self.peek() {
            None => true,
            Some(b'\n') => true,
            Some(b'\r') => self.peek_ahead(1) == Some(b'\n'),
            Some(_) => false,
        }
    }

    /// Read the next record, or `None` at the end of the document.
    fn read_record(&mut self) -> Result<Option<Record>, CsvError> {
        if self.pos >= self.text.len() {
            return Ok(None);
        }
        let start = self.pos;
        let line = self.line;
        let mut fields: Vec<String> = Vec::new();
        let mut field = String::new();

        loop {
            field.clear();
            if self.peek() == Some(self.quote) {
                self.read_quoted_field(&mut field)?;
            } else {
                self.read_plain_field(&mut field);
            }
            fields.push(field.clone());

            if self.peek() == Some(self.delimiter) {
                self.bump();
                continue;
            }
            break;
        }

        // The record's own text, so a blank line can be told from a record
        // that holds one empty field.
        let blank = self.text[start..self.pos].is_empty();

        // The record separator: `\n` or `\r\n`. A lone `\r` is content, not a
        // separator - RFC 4180 names the pair, and Python's reader agrees.
        if self.peek() == Some(b'\r') && self.peek_ahead(1) == Some(b'\n') {
            self.bump();
        }
        if self.peek() == Some(b'\n') {
            self.bump();
        }

        Ok(Some(Record {
            fields,
            line,
            blank,
        }))
    }

    /// Read a field that opens with the quote byte.
    ///
    /// A doubled quote inside the field is one quote, and the field ends at
    /// the quote that is not doubled. Bytes between the closing quote and
    /// the next separator are kept, which is how a lenient reader treats
    /// `"ab"c` - the value is `abc`.
    fn read_quoted_field(&mut self, out: &mut String) -> Result<(), CsvError> {
        let quote_line = self.line;
        let quote_column = self.column;
        self.bump();
        let mut run_start = self.pos;
        loop {
            match self.peek() {
                None => {
                    return Err(CsvError::UnterminatedQuote {
                        line: quote_line,
                        column: quote_column,
                    });
                }
                Some(byte) if byte == self.quote => {
                    out.push_str(&self.text[run_start..self.pos]);
                    self.bump();
                    if self.peek() == Some(self.quote) {
                        out.push(self.quote as char);
                        self.bump();
                        run_start = self.pos;
                    } else {
                        break;
                    }
                }
                Some(_) => self.bump(),
            }
        }
        // Whatever follows the closing quote, up to the next separator.
        let tail_start = self.pos;
        while !self.at_record_end() && self.peek() != Some(self.delimiter) {
            self.bump();
        }
        out.push_str(&self.text[tail_start..self.pos]);
        Ok(())
    }

    /// Read a field that does not open with the quote byte.
    ///
    /// A quote byte inside such a field is ordinary text, the way a lenient
    /// reader treats it.
    fn read_plain_field(&mut self, out: &mut String) {
        let start = self.pos;
        while !self.at_record_end() && self.peek() != Some(self.delimiter) {
            self.bump();
        }
        out.push_str(&self.text[start..self.pos]);
    }
}

#[cfg(test)]
mod tests {
    use super::{CsvOptions, Delimiter, from_csv, from_csv_with_options};
    use alloc::string::String;
    use alloc::vec::Vec;
    use pyrs_ast::ast::{CustomNode, ScalarStyle};

    /// The cells of the first mapping of a header-mode document.
    fn first_row(text: &str) -> Vec<(String, CustomNode)> {
        let node = from_csv(text).expect("the document parses");
        let CustomNode::Sequence { items, .. } = node else {
            panic!("a CSV document is a sequence");
        };
        let CustomNode::Mapping { pairs, .. } = &items[0] else {
            panic!("a header-mode document is a sequence of mappings");
        };
        pairs
            .iter()
            .map(|(key, value)| {
                let CustomNode::Scalar { value: name, .. } = key else {
                    panic!("a column name is a scalar");
                };
                (String::from(&**name), value.clone())
            })
            .collect()
    }

    /// The text of one cell.
    fn cell_text(node: &CustomNode) -> &str {
        match node {
            CustomNode::Scalar { value, .. } => value,
            CustomNode::Null { .. } => "",
            other => panic!("a cell is a scalar or null, got {other:?}"),
        }
    }

    #[test]
    fn a_leading_zero_keeps_its_spelling() {
        let rows = first_row("id\n007\n");
        assert_eq!(cell_text(&rows[0].1), "007");
    }

    #[test]
    fn a_quoted_field_may_hold_the_delimiter() {
        let rows = first_row("a\n\"x,y\"\n");
        assert_eq!(cell_text(&rows[0].1), "x,y");
    }

    #[test]
    fn a_doubled_quote_is_one_quote() {
        let rows = first_row("a\n\"x\"\"y\"\n");
        assert_eq!(cell_text(&rows[0].1), "x\"y");
    }

    #[test]
    fn a_quoted_field_may_hold_a_line_break() {
        let rows = first_row("a\n\"x\ny\"\n");
        assert_eq!(cell_text(&rows[0].1), "x\ny");
    }

    #[test]
    fn an_empty_cell_is_null() {
        let rows = first_row("a,b\n,\n");
        assert!(matches!(rows[0].1, CustomNode::Null { .. }));
    }

    #[test]
    fn a_null_spelling_keeps_its_text() {
        let rows = first_row("a\nnull\n");
        assert_eq!(cell_text(&rows[0].1), "null");
    }

    #[test]
    fn a_byte_order_mark_is_stripped() {
        let rows = first_row("\u{feff}a\n1\n");
        assert_eq!(rows[0].0, "a");
    }

    #[test]
    fn a_crlf_record_separator_is_accepted() {
        let rows = first_row("a,b\r\n1,2\r\n");
        assert_eq!(rows.len(), 2);
        assert_eq!(cell_text(&rows[1].1), "2");
    }

    #[test]
    fn a_blank_line_is_skipped() {
        // The header is `a`; the blank line between the header and the row is
        // not a record, so the document holds exactly one row.
        let rows = first_row("a\n\n1\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(cell_text(&rows[0].1), "1");
    }

    #[test]
    fn a_ragged_row_is_refused() {
        let error = from_csv("a,b\n1\n").expect_err("a short row is refused");
        assert_eq!(error.code(), "E-CSV-002");
    }

    #[test]
    fn a_duplicate_column_is_refused() {
        let error = from_csv("a,a\n1,2\n").expect_err("a duplicate column is refused");
        assert_eq!(error.code(), "E-CSV-003");
    }

    #[test]
    fn an_unterminated_quote_is_refused() {
        let error = from_csv("a\n\"oops\n").expect_err("an unclosed quote is refused");
        assert_eq!(error.code(), "E-CSV-001");
    }

    #[test]
    fn the_no_header_mode_reads_rows_as_sequences() {
        let node = from_csv_with_options(
            "1,2\n3,4\n",
            &CsvOptions {
                header: false,
                ..CsvOptions::default()
            },
        )
        .expect("the document parses");
        let CustomNode::Sequence { items, .. } = node else {
            panic!("a CSV document is a sequence");
        };
        assert_eq!(items.len(), 2);
        let CustomNode::Sequence { items: cells, .. } = &items[0] else {
            panic!("a no-header row is a sequence");
        };
        assert_eq!(cell_text(&cells[1]), "2");
    }

    #[test]
    fn the_inference_off_mode_quotes_every_cell() {
        let node = from_csv_with_options(
            "a\nhello\n",
            &CsvOptions {
                infer_types: false,
                ..CsvOptions::default()
            },
        )
        .expect("the document parses");
        let CustomNode::Sequence { items, .. } = node else {
            panic!("a CSV document is a sequence");
        };
        let CustomNode::Mapping { pairs, .. } = &items[0] else {
            panic!("a header-mode document is a sequence of mappings");
        };
        let (_, value) = pairs.iter().next().expect("the row has a cell");
        let CustomNode::Scalar { style, .. } = value else {
            panic!("a cell is a scalar");
        };
        assert_eq!(*style, ScalarStyle::DoubleQuoted);
    }

    #[test]
    fn a_tab_delimiter_reads_a_tsv_document() {
        let node = from_csv_with_options(
            "a\tb\n1\t2\n",
            &CsvOptions {
                delimiter: Delimiter::Tab,
                ..CsvOptions::default()
            },
        )
        .expect("the document parses");
        let CustomNode::Sequence { items, .. } = node else {
            panic!("a CSV document is a sequence");
        };
        let CustomNode::Mapping { pairs, .. } = &items[0] else {
            panic!("a header-mode document is a sequence of mappings");
        };
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn a_document_with_no_records_is_an_empty_table() {
        let node = from_csv("").expect("an empty document parses");
        let CustomNode::Sequence { items, .. } = node else {
            panic!("a CSV document is a sequence");
        };
        assert!(items.is_empty());
    }
}
