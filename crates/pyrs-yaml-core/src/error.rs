//! Structured error types shared across the core engine.
//!
//! All errors implement `std::error::Error` so downstream consumers can use
//! `Box<dyn Error>` or `?`-chain across crate boundaries.

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

/// YAML parse failures, carrying source position where available.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ParseError {
    /// Generic syntax error with a source position (0-indexed line/column).
    #[error("YAML parse error: {message}")]
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
}

/// YAML serialization failures.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SerializeError {
    /// Recursion depth exceeded the configured maximum.
    #[error(transparent)]
    MaxDepthExceeded(DepthError),
    /// Internal invariant violation / unexpected state.
    #[error("internal-error: {0}")]
    Internal(&'static str),
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
