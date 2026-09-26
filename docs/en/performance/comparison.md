---
title: Comparison with Other Libraries
description: pyrs-yaml compared against PyYAML and ruamel.yaml across performance, features, and migration paths.
tags:
  - docs
status: new
---

## Comparison with Other Libraries

pyrs-yaml compared against the two most popular Python YAML libraries.

### Performance Comparison

#### Parse Speed (Large YAML, ~2 KB)

| Library | Time | Speedup |
|---------|------|---------|
| **pyrs-yaml** | **1.5 ms** | — |
| PyYAML | 57.7 ms | 38× slower |
| ruamel.yaml | 127.9 ms | 85× slower |

#### Serialize Speed (Large YAML, ~2 KB)

| Library | Time | Speedup |
|---------|------|---------|
| **pyrs-yaml** | **0.17 ms** | — |
| PyYAML | 30.2 ms | 177× slower |
| ruamel.yaml | 63.1 ms | 371× slower |

#### Round-Trip Speed (Large YAML, ~2 KB)

| Library | Time | Speedup |
|---------|------|---------|
| **pyrs-yaml** | **1.6 ms** | — |
| PyYAML | 87.9 ms[^1] | 55× slower |
| ruamel.yaml | 191.0 ms[^1] | 119× slower |

[^1]: PyYAML/ruamel round-trip times are estimated as parse + serialize from the same benchmark run.

### Trade-off vs. native-object Rust parsers

pyrs-yaml is the fastest YAML library that also keeps a **round-trip AST**
(comments, anchors, tags, and source spans survive a load→edit→dump). When
compared against Rust parsers that deserialize straight to plain Python
objects and do **not** offer round-trip editing — `yaml-rs` in our
cross-library benchmark — the picture is split:

- **Serialize:** pyrs-yaml wins by ~2–3× (see the tables above).
- **Eager parse into a `YamlDocument`:** pyrs-yaml is ~2× slower than
  `yaml-rs`, because it builds the richer round-trip structure `yaml-rs`
  never produces. This is a deliberate design trade — the extra work is what
  buys comment/anchor/order preservation, not a regression to optimize away.

Choose a native-only parser like `yaml-rs` when you only ever *read* YAML,
need the last microsecond on parse, and never need to write changes back
preserving the original formatting. Choose pyrs-yaml when round-trip
fidelity matters — and note that even then it stays 20–40× ahead of the
pure-Python alternatives above.

### Feature Comparison

| Feature | pyrs-yaml | PyYAML | ruamel.yaml |
|---------|-----------|--------|-------------|
| **YAML 1.2 compliance** | :material-check: | :material-check: | :material-check: |
| **Comments (standalone)** | :material-check: | :material-close: | :material-check: |
| **Comments (inline)** | :material-check: | :material-close: | :material-check: |
| **Anchors/aliases** | :material-check: | :material-close: | :material-check: |
| **Tags (explicit)** | :material-check: | :material-close: | :material-check: |
| **Block scalars** | :material-check: | :material-check: | :material-check: |
| **Flow collections** | :material-check: | :material-check: | :material-check: |
| **Merge keys (<<)** | :material-check: | :material-close: | :material-check: |
| **Complex keys** | :material-check: | :material-check: | :material-check: |
| **Round-trip preservation** | :material-check: | :material-close: | :material-check: |
| **Python bindings** | :material-check: | :material-check: | :material-check: |
| **ABI3 (py3.8+)** | :material-check: | :material-close: | :material-close: |
| **Type stubs (.pyi)** | :material-check: | :material-check: | :material-close: |
| **i18n error messages** | :material-check: | :material-close: | :material-close: |
| **Rust backend** | :material-check: | :material-close: | :material-close: |
| **Performance** | :material-rocket-launch: Fastest | :material-snail: Slow | :material-snail: Slow |

### Summary

#### Choose pyrs-yaml when

!!! success "Recommended for most use cases"
    - **Performance matters** — 21–43× faster parsing and 55–177× faster serialization than PyYAML
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
