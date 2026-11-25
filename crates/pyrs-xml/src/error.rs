//! The XML engine's read refusals: fourteen stable codes, each with a position.
//!
//! Every variant that comes from input names a line and column (or an option
//! field) rather than only a shape, because XML is edited by hand: the
//! engine's job is to say *where*. The codes are stable strings because the
//! Python layer maps them onto the localized `xml-parse-error` catalog key,
//! and a script matching on a code must not have to parse a message that is
//! translated four ways.
//!
//! There is no write-side refusal: this crate has no writer, so every variant
//! is a parse error (`is_parse_error()` is always `true`).

use alloc::string::String;
use core::fmt;

/// A read refusal from the XML engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XmlError {
    /// `<!DOCTYPE` was seen; refused at the markup boundary (`E-XML-001`).
    DtdDeclared {
        /// 1-based line the token starts on.
        line: usize,
        /// 1-based column the token starts on.
        column: usize,
    },
    /// `<!ENTITY` was seen, inside or outside a DTD (`E-XML-002`).
    EntityDeclaration {
        /// 1-based line the token starts on.
        line: usize,
        /// 1-based column the token starts on.
        column: usize,
    },
    /// A named reference outside the five predefined ones (`E-XML-003`).
    UndeclaredEntity {
        /// The entity name between `&` and `;`.
        name: String,
        /// 1-based line of the `&`.
        line: usize,
        /// 1-based column of the `&`.
        column: usize,
    },
    /// Well-formedness failure the scanner/mapping cannot repair (`E-XML-004`).
    MalformedMarkup {
        /// 1-based line the defect starts on.
        line: usize,
        /// 1-based column the defect starts on.
        column: usize,
        /// What was wrong, in engine vocabulary.
        detail: String,
    },
    /// An end tag that does not match its open tag literally (`E-XML-005`).
    MismatchedEndTag {
        /// The open tag's QName.
        expected: String,
        /// The end tag's QName.
        found: String,
        /// 1-based line of the end tag.
        line: usize,
        /// 1-based column of the end tag.
        column: usize,
    },
    /// A second root-level element (`E-XML-006`).
    MultipleRoots {
        /// 1-based line of the second root.
        line: usize,
        /// 1-based column of the second root.
        column: usize,
    },
    /// A comment or PI outside the root element (`E-XML-007`).
    UnsupportedDocumentNode {
        /// `comment` or `processing-instruction`.
        kind: String,
        /// 1-based line of the node.
        line: usize,
        /// 1-based column of the node.
        column: usize,
    },
    /// An XML declaration other than version 1.0 / UTF-8 (`E-XML-008`).
    UnsupportedDeclaration {
        /// What was unsupported.
        detail: String,
    },
    /// The same attribute name twice on one element (`E-XML-009`).
    DuplicateAttribute {
        /// The duplicated attribute QName.
        name: String,
        /// 1-based line of the second occurrence.
        line: usize,
        /// 1-based column of the second occurrence.
        column: usize,
    },
    /// A numeric reference outside XML 1.0 valid characters (`E-XML-010`).
    InvalidCharacterReference {
        /// The reference text, e.g. `&#xD800;`.
        text: String,
        /// 1-based line of the `&`.
        line: usize,
        /// 1-based column of the `&`.
        column: usize,
    },
    /// Element nesting exceeded `XmlOptions::max_depth` (`E-XML-011`).
    DepthLimitExceeded {
        /// The configured cap.
        limit: usize,
    },
    /// Too many materialised nodes (`E-XML-012`).
    NodeLimitExceeded {
        /// The configured cap.
        limit: usize,
    },
    /// A QName prefix with no in-scope `xmlns:` binding (`E-XML-013`).
    UndeclaredPrefix {
        /// The prefix without a colon.
        prefix: String,
        /// 1-based line of the QName.
        line: usize,
        /// 1-based column of the QName.
        column: usize,
    },
    /// `XmlOptions::validate()` failed before any input was read (`E-XML-014`).
    InvalidOptions {
        /// `max_depth` or `max_nodes`.
        field: &'static str,
        /// Why the value is invalid.
        detail: String,
    },
}

