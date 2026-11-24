//! The CSV engine's error type: five refusals, each with a stable code.
//!
//! Every variant names a position (a line, a column, a path) rather than
//! only a shape, because a CSV file is edited by hand: "this row has the
//! wrong number of cells" is a fact the caller has to find in the file, and
//! the engine's job is to say *where*.
//!
//! The codes are stable strings (`E-CSV-001` … `E-CSV-005`) because they
//! cross a process boundary - the Python layer maps them onto the
//! localized `csv-parse-error` / `csv-serialize-error` catalog keys, and a
//! script that matches on the code must not have to parse a message that
//! is translated four ways.

use alloc::string::String;
use core::fmt;

/// A CSV read or write refusal.
///
/// The variants split along the two doors of the engine: the first three are
/// parse refusals (the text is not a table this engine can read), the last
/// two are serialize refusals (the tree is not a table this engine can
/// write).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CsvError {
    /// A quoted field was never closed (`E-CSV-001`).
    ///
    /// There is no silent recovery: a field that was opened with a quote and
    /// never closed could have swallowed the rest of the file, and a reader
    /// that guessed where it ended would turn a typo into a table.
    UnterminatedQuote {
        /// 1-based line the unclosed quote opened on.
        line: usize,
        /// 1-based column the unclosed quote sits at.
        column: usize,
    },
    /// A record's field count differs from the header's (`E-CSV-002`).
    ///
    /// No silent padding and no silent truncation: both lose a value the
    /// caller cannot see losing.
    RaggedRow {
        /// 1-based line of the offending record.
        line: usize,
        /// The field count the header declared.
        expected: usize,
        /// The field count the record actually carries.
        found: usize,
    },
    /// Two columns carry the same name (`E-CSV-003`).
    ///
    /// A mapping cannot hold two equal keys without losing one of them, so
    /// the header itself is the defect.
    DuplicateHeader {
        /// The name both columns carry.
        name: String,
        /// 1-based line the second occurrence is on.
        line: usize,
    },
    /// The AST shape is not a table (`E-CSV-004`).
    UnsupportedShape {
        /// What the shape was, and what a table would have been.
        detail: String,
    },
    /// A cell (or a key) has no CSV spelling (`E-CSV-005`).
    UnsupportedCell {
        /// Where the offending cell sits (`$[0].name`, `$.key`, ...).
        path: String,
        /// What cannot be spelled.
        detail: String,
    },
}

impl CsvError {
    /// The stable code this refusal carries.
    ///
    /// The codes are part of the engine's contract: the Python layer and any
    /// script that matches on them depend on these strings, so they do not
    /// change with the message text or the locale.
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnterminatedQuote { .. } => "E-CSV-001",
            Self::RaggedRow { .. } => "E-CSV-002",
            Self::DuplicateHeader { .. } => "E-CSV-003",
            Self::UnsupportedShape { .. } => "E-CSV-004",
            Self::UnsupportedCell { .. } => "E-CSV-005",
        }
    }

    /// Whether this refusal came from the reader or from the writer.
    ///
    /// The Python layer has two catalog keys - one for a parse failure and one
    /// for a serialize failure - and this is the question that picks between
    /// them, so the answer lives with the error instead of being re-derived
    /// at every call site.
    pub fn is_parse_error(&self) -> bool {
        matches!(
            self,
            Self::UnterminatedQuote { .. } | Self::RaggedRow { .. } | Self::DuplicateHeader { .. }
        )
    }
}

impl fmt::Display for CsvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnterminatedQuote { line, column } => write!(
                f,
                "unterminated quoted field starting at line {line}, column {column}"
            ),
            Self::RaggedRow {
                line,
                expected,
                found,
            } => write!(
                f,
                "ragged row at line {line}: expected {expected} fields, found {found}"
            ),
            Self::DuplicateHeader { name, line } => {
                write!(f, "duplicate header column {name:?} at line {line}")
            }
            Self::UnsupportedShape { detail } => {
                write!(f, "unsupported shape for CSV: {detail}")
            }
            Self::UnsupportedCell { path, detail } => {
                write!(f, "unsupported cell at {path}: {detail}")
            }
        }
    }
}

// `core::error::Error` is the `no_std` trait; with the `std` feature on it
// is the same trait `std::error::Error` names, so a `Box<dyn Error>` on the
// host accepts it without a second impl.
impl core::error::Error for CsvError {}

#[cfg(test)]
mod tests {
    use super::CsvError;
    use alloc::string::ToString;

    #[test]
    fn every_refusal_carries_its_own_code() {
        let cases = [
            (
                CsvError::UnterminatedQuote { line: 1, column: 2 },
                "E-CSV-001",
            ),
            (
                CsvError::RaggedRow {
                    line: 3,
                    expected: 2,
                    found: 3,
                },
                "E-CSV-002",
            ),
            (
                CsvError::DuplicateHeader {
                    name: "a".to_string(),
                    line: 1,
                },
                "E-CSV-003",
            ),
            (
                CsvError::UnsupportedShape {
                    detail: "scalar".to_string(),
                },
                "E-CSV-004",
            ),
            (
                CsvError::UnsupportedCell {
                    path: "$[0].a".to_string(),
                    detail: "mapping".to_string(),
                },
                "E-CSV-005",
            ),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code);
        }
    }

    #[test]
    fn the_reader_and_the_writer_refusals_are_told_apart() {
        assert!(CsvError::UnterminatedQuote { line: 1, column: 1 }.is_parse_error());
        assert!(
            !CsvError::UnsupportedShape {
                detail: "scalar".to_string()
            }
            .is_parse_error()
        );
    }

    #[test]
    fn a_refusal_names_its_position() {
        let error = CsvError::RaggedRow {
            line: 7,
            expected: 2,
            found: 4,
        };
        let text = error.to_string();
        assert!(text.contains('7'), "{text}");
        assert!(text.contains('2'), "{text}");
        assert!(text.contains('4'), "{text}");
    }
}
