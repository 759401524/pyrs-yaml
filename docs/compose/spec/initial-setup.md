---
feature: initial-setup
status: delivered
updated: 2026-07-25
branch: master
commits: initial
---

# Initial Setup — pyamlium-custom YAML Library

## Report

**What was built** — A foundational Rust/PyO3 YAML library with three-module architecture (ast.rs, parser.rs, serializer.rs). The library parses YAML using yaml-rust2's Scanner for tokenization, builds a custom AST with metadata for round-trip support, and serializes back to YAML with proper formatting preservation.

**Verification** — All 40 tests pass: 10 Rust unit tests and 30 Python pytest tests covering basic parsing, mappings, sequences, special characters, block scalars, anchors/aliases, flow collections, and nested structures.

**Journey log** — 
1. Initial yaml-rust2 API mismatch: Scanner returns `Token` not `Result<Token>`, fixed by using `next_token()` method.
2. Empty flow collections (`{}`, `[]`) required adding `FlowMappingStart`/`FlowSequenceStart` handling.
3. All tests pass after fixes, project builds successfully with maturin.

## [S1] Problem
The project needs a foundational Rust/PyO3 YAML library that can parse YAML while preserving comments, formatting, and key order (round-trip support). The current repo is empty with no implementation.

## [S2] Design

### Architecture: Three-Module System
1. **`src/ast.rs`** — Custom AST with metadata for round-trip
   - `CustomNode` enum: Scalar, Mapping, Sequence, Null
   - `ScalarStyle` enum: Plain, SingleQuoted, DoubleQuoted, Literal, Folded
   - Comment attachment (line-end and standalone)
   - Anchor/alias tracking
   - `IndexMap` for mapping key order preservation

2. **`src/parser.rs`** — Token-based parser
   - Use `yaml_rust2::yaml::Scanner` for tokenization
   - State machine converting tokens to `CustomNode` AST
   - Comment capture and attachment to context nodes

3. **`src/serializer.rs`** — Custom YAML output
   - Indent-level state management
   - Scalar style preservation and escaping
   - Multi-line string base-indent calculation

### Dependencies (from AGENTS.md)
```toml
pyo3 = { version = "0.21", features = ["extension-module"] }
indexmap = { version = "2.2", features = ["serde"] }
yaml-rust2 = "0.9"
```

### Python Interface
- `parse(yaml_str: str) -> YamlDocument`
- `YamlDocument.to_yaml() -> str`
- `YamlDocument.get(key: str) -> Any`

### Coding Standards
- No `.unwrap()` or `.expect()` in business logic
- All Rust errors mapped to Python exceptions
- Release GIL during heavy computation via `py.allow_threads`

## [S3] Out of Scope
- Writing mode (create YAML from scratch)
- YAML merge keys (`<<`)
- Complex key types (list/dict as mapping keys)
- Streaming/lazy parsing
- YAML 1.2 features beyond what yaml-rust2 supports

## Tasks
- [x] T1: Project scaffolding — Cargo.toml with correct dependencies, src/ module structure, maturin config — acceptance: `cargo check` passes, `maturin develop` builds successfully (covers: S2)
- [x] T2: Custom AST implementation — src/ast.rs with CustomNode, ScalarStyle, IndexMap mappings — acceptance: unit tests pass for node construction and metadata (covers: S2; depends: T1)
- [x] T3: Token-based parser — src/parser.rs using yaml_rust2 Scanner to build AST — acceptance: parses simple YAML to AST, preserves comments in AST metadata (covers: S2; depends: T2)
- [x] T4: Custom serializer — src/serializer.rs outputting YAML from AST — acceptance: round-trip test passes for basic YAML (covers: S2; depends: T2)
- [x] T5: PyO3 Python bindings — pyamlium_custom module with parse/to_yaml/get — acceptance: pytest passes for parse round-trip (covers: S2; depends: T3, T4)
- [x] T6: Edge case tests — special chars, multi-line strings, anchors/aliases — acceptance: all pytest tests pass (covers: S2; depends: T5)
