//! The CSV/TSV writer: the shared AST rendered back to CSV text.
//!
//! # What the writer writes
//!
//! A cell is written from its **source text**, never from a re-serialized
//! numeric value - that is what makes a CSV -> AST -> CSV cycle
//! byte-exact. The text is spelled plain unless it cannot be:
//!
//! - A plain spelling that holds the delimiter, a quote, `CR`, `LF`, or
//!   leading or trailing whitespace is not a spelling the reader would read
//!   back, so it is quoted and an embedded quote is doubled.
//! - A node that declares itself a string (a quoted or a block scalar) is
//!   quoted when its text would otherwise re-infer as a non-string, so a
//!   YAML string `"true"` comes back as a *quoted* cell and re-imports as a
//!   string.
//!
//! A column name is a name, not a value: it is written with the plain
//! spelling rule, so a header that reads `007` is written back as `007` and
//! not as `"007"`.
//!
//! # What the writer refuses
//!
//! - A shape that is not a table (`E-CSV-004`): a sequence of mappings, a
//!   sequence of sequences, or a single mapping.
//! - A cell the format cannot hold (`E-CSV-005`): a mapping or a sequence
//!   inside a cell, a non-scalar key, an alias, and a row that does not
//!   carry the columns the first row carried.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use pyrs_ast::ast::{CustomNode, NodeMap, ScalarStyle};
use pyrs_schema::schema::core_type_is_non_string;

use crate::error::CsvError;
use crate::parser::CsvOptions;

/// Render the shared AST as CSV text.
///
/// A sequence of mappings becomes a header row and one row per mapping; a
/// sequence of sequences becomes rows only; a mapping becomes a single row
/// under its own header.
///
/// ```
/// use pyrs_csv::{from_csv, to_csv_text};
/// let node = from_csv("name,port\nweb-1,8080\n").unwrap();
/// assert_eq!(to_csv_text(&node).unwrap(), "name,port\nweb-1,8080\n");
/// ```
pub fn to_csv_text(node: &CustomNode) -> Result<String, CsvError> {
    to_csv_text_with_options(node, &CsvOptions::default())
}

/// Render the shared AST as CSV text, with the engine's own options.
pub fn to_csv_text_with_options(
    node: &CustomNode,
    options: &CsvOptions,
) -> Result<String, CsvError> {
    let mut out = String::new();
    match node {
        CustomNode::Sequence { items, .. } => {
            let Some(first) = items.first() else {
                // An empty table is an empty document: there is no row to
                // write, and no header to write without one.
                return Ok(out);
            };
            match first {
                CustomNode::Mapping { .. } => {
                    let CustomNode::Mapping { pairs, .. } = first else {
                        unreachable!("the arm matched a mapping");
                    };
                    let columns = column_names(pairs)?;
                    if options.header {
                        write_header_row(&mut out, &columns, options);
                    }
                    for (index, item) in items.iter().enumerate() {
                        let CustomNode::Mapping { pairs, .. } = item else {
                            return Err(CsvError::UnsupportedCell {
                                path: format!("$[{index}]"),
                                detail: "every row of a table is a mapping".to_string(),
                            });
                        };
                        let names = column_names(pairs)?;
                        check_same_columns(&columns, &names, index)?;
                        write_mapping_row(&mut out, pairs, &columns, options)?;
                    }
                }
                CustomNode::Sequence { .. } => {
                    for (index, item) in items.iter().enumerate() {
                        let CustomNode::Sequence { items: cells, .. } = item else {
                            return Err(CsvError::UnsupportedCell {
                                path: format!("$[{index}]"),
                                detail: "every row of a table is a sequence".to_string(),
                            });
                        };
                        write_sequence_row(&mut out, cells, options)?;
                    }
                }
                _ => {
                    return Err(CsvError::UnsupportedShape {
                        detail: "a table is a sequence of mappings or of sequences".to_string(),
                    });
                }
            }
        }
        CustomNode::Mapping { pairs, .. } => {
            let columns = column_names(pairs)?;
            if options.header {
                write_header_row(&mut out, &columns, options);
            }
            write_mapping_row(&mut out, pairs, &columns, options)?;
        }
        CustomNode::Scalar { .. } | CustomNode::Null { .. } | CustomNode::Alias { .. } => {
            return Err(CsvError::UnsupportedShape {
                detail: "a table is a sequence of mappings or of sequences".to_string(),
            });
        }
    }
    Ok(out)
}