impl XmlError {
    /// The stable code this refusal carries.
    pub fn code(&self) -> &'static str {
        match self {
            Self::DtdDeclared { .. } => "E-XML-001",
            Self::EntityDeclaration { .. } => "E-XML-002",
            Self::UndeclaredEntity { .. } => "E-XML-003",
            Self::MalformedMarkup { .. } => "E-XML-004",
            Self::MismatchedEndTag { .. } => "E-XML-005",
            Self::MultipleRoots { .. } => "E-XML-006",
            Self::UnsupportedDocumentNode { .. } => "E-XML-007",
            Self::UnsupportedDeclaration { .. } => "E-XML-008",
            Self::DuplicateAttribute { .. } => "E-XML-009",
            Self::InvalidCharacterReference { .. } => "E-XML-010",
            Self::DepthLimitExceeded { .. } => "E-XML-011",
            Self::NodeLimitExceeded { .. } => "E-XML-012",
            Self::UndeclaredPrefix { .. } => "E-XML-013",
            Self::InvalidOptions { .. } => "E-XML-014",
        }
    }

    /// This crate is read-only: every refusal is a parse refusal, so the
    /// question the Python layer asks to pick its catalog key is always true.
    pub fn is_parse_error(&self) -> bool {
        true
    }
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The stable code travels with the message: the Python binding puts
        // Display in the localized detail, and scripts match on the code.
        write!(f, "{}: ", self.code())?;
        match self {
            Self::DtdDeclared { line, column } => {
                write!(f, "DTD declaration refused at line {line}, column {column}")
            }
            Self::EntityDeclaration { line, column } => {
                write!(
                    f,
                    "entity declaration refused at line {line}, column {column}"
                )
            }
            Self::UndeclaredEntity { name, line, column } => write!(
                f,
                "undeclared entity &{name}; at line {line}, column {column}"
            ),
            Self::MalformedMarkup {
                line,
                column,
                detail,
            } => write!(f, "malformed XML at line {line}, column {column}: {detail}"),
            Self::MismatchedEndTag {
                expected,
                found,
                line,
                column,
            } => write!(
                f,
                "mismatched end tag at line {line}, column {column}: expected </{expected}>, found </{found}>"
            ),
            Self::MultipleRoots { line, column } => {
                write!(f, "multiple root elements at line {line}, column {column}")
            }
            Self::UnsupportedDocumentNode { kind, line, column } => write!(
                f,
                "document-level {kind} is unsupported at line {line}, column {column}"
            ),
            Self::UnsupportedDeclaration { detail } => {
                write!(f, "unsupported XML declaration: {detail}")
            }
            Self::DuplicateAttribute { name, line, column } => {
                write!(
                    f,
                    "duplicate attribute {name} at line {line}, column {column}"
                )
            }
            Self::InvalidCharacterReference { text, line, column } => write!(
                f,
                "invalid character reference {text} at line {line}, column {column}"
            ),
            Self::DepthLimitExceeded { limit } => {
                write!(f, "XML depth limit {limit} exceeded")
            }
            Self::NodeLimitExceeded { limit } => {
                write!(f, "XML node limit {limit} exceeded")
            }
            Self::UndeclaredPrefix {
                prefix,
                line,
                column,
            } => write!(
                f,
                "undeclared namespace prefix {prefix:?} at line {line}, column {column}"
            ),
            Self::InvalidOptions { field, detail } => {
                write!(f, "invalid XML option {field}: {detail}")
            }
        }
    }
}

impl core::error::Error for XmlError {}

#[cfg(test)]
mod tests {
    use super::XmlError;
    use alloc::string::ToString;

    #[test]
    fn every_refusal_carries_its_own_code() {
        let cases: [(XmlError, &str); 14] = [
            (XmlError::DtdDeclared { line: 1, column: 1 }, "E-XML-001"),
            (
                XmlError::EntityDeclaration { line: 1, column: 1 },
                "E-XML-002",
            ),
            (
                XmlError::UndeclaredEntity {
                    name: "x".to_string(),
                    line: 1,
                    column: 1,
                },
                "E-XML-003",
            ),
            (
                XmlError::MalformedMarkup {
                    line: 1,
                    column: 1,
                    detail: "x".to_string(),
                },
                "E-XML-004",
            ),
            (
                XmlError::MismatchedEndTag {
                    expected: "a".to_string(),
                    found: "b".to_string(),
                    line: 1,
                    column: 1,
                },
                "E-XML-005",
            ),
            (XmlError::MultipleRoots { line: 1, column: 1 }, "E-XML-006"),
            (
                XmlError::UnsupportedDocumentNode {
                    kind: "comment".to_string(),
                    line: 1,
                    column: 1,
                },
                "E-XML-007",
            ),
            (
                XmlError::UnsupportedDeclaration {
                    detail: "x".to_string(),
                },
                "E-XML-008",
            ),
            (
                XmlError::DuplicateAttribute {
                    name: "x".to_string(),
                    line: 1,
                    column: 1,
                },
                "E-XML-009",
            ),
            (
                XmlError::InvalidCharacterReference {
                    text: "&#xD800;".to_string(),
                    line: 1,
                    column: 1,
                },
                "E-XML-010",
            ),
            (XmlError::DepthLimitExceeded { limit: 2 }, "E-XML-011"),
            (XmlError::NodeLimitExceeded { limit: 2 }, "E-XML-012"),
            (
                XmlError::UndeclaredPrefix {
                    prefix: "p".to_string(),
                    line: 1,
                    column: 1,
                },
                "E-XML-013",
            ),
            (
                XmlError::InvalidOptions {
                    field: "max_depth",
                    detail: "x".to_string(),
                },
                "E-XML-014",
            ),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code);
        }
    }

    #[test]
    fn a_read_only_engine_has_only_parse_refusals() {
        assert!(XmlError::DepthLimitExceeded { limit: 1 }.is_parse_error());
        assert!(XmlError::DtdDeclared { line: 1, column: 1 }.is_parse_error());
    }

    #[test]
    fn a_refusal_names_its_position() {
        let error = XmlError::MalformedMarkup {
            line: 7,
            column: 3,
            detail: "bad name".to_string(),
        };
        let text = error.to_string();
        assert!(text.contains('7'), "{text}");
        assert!(text.contains('3'), "{text}");
    }
}
