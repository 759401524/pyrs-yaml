# ---
---

title: Coding Standards
lang: ko-KR

# 코드 표준

Follow these standards when contributing to pyyaml-rs.

## Rust

### Style

- Use `cargo fmt` before committing
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `#[allow(unused_imports)]` only when necessary (tests, feature flags)

### Error Handling

- **Never use `.unwrap()` or `.expect()`** in business logic
- Convert all Rust errors to Python exceptions
- Use `PyResult<T>` for functions that can fail
- Map specific errors to specific Python exception types

```rust
// Good
let content = std::fs::read_to_string(path)
    .map_err(|e| YamlParseError::new_err(format_i18n_error("file-read-error", ...)))?;

// Bad
let content = std::fs::read_to_string(path).unwrap();
```

### Documentation

- All public functions must have `///` doc comments
- Include `# Arguments`, `# Returns`, `# Errors`, `# Examples` sections
- Write doc comments in English (Rust convention)
- Chinese doc comments are acceptable for internal functions

```rust
/// Parse a YAML string into a CustomNode AST.
///
/// # Arguments
/// * `yaml` - YAML content string
///
/// # Returns
/// The parsed AST root node, or `Err(String)` on failure
///
/// # Errors
/// Returns `Err(String)` formatted as `"YAML parse error: line N, col M: <msg>"`
///
/// # Examples
/// ```
/// let ast = pyyaml_rs::parser::parse("key: value").unwrap();
/// ```
pub fn parse(yaml: &str) -> Result<CustomNode, String> {
```

### GIL Management

- Release GIL during heavy computation using `py.detach()` or `py.allow_threads()`
- Never hold GIL during file I/O or parsing

```rust
// Good
let ast = py.detach(|| {
    parser::parse_with_options(&yaml_str, resolve_merges)
        .map_err(|e| YamlParseError::new_err(...))?
})?;

// Bad — holds GIL during parsing
let ast = parser::parse_with_options(&yaml_str, resolve_merges)?;
```

### Clippy

Run `cargo clippy -- -D warnings` — treat all warnings as errors.

## Python

### Style

- Follow [PEP 8](https://peps.python.org/pep-0008/)
- Use type hints everywhere
- Docstrings in Google style

```python
def parse(yaml: str, resolve_merges: bool = True) -> YamlDocument:
    """Parse a YAML string into a YamlDocument.

    Args:
        yaml: A string containing YAML content
        resolve_merges: Whether to resolve merge keys (default: True)

    Returns:
        A YamlDocument containing the parsed YAML

    Raises:
        YamlParseError: If the YAML is invalid
    """
```

### Testing

- Write tests before code (TDD)
- Use `pytest` with fixtures where appropriate
- Test edge cases: empty input, special characters, large documents
- Include round-trip assertions

## Git

- Commit messages in imperative mood: "Add feature X", not "Added feature X"
- One logical change per commit
- Run `cargo test` and `pytest tests/` before committing

## Documentation

- Update docs when changing behavior
- Use code examples that can be copy-pasted and run
- Keep examples concise but complete