/// The column names a mapping carries, in the mapping's own order.
///
/// A key that is not a scalar has no CSV spelling: a column is named by a
/// name, and a name is text.
fn column_names(pairs: &NodeMap<CustomNode, CustomNode>) -> Result<Vec<&str>, CsvError> {
    let mut names = Vec::with_capacity(pairs.len());
    for (key, _) in pairs.iter() {
        let CustomNode::Scalar { value, .. } = key else {
            return Err(CsvError::UnsupportedCell {
                path: "$".to_string(),
                detail: "a column name must be a scalar".to_string(),
            });
        };
        names.push(&**value);
    }
    Ok(names)
}

/// Refuse a row that does not carry the columns the first row carried.
///
/// The comparison is on the *set* of names, not on their order: a row whose
/// columns are written down in another order still has the same columns, and
/// the writer lays it out under the first row's order. A row that carries a
/// different column, or the same column twice, is refused - padding it or
/// dropping a cell would lose a value the caller cannot see losing.
fn check_same_columns(columns: &[&str], names: &[&str], index: usize) -> Result<(), CsvError> {
    let mut expected: Vec<&str> = columns.to_vec();
    let mut found: Vec<&str> = names.to_vec();
    expected.sort_unstable();
    found.sort_unstable();
    if expected == found {
        return Ok(());
    }
    Err(CsvError::UnsupportedCell {
        path: format!("$[{index}]"),
        detail: format!(
            "the row carries the columns {} but the table carries {}",
            names.join(", "),
            columns.join(", ")
        ),
    })
}

/// Write the header row: the column names, in the table's order.
fn write_header_row(out: &mut String, columns: &[&str], options: &CsvOptions) {
    for (index, name) in columns.iter().enumerate() {
        if index > 0 {
            out.push(options.delimiter.as_byte() as char);
        }
        // A name is written with the plain spelling rule: it is a name, not a
        // value, so its type is none of the writer's business.
        write_cell(out, name, options, false);
    }
    push_terminator(out, options);
}

/// Write one mapping as a row, under the table's column order.
fn write_mapping_row(
    out: &mut String,
    pairs: &NodeMap<CustomNode, CustomNode>,
    columns: &[&str],
    options: &CsvOptions,
) -> Result<(), CsvError> {
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            out.push(options.delimiter.as_byte() as char);
        }
        let value = value_of(pairs, column);
        let path = format!("$[{index}].{column}");
        match value {
            Some(node) => write_value(out, node, options, &path)?,
            None => {
                // The row passed the column check, so this cannot happen;
                // an empty field is the safe answer if it ever does.
            }
        }
    }
    push_terminator(out, options);
    Ok(())
}

/// Write one sequence as a row.
fn write_sequence_row(
    out: &mut String,
    cells: &[CustomNode],
    options: &CsvOptions,
) -> Result<(), CsvError> {
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            out.push(options.delimiter.as_byte() as char);
        }
        write_value(out, cell, options, &format!("$[{index}]"))?;
    }
    push_terminator(out, options);
    Ok(())
}

/// The value a mapping carries under a column name.
fn value_of<'a>(
    pairs: &'a NodeMap<CustomNode, CustomNode>,
    column: &str,
) -> Option<&'a CustomNode> {
    pairs
        .iter()
        .find(|(key, _)| matches!(key, CustomNode::Scalar { value, .. } if &**value == column))
        .map(|(_, value)| value)
}

/// Write one cell.
fn write_value(
    out: &mut String,
    node: &CustomNode,
    options: &CsvOptions,
    path: &str,
) -> Result<(), CsvError> {
    match node {
        CustomNode::Scalar { value, style, .. } => {
            // A node that declares itself a string - a quoted scalar, or a
            // block scalar - is quoted when its text would otherwise
            // re-infer as a non-string, so a YAML string `"true"` comes back
            // as a quoted cell and re-imports as a string.
            let force = is_string_style(*style) && core_type_is_non_string(value);
            write_cell(out, value, options, force);
            Ok(())
        }
        CustomNode::Null { .. } => {
            // A null is an empty field.
            Ok(())
        }
        CustomNode::Mapping { .. } | CustomNode::Sequence { .. } => {
            Err(CsvError::UnsupportedCell {
                path: path.to_string(),
                detail: "a cell cannot hold a mapping or a sequence".to_string(),
            })
        }
        CustomNode::Alias { .. } => Err(CsvError::UnsupportedCell {
            path: path.to_string(),
            detail: "an alias has no CSV spelling".to_string(),
        }),
    }
}

