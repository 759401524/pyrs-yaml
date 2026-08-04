# Architecture

pyrs-yaml uses a modular architecture designed for performance and correctness.

## Overview

```text
┌─────────────────────────────────────────────────────────┐
│                     Python Layer                        │
│  ┌─────────────────────────────────────────────────────┐│
│  │               pyrs_yaml module                       ││
│  │  parse | safe_load | safe_dump | dump_file | ...    ││
│  └─────────────────────┬───────────────────────────────┘│
│                        │ PyO3 bindings                   │
├────────────────────────▼─────────────────────────────────┤
│                    Rust Layer                            │
│  ┌─────────────────────────────────────────────────────┐│
│  │  src/py/ — PyO3 bindings + Python type conversion   ││
│  │  • mod.rs — Module definition & exports              ││
│  │  • python_types.rs — Python → CustomNode conversion ││
│  │  • ndarray.rs — NumPy ndarray conversion            ││
│  │  • stream_events.rs — Stream event types            ││
│  └─────────────────────┬───────────────────────────────┘│
│                        │                                 │
│      ┌─────────────────┼──────────────────┐              │
│      ▼                 ▼                  ▼              │
│  ┌─────────┐    ┌────────────┐    ┌──────────────┐       │
│  │ ast.rs  │    │ parser/    │    │serializer.rs │       │
│  │ Custom  │◄──►│ saphyr     │    │ to_yaml()    │       │
│  │ Node    │    │ integration│    │ to_yaml_*    │       │
│  └─────────┘    └────────────┘    └───────────────┘       │
│      ▲                 ▲                                    │
│      └─────────────────┴────────────────────┘              │
│                      CustomNode                           │
└─────────────────────────────────────────────────────────┘

## Workspace Structure

The codebase is split into two crates under `crates/`:

```

crates/
├── pyrs-yaml-core/ # Pure Rust, no PyO3 dependencies
│ └── src/
│ ├── lib.rs # Re-exports all core modules
│ ├── ast.rs # CustomNode AST
│ ├── editing/ # Edit primitives (navigate, region, dirty, metadata)
│ ├── i18n.rs # Internationalization
│ ├── parser/ # YAML parser (saphyr-based)
│ ├── serializer.rs    # YAML serializer
│ └── splice.rs        # Splice-based text assembly
└── pyrs-yaml/ # PyO3 bindings layer
    └── src/
        ├── lib.rs # Re-exports core + defines #[pymodule]
        ├── py/ # PyO3 bindings
        │ ├── mod.rs # YamlDocument pyclass
        │ ├── convert.rs # CustomNode ↔ Python type conversion
        │ └── editing/ # Python-facing editing wrappers
        └── fidelity.rs # Property-based tests

```text
```

## Module Architecture

### 1. `crates/pyrs-yaml-core/src/ast.rs` — Custom AST

The **CustomNode** enum is the heart of pyrs-yaml:

- **Scalar** — with style (plain, quoted, literal, folded), comment, anchor, tag, chomping
- **Mapping** — `IndexMap` for key order preservation, flow_style flag
- **Sequence** — ordered list, flow_style flag
- **Null** — with comment, anchor, tag
- **Alias** — alias reference (name only)

**Why Custom AST?**

- Standard YAML parsers discard metadata (comments, formatting)
- Custom AST preserves everything needed for round-trip
- Extensible for future features (custom node types, metadata)

### 2. `crates/pyrs-yaml-core/src/parser/` — YAML Parser

Built on **saphyr-parser** (YAML 1.2 compliant):

- **`mod.rs`** — `AstReceiver` state machine, event-based parsing, flow style detection
- **`stream.rs`** — Streaming event parser (line-by-line YAML events)
- **`yaml/comment.rs`** — Comment and anchor extraction from raw text
- **`yaml/merge.rs`** — Merge key (`<<`) resolution
- **`yaml/scalar.rs`** — Scalar style detection, unescaping, chomping
- **`yaml/schema.rs`** — YAML schema resolution (core, JSON, failsafe, YAML 1.1)
- **`yaml/types.rs`** — YAML 1.2 type resolution (null, bool, int, float)

**Key Design Decisions:**

- Event-based API (not token-based) — better for structured output
- Two-pass parsing: first extract comments/anchors, then parse events
- Merge key resolution happens after parsing (configurable)

### 3. `crates/pyrs-yaml-core/src/serializer.rs` — YAML Serializer

Custom serializer that reconstructs YAML from the AST:

- **`to_yaml()`** — Serialize with default options
- **`to_yaml_with_options()`** — Custom indent, markers, sorting
- **`write_anchor_tag()`** — Helper for anchor/tag output
- **`write_inline_comment()`** — Helper for inline comment output

