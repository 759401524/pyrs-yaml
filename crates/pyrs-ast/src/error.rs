//! Structured error types shared across the core engine.
//!
//! All errors implement `core::error::Error` (the `no_std` trait) so downstream
//! consumers can use `Box<dyn Error>` or `?`-chain across crate boundaries.
//! With the `std` feature enabled they additionally impl `std::error::Error`.

use alloc::string::String;

/// Recursion depth limit exceeded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("max depth exceeded (max={0})")]
pub struct DepthError(pub usize);

/// Path-navigation failures within an AST.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NavigateError {
    /// A key or index was not found on the target container.
    #[error("missing-path:{0}")]
    Missing(String),
    /// Navigation tried to descend into a scalar node.
    #[error("cannot-descend-into-scalar:{0}")]
    CannotDescend(String),
    /// The target node is not a mapping/sequence (cannot host children).
    #[error("create-needs-mapping")]
    NotContainer,
}

/// Parse failures from any reader in the family, carrying source position where available.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ParseError {
    /// Generic syntax error with a source position (0-indexed line/column).
    ///
    /// `message` is the whole refusal text. The YAML, JSON, TOML and schema readers all build
    /// this variant, so a format label added here would name the wrong format for three of
    /// them; each reader puts its own label in the message, or its door adds one.
    #[error("{message}")]
    Syntax {
        /// Human-readable error message.
        message: String,
        /// Line number (0-indexed) where the error occurred.
        line: usize,
        /// Column number (0-indexed) where the error occurred.
        col: usize,
    },
    /// Duplicate mapping key (when `allow_duplicate_keys` is false).
    #[error("duplicate key: {0}")]
    DuplicateKey(String),
    /// Recursion depth exceeded the configured maximum.
    #[error(transparent)]
    MaxDepthExceeded(DepthError),
    /// The input carries a second document, but the caller asked for one.
    ///
    /// Not a syntax defect: each document here is well-formed, so what the call got wrong is
    /// its arity. Deliberately not folded into [`ParseError::Syntax`], which means "this text
    /// does not parse" - a reader who is told their valid input is malformed goes looking in
    /// the wrong place. `line` and `col` locate the *second* document's start marker; naming
    /// the routes that do accept a stream belongs to the layer above, which knows the caller's
    /// vocabulary.
    #[error("YAML parse error: expected a single document, found a second one")]
    SingleDocumentExpected {
        /// Line of the second document's start marker (0-indexed).
        line: usize,
        /// Column of the second document's start marker (0-indexed).
        col: usize,
    },
}

/// Serialization failures from any writer in the family.
///
/// The variants carry no format label; the binding adds each writer's own
/// `json-`/`toml-`/`yaml-serialize-error` key before rendering this error.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SerializeError {
    /// Recursion depth exceeded the configured maximum.
    #[error(transparent)]
    MaxDepthExceeded(DepthError),
    /// Internal invariant violation / unexpected state.
    #[error("internal-error: {0}")]
    Internal(&'static str),
    /// The target format has no representation for this value or shape, so the writer refused.
    ///
    /// Separate from [`SerializeError::Internal`] because the caller did nothing wrong: a document
    /// holding an infinity, an alias, or a non-scalar object key is a valid document that JSON or TOML
    /// simply cannot spell. The `&'static str` is a stable reason key (`json-cannot-represent-non-finite`),
    /// greppable out of a user's exception message - and it must not be phrased as "internal error", which
    /// tells the reader the library broke rather than that the format has no room for the value.
    #[error("{0}")]
    UnsupportedValue(&'static str),
}

/// Edit-path parsing failures.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PathError {
    /// `*` wildcard or deep-scan selector is unsupported.
    #[error("wildcard-or-deep-scan")]
    WildcardOrDeepScan,
    /// Malformed `[index]` segment.
    #[error("invalid-index:{0}")]
    InvalidIndex(String),
    /// Path could not be recognized.
    #[error("invalid-path")]
    InvalidPath,
}

/// YAML document editing failures.
///
/// `Display` intentionally preserves the historical i18n-key style strings so
/// Python-facing error messages are unchanged.
#[derive(Debug, thiserror::Error)]
pub enum EditError {
    /// Navigation failure (from [`NavigateError`]).
    #[error(transparent)]
    Navigate(#[from] NavigateError),
    /// Editing an alias node is not allowed.
    #[error("cannot-edit-alias")]
    CannotEditAlias,
    /// Target is not a sequence.
    #[error("not-a-sequence")]
    NotASequence,
    /// Index out of range for the edit.
    #[error("index-out-of-range-edit")]
    IndexOutOfRange,
    /// Missing key on a mapping parent.
    #[error("missing-path")]
    MissingKey,
    /// Renaming the root node is not allowed.
    #[error("cannot-rename-root")]
    CannotRenameRoot,
    /// Renaming a complex (non-scalar) key is not allowed.
    #[error("cannot-rename-complex-key")]
    CannotRenameComplexKey,
    /// Target key already exists.
    #[error("rename-key-exists")]
    RenameKeyExists,
    /// Generic edit failure.
    #[error("edit-error")]
    Generic,
    /// Internal invariant violation / unexpected state.
    #[error("internal-error: {0}")]
    Internal(&'static str),
    /// Serialization failed while producing edit text.
    #[error(transparent)]
    Serialize(#[from] SerializeError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn syntax_display_adds_no_format_label() {
        let error = ParseError::Syntax {
            message: String::from("expected a JSON value at line 1 column 10"),
            line: 0,
            col: 0,
        };
        assert_eq!(
            error.to_string(),
            "expected a JSON value at line 1 column 10"
        );
    }

    #[test]
    fn a_message_naming_its_own_format_is_not_relabelled() {
        let error = ParseError::Syntax {
            message: String::from("TOML parse error: unterminated array at line 2 column 1"),
            line: 1,
            col: 0,
        };
        assert_eq!(
            error.to_string(),
            "TOML parse error: unterminated array at line 2 column 1"
        );
    }
}