/// Whether a scalar's style declares it a string rather than a plain value.
fn is_string_style(style: ScalarStyle) -> bool {
    matches!(
        style,
        ScalarStyle::DoubleQuoted
            | ScalarStyle::SingleQuoted
            | ScalarStyle::Literal
            | ScalarStyle::Folded
    )
}

/// Write one field, quoted when the plain spelling would not read back.
fn write_cell(out: &mut String, text: &str, options: &CsvOptions, force_quote: bool) {
    if !force_quote && !must_quote(text, options) {
        out.push_str(text);
        return;
    }
    out.push(options.quote as char);
    for ch in text.chars() {
        // Only an ASCII quote can be doubled: a multi-byte character never
        // carries one, and splitting it would corrupt the text.
        if ch.len_utf8() == 1 && (ch as u8) == options.quote {
            out.push(options.quote as char);
        }
        out.push(ch);
    }
    out.push(options.quote as char);
}

/// Whether the plain spelling of `text` would not read back as `text`.
///
/// The reader reads a plain field as everything up to the next separator, so
/// a field that holds the delimiter, a quote, a line break, or edge
/// whitespace needs the quotes to survive.
fn must_quote(text: &str, options: &CsvOptions) -> bool {
    let bytes = text.as_bytes();
    bytes.contains(&options.delimiter.as_byte())
        || bytes.contains(&options.quote)
        || text.contains('\r')
        || text.contains('\n')
        || text.starts_with(is_edge_blank)
        || text.ends_with(is_edge_blank)
}

/// Whether a character is separation at the edge of a field.
fn is_edge_blank(ch: char) -> bool {
    pyrs_schema::is_yaml_blank(ch)
}

