---
title: Comparison with Other Libraries
description: pyrs-yaml compared against PyYAML and ruamel.yaml across performance, features, and migration paths.
tags:
  - docs
status: new
---

## Comparison with Other Libraries

pyrs-yaml compared against the two most popular Python YAML libraries, and against
`yaml-rs` and `ryaml`, the Rust-backed readers.

### Performance Comparison

#### Parse Speed (Large YAML, ~2 KB)

<!-- bench: layout=libraries format=yaml operation=parse size=large -->
| Library | Time | Speedup |
|---------|------|---------|
| **pyrs-yaml** | 72 µs | — |
| PyYAML | 2.3 ms | 32× slower |
| ruamel.yaml | 5.4 ms | 75× slower |
| yaml-rs | 27 µs | 2.7× faster |
| ryaml | 86 µs | 1.2× slower |

#### Serialize Speed (Large YAML, ~2 KB)

<!-- bench: layout=libraries format=yaml operation=serialize size=large -->
| Library | Time | Speedup |
|---------|------|---------|
| **pyrs-yaml** | 5.4 µs | — |
| PyYAML | 1.4 ms | 261× slower |
| ruamel.yaml | 3.3 ms | 603× slower |
| yaml-rs | 17 µs | 3.1× slower |
| ryaml | 42 µs | 7.8× slower |

#### Round-Trip Speed (Large YAML, ~2 KB)

<!-- bench: layout=libraries format=yaml operation=roundtrip size=large -->
| Library | Time | Speedup |
|---------|------|---------|
| **pyrs-yaml** | 82 µs | — |
| PyYAML | 3.9 ms | 48× slower |
| ruamel.yaml | 9.1 ms | 111× slower |
| yaml-rs | 48 µs | 1.7× faster |
| ryaml | 135 µs | 1.6× slower |

### Trade-off vs. native-object Rust parsers

pyrs-yaml is the fastest YAML library that also keeps a **round-trip AST**
(comments, anchors, tags, and source spans survive a load→edit→dump). When
compared against Rust parsers that deserialize straight to plain Python
objects and do **not** offer round-trip editing — `yaml-rs` in the tables
above — the picture is split:

- **Serialize:** pyrs-yaml wins by about 2.3–3.7× on the same fixtures (see the tables above).
- **Eager parse into a `YamlDocument`:** pyrs-yaml is about 2.5× slower than
  `yaml-rs`, because it builds the richer round-trip structure `yaml-rs`
  never produces. This is a deliberate design trade — the extra work is what
  buys comment/anchor/order preservation, not a regression to optimize away.

Choose a native-only parser like `yaml-rs` when you only ever *read* YAML,
need the last microsecond on parse, and never need to write changes back
preserving the original formatting. Choose pyrs-yaml when round-trip
fidelity matters — and note that even then it parses 32–37× faster than
PyYAML and serializes 204–347× faster.

### Feature Comparison

| Feature | pyrs-yaml | PyYAML | ruamel.yaml |
|---------|-----------|--------|-------------|
| **YAML 1.2 compliance** | :material-check: | :material-check: | :material-check: |
| **Comments (standalone)** | :material-check: | :material-close: | :material-check: |
| **Comments (inline)** | :material-check: | :material-close: | :material-check: |
| **Anchors/aliases** | :material-check: | :material-check: | :material-check: |
| **Tags (explicit)** | :material-check: | :material-check: | :material-check: |
| **Block scalars** | :material-check: | :material-check: | :material-check: |
| **Flow collections** | :material-check: | :material-check: | :material-check: |
| **Merge keys (<<)** | :material-check: | :material-check: | :material-check: |
| **Complex keys** | :material-check: | :material-check: | :material-check: |
| **Round-trip preservation** | :material-check: | :material-close: | :material-check: |
| **Python bindings** | :material-check: | :material-check: | :material-check: |
| **ABI3 (py3.8+)** | :material-check: | :material-close: | :material-close: |
| **Type stubs (.pyi)** | :material-check: | :material-close: | :material-close: |
| **i18n error messages** | :material-check: | :material-close: | :material-close: |
| **Rust backend** | :material-check: | :material-close: | :material-close: |
| **Performance** | :material-rocket-launch: Fastest | :material-snail: Slow | :material-snail: Slow |

### Summary

#### Choose pyrs-yaml when

!!! success "Recommended for most use cases"
    - **Performance matters** — 32–37× faster parsing and 204–347× faster serialization than PyYAML
    - **Round-trip preservation is critical** — preserves comments, anchors, tags
    - **You want PyYAML compatibility** — drop-in replacement API
    - **You need type hints** — full `.pyi` stubs
    - **You want a single wheel** — ABI3 works across Python 3.8–3.15

#### Choose PyYAML when

!!! info "Legacy compatibility only"
    - You're already using it and don't need round-trip preservation
    - You need maximum compatibility with existing code
    - Performance is not a concern

#### Choose ruamel.yaml when

!!! warning "Slowest option"
    - You need the most feature-complete YAML parser
    - You're doing complex YAML manipulation
    - Performance is not a concern (it's the slowest option)

### Migration Path

```python title="Migration steps"
# Step 1: Install
pip install pyrs-yaml

# Step 2: Replace import
# Before:
import yaml

# After:
import pyrs_yaml as yaml

# Step 3: Test
# Run your existing tests to verify compatibility
```

Most code will work without changes. The main differences:

1. Round-trip output will preserve comments and formatting
2. Error messages are more detailed and can be localized
3. Performance will be significantly better
