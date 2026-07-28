---

title: Architecture
lang: zh-CN

## 架构

pyyaml-rs uses a modular architecture designed for performance and correctness.

### 概述

```text
┌─────────────────────────────────────────────────────────┐
│                     Python Layer                        │
│  ┌─────────────────────────────────────────────────────┐│
│  │               pyyaml_rs module                       ││
│  │  parse() | safe_load() | dump_file() | ...          ││
│  └─────────────────────┬───────────────────────────────┘│
│                        │ PyO3 bindings                   │
├────────────────────────▼─────────────────────────────────┤
│                    Rust Layer                            │
│  ┌─────────────────────────────────────────────────────┐│
│  │  lib.rs — PyO3 module (inline pymodule)            ││
│  │  • YamlDocument class                               ││
│  │  • Exception types (YamlParseError, etc.)           ││
│  │  • Function wrappers                                ││
│  └─────────────────────┬───────────────────────────────┘│
│                        │                                 │
│      ┌─────────────────┼─────────────────┐              │
│      ▼                 ▼                 ▼              │
│  ┌─────────┐    ┌────────────┐    ┌────────────┐        │
│  │ ast.rs  │    │ parser/    │    │serializer  │        │
│  │ Custom  │◄──►│ saphyr     │    │ to_yaml()  │        │
│  │ Node    │    │ integration│    │ to_yaml_*  │        │
│  └─────────┘    └────────────┘    └────────────┘        │
│      ▲                 ▲                                    │
│      └─────────────────┴────────────────────┘              │
│                      CustomNode                           │
└─────────────────────────────────────────────────────────┘
```

### Module 架构

#### 1. `src/ast.rs` — Custom AST

The **CustomNode** enum is the heart of pyyaml-rs:

- **Scalar** — with style (plain, quoted, literal, folded), comment, anchor, tag, chomping
- **Mapping** — `IndexMap` for key order preservation, flow_style flag
- **Sequence** — ordered list, flow_style flag
- **Null** — with comment, anchor, tag
- **Alias** — alias reference (name only)

**Why Custom AST?**

- Standard YAML parsers discard metadata (comments, formatting)
- Custom AST preserves everything needed for round-trip
- Extensible for future features (custom node types, metadata)

#### 2. `src/parser/` — YAML Parser

Built on **saphyr-parser** (YAML 1.2 compliant):

- **`mod.rs`** — `AstReceiver` state machine, event-based parsing
- **`yaml/comment.rs`** — Comment extraction from raw text
- **`yaml/merge.rs`** — Merge key (`<<`) resolution
- **`yaml/scalar.rs`** — Scalar style detection, unescaping, chomping
- **`yaml/types.rs`** — YAML 1.2 type resolution (null, bool, int, float)

**Key Design Decisions:**

- Event-based API (not token-based) — better for structured output
- Two-pass parsing: first extract comments/anchors, then parse events
- Merge key resolution happens after parsing (configurable)

#### 3. `src/serializer.rs` — YAML Serializer

Custom serializer that reconstructs YAML from the AST:

- **`to_yaml()`** — Serialize with default options
- **`to_yaml_with_options()`** — Custom indent, markers, sorting
- **`write_anchor_tag()`** — Helper for anchor/tag output
- **`write_inline_comment()`** — Helper for inline comment output

**Key Design Decisions:**

- No third-party emitter — full control over output format
- Indent-level state management for nested structures
- Chomping indicator handling for block scalars

#### 4. `src/lib.rs` — PyO3 Module

Inline `#[pymodule] mod pyyaml_rs` with:

- **`YamlDocument`** — `#[pyclass]` wrapper around `CustomNode`
- **异常** — `create_exception!` macros for custom errors
- **Functions** — `parse`, `safe_load`, `dump_file`, etc.
- **i18n** — `rust-i18n` integration for bilingual errors

### Data Flow

#### Parse Flow

```text
YAML String
    │
    ▼
┌─────────────────────────────────────┐
│ 1. Extract comments from raw text   │
│ 2. Extract anchors from raw text    │
│ 3. saphyr-parser → YAML events      │
│ 4. AstReceiver builds CustomNode    │
│ 5. Resolve merge keys (if enabled)  │
└─────────────────────────────────────┘
    │
    ▼
CustomNode (AST)
```

#### Serialize Flow

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

### 性能 Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Parse | O(n) | Single pass over YAML events |
| Serialize | O(n) | Single pass over AST |
| Round-trip | O(n) | Parse + Serialize |
| Merge resolution | O(n × m) | Where n = docs, m = merges per doc |
| Comment extraction | O(n) | Single pass over raw text |

### Dependencies

| Crate | Purpose |
|-------|---------|
| **pyo3** | Python bindings (with `experimental-inspect`) |
| **saphyr-parser** | YAML 1.2 compliant parsing |
| **indexmap** | Ordered hash map for key preservation |
| **serde_json** | JSON ↔ YAML conversion |
| **rust-i18n** | Internationalized error messages |