/// The record separator.
fn push_terminator(out: &mut String, options: &CsvOptions) {
    match options.line_terminator {
        crate::parser::LineTerminator::Lf => out.push('\n'),
        crate::parser::LineTerminator::CrLf => out.push_str("\r\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::{to_csv_text, to_csv_text_with_options};
    use crate::parser::{CsvOptions, Delimiter, LineTerminator};
    use alloc::sync::Arc;
    use alloc::vec::Vec;
    use pyrs_ast::ast::{Chomping, CustomNode, NodeMap, NodeMeta, ScalarStyle};

    /// A plain scalar.
    fn plain(text: &str) -> CustomNode {
        CustomNode::Scalar {
            value: Arc::from(text),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            block_indent: None,
            meta: NodeMeta::default(),
        }
    }

    /// A quoted scalar - a string by declaration.
    fn quoted(text: &str) -> CustomNode {
        CustomNode::Scalar {
            value: Arc::from(text),
            style: ScalarStyle::DoubleQuoted,
            chomping: Chomping::Clip,
            block_indent: None,
            meta: NodeMeta::default(),
        }
    }

    /// A mapping from text to text.
    fn mapping(entries: &[(&str, CustomNode)]) -> CustomNode {
        let mut pairs: NodeMap<CustomNode, CustomNode> = NodeMap::default();
        for (key, value) in entries {
            pairs.insert(plain(key), value.clone());
        }
        CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: NodeMeta::default(),
        }
    }

    /// A sequence of nodes.
    fn sequence(items: Vec<CustomNode>) -> CustomNode {
        CustomNode::Sequence {
            items,
            flow_style: false,
            meta: NodeMeta::default(),
        }
    }

    #[test]
    fn a_table_becomes_a_header_and_rows() {
        let node = sequence(vec![mapping(&[("a", plain("1")), ("b", plain("2"))])]);
        assert_eq!(to_csv_text(&node).unwrap(), "a,b\n1,2\n");
    }

    #[test]
    fn a_leading_zero_is_not_re_serialized() {
        let node = sequence(vec![mapping(&[("id", plain("007"))])]);
        assert_eq!(to_csv_text(&node).unwrap(), "id\n007\n");
    }

    #[test]
    fn a_string_that_would_re_infer_is_quoted() {
        let node = sequence(vec![mapping(&[("a", quoted("true"))])]);
        assert_eq!(to_csv_text(&node).unwrap(), "a\n\"true\"\n");
    }

    #[test]
    fn a_plain_value_that_would_re_infer_is_not_quoted() {
        let node = sequence(vec![mapping(&[("a", plain("true"))])]);
        assert_eq!(to_csv_text(&node).unwrap(), "a\ntrue\n");
    }

    #[test]
    fn a_value_holding_the_delimiter_is_quoted() {
        let node = sequence(vec![mapping(&[("a", plain("x,y"))])]);
        assert_eq!(to_csv_text(&node).unwrap(), "a\n\"x,y\"\n");
    }

    #[test]
    fn a_value_holding_a_quote_is_quoted_and_doubled() {
        let node = sequence(vec![mapping(&[("a", plain("x\"y"))])]);
        assert_eq!(to_csv_text(&node).unwrap(), "a\n\"x\"\"y\"\n");
    }

    #[test]
    fn a_value_holding_a_line_break_is_quoted() {
        let node = sequence(vec![mapping(&[("a", plain("x\ny"))])]);
        assert_eq!(to_csv_text(&node).unwrap(), "a\n\"x\ny\"\n");
    }

    #[test]
    fn a_null_is_an_empty_field() {
        let node = sequence(vec![mapping(&[(
            "a",
            CustomNode::Null {
                meta: NodeMeta::default(),
            },
        )])]);
        assert_eq!(to_csv_text(&node).unwrap(), "a\n\n");
    }

    #[test]
    fn a_sequence_of_sequences_has_no_header() {
        let node = sequence(vec![sequence(vec![plain("1"), plain("2")])]);
        assert_eq!(to_csv_text(&node).unwrap(), "1,2\n");
    }

    #[test]
    fn a_mapping_is_a_single_row() {
        let node = mapping(&[("a", plain("1"))]);
        assert_eq!(to_csv_text(&node).unwrap(), "a\n1\n");
    }

    #[test]
    fn the_header_can_be_left_out() {
        let node = sequence(vec![mapping(&[("a", plain("1"))])]);
        let options = CsvOptions {
            header: false,
            ..CsvOptions::default()
        };
        assert_eq!(to_csv_text_with_options(&node, &options).unwrap(), "1\n");
    }

    #[test]
    fn a_crlf_terminator_is_emitted() {
        let node = sequence(vec![mapping(&[("a", plain("1"))])]);
        let options = CsvOptions {
            line_terminator: LineTerminator::CrLf,
            ..CsvOptions::default()
        };
        assert_eq!(
            to_csv_text_with_options(&node, &options).unwrap(),
            "a\r\n1\r\n"
        );
    }

    #[test]
    fn a_tab_delimiter_is_emitted() {
        let node = sequence(vec![mapping(&[("a", plain("1")), ("b", plain("2"))])]);
        let options = CsvOptions {
            delimiter: Delimiter::Tab,
            ..CsvOptions::default()
        };
        assert_eq!(
            to_csv_text_with_options(&node, &options).unwrap(),
            "a\tb\n1\t2\n"
        );
    }

    #[test]
    fn a_cell_holding_a_mapping_is_refused() {
        let node = sequence(vec![mapping(&[("a", mapping(&[("b", plain("1"))]))])]);
        let error = to_csv_text(&node).expect_err("a nested mapping has no cell");
        assert_eq!(error.code(), "E-CSV-005");
    }

    #[test]
    fn a_scalar_document_is_refused() {
        let error = to_csv_text(&plain("x")).expect_err("a scalar is not a table");
        assert_eq!(error.code(), "E-CSV-004");
    }

    #[test]
    fn a_row_with_other_columns_is_refused() {
        let node = sequence(vec![
            mapping(&[("a", plain("1"))]),
            mapping(&[("b", plain("2"))]),
        ]);
        let error = to_csv_text(&node).expect_err("the columns differ");
        assert_eq!(error.code(), "E-CSV-005");
    }
}