**Key Design Decisions:**

- No third-party emitter — full control over output format
- Indent-level state management for nested structures
- Chomping indicator handling for block scalars

### 5. `crates/pyrs-yaml/src/py/` — PyO3 Bindings

The Python-facing layer that exposes Rust functionality to Python:

- **`mod.rs`** — `YamlDocument` pyclass, `#[pymodule]` entry point
- **`convert.rs`** — Python ↔ CustomNode conversion and error formatting
- **`python_types.rs`** — Python → CustomNode type conversion
- **`ndarray.rs`** — NumPy ndarray serialization (optional, `numpy` feature)
- **`stream_events.rs`** — Stream event types for Python
- **`streaming.rs`** — Streaming parse (constant memory)
- **`writing.rs`** — Streaming write (constant memory)
- **`tag_registry.rs`** — Python tag handler registration
- **`editing/`** — Python-facing editing wrappers (`segment_py.rs` + re-exports from core)

Pure Rust edit primitives used by the Python-facing editing API:

- **`navigate.rs`** — AST path navigation (`navigate`, `navigate_mut`, `key_eq`, `mapping_key_index`, `normalize_index`, `parse_path_segments`)
- **`region.rs`** — Edit region computation (`path_nodes`, `region_unit`, `precompute`, line helpers, `extend_delete_over_comments`)
- **`dirty.rs`** — Edit operation types (`DirtyKind`, `DirtyUnit`)
- **`metadata.rs`** — Metadata preservation (`with_metadata_from`)

### 5. `crates/pyrs-yaml/src/py/` — PyO3 Bindings

Python-facing module definitions and type conversions:

- **`mod.rs`** — Inline `#[pymodule(gil_used = false)]` with `YamlDocument` class, exception types, and all exported functions
- **`python_types.rs`** — Converts Python objects (dict, list, scalars, ndarray) to `CustomNode`
- **`ndarray.rs`** — NumPy ndarray conversion (optional, behind `numpy` feature)
- **`stream_events.rs`** — Stream event types for `parse_stream()`

**Exported Python functions (18 total):**
`parse`, `safe_load`, `safe_loads`, `safe_dump`, `safe_dumps`, `parse_file`, `dump_file`, `parse_all_docs`, `parse_stream`, `read_markdown`, `from_dict`, `from_json`, `set_language`, `get_language`, `list_languages`, `detect_language`, `negotiate_language`, `YamlDocument`

### 5. `src/lib.rs` — Module Entry

- Re-exports all modules
- Error types: `YamlParseError`, `YamlSerializeError`, `YamlTypeError`
- `create_exception!` macros for custom Python exceptions
- `rust-i18n` initialization

### 6. `src/i18n/` — Internationalization

- `src/i18n.rs` — Configuration and language negotiation
- `src/i18n/` — Locale bundles (en, zh-CN, ja-JP, ko-KR)
- Bilingual error messages with format strings

### 7. `src/integration/` — Integration Helpers

- `yaml_suite.rs` — YAML Test Suite runner for validation
- Test helpers for benchmarks and compliance checks

## Data Flow

### Parse Flow

```text
YAML String
    │
    ▼
┌─────────────────────────────────────┐
│ 1. Extract comments from raw text   │
│ 2. Extract anchors from raw text    │
│ 3. saphyr-parser → YAML events      │
│ 4. AstReceiver builds CustomNode    │
│ 5. Resolve schema types             │
│ 6. Resolve merge keys (if enabled)  │
└─────────────────────────────────────┘
    │
    ▼
CustomNode (AST)
```

### Serialize Flow

```text
CustomNode (AST)
    │
    ▼
┌─────────────────────────────────────┐
│ 1. Determine node type              │
│ 2. Write opening (anchor, tag)      │
│ 3. Write content (key: value)       │
│ 4. Write inline comment             │
│ 5. Recurse for nested nodes         │
└─────────────────────────────────────┘
    │
    ▼
YAML String
```

## Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Parse | O(n) | Single pass over YAML events |
| Serialize | O(n) | Single pass over AST |
| Round-trip | O(n) | Parse + Serialize |
| Merge resolution | O(n × m) | Where n = docs, m = merges per doc |
| Comment extraction | O(n) | Single pass over raw text |

## Dependencies

| Crate | Purpose |
|-------|---------|
| **pyo3** | Python bindings (with `experimental-inspect`, `abi3-py38`, `abi3t`) |
| **saphyr-parser** | YAML 1.2 compliant parsing |
| **indexmap** | Ordered hash map for key preservation |
| **serde_json** | JSON ↔ YAML conversion |
| **numpy** | NumPy ndarray support (optional, default enabled) |
| **rust-i18n** | Internationalized error messages |
