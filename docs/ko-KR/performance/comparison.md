# ---
---

title: Comparison with Other Libraries
lang: ko-KR

# 비교 with Other Libraries

pyyaml-rs compared against the two most popular Python YAML libraries.

## 성능 비교

### Parse Speed (Large YAML, ~2 KB)

| Library | Time | Speedup |
|---------|------|---------|
| **pyyaml-rs** | **0.07 ms** | — |
| PyYAML | 1.83 ms | 26× slower |
| ruamel.yaml | 4.26 ms | 61× slower |

### Serialize Speed (Large YAML, ~2 KB)

| Library | Time | Speedup |
|---------|------|---------|
| **pyyaml-rs** | **0.08 ms** | — |
| PyYAML | 2.96 ms | 37× slower |
| ruamel.yaml | 6.74 ms | 84× slower |

### Round-Trip Speed (Large YAML, ~2 KB)

| Library | Time | Speedup |
|---------|------|---------|
| **pyyaml-rs** | **0.08 ms** | — |
| PyYAML | 2.98 ms | 37× slower |
| ruamel.yaml | 6.79 ms | 85× slower |

## Feature 비교

| Feature | pyyaml-rs | PyYAML | ruamel.yaml |
|---------|-----------|--------|-------------|
| **YAML 1.2 compliance** | ✅ | ✅ | ✅ |
| **Comments (standalone)** | ✅ | ❌ | ✅ |
| **Comments (inline)** | ✅ | ❌ | ✅ |
| **Anchors/aliases** | ✅ | ❌ | ✅ |
| **Tags (explicit)** | ✅ | ❌ | ✅ |
| **Block scalars** | ✅ | ✅ | ✅ |
| **Flow collections** | ✅ | ✅ | ✅ |
| **Merge keys (<<)** | ✅ | ❌ | ✅ |
| **Complex keys** | ✅ | ✅ | ✅ |
| **Round-trip preservation** | ✅ | ❌ | ✅ |
| **Python bindings** | ✅ | ✅ | ✅ |
| **ABI3 (py3.9+)** | ✅ | ❌ | ❌ |
| **Type stubs (.pyi)** | ✅ | ✅ | ❌ |
| **i18n error messages** | ✅ | ❌ | ❌ |
| **Rust backend** | ✅ | ❌ | ❌ |
| **성능** | 🚀 Fastest | 🐌 Slow | 🐌 Slow |

## Summary

### Choose pyyaml-rs when

- **성능 matters** — 25–40× faster than PyYAML
- **Round-trip preservation is critical** — preserves comments, anchors, tags
- **You want PyYAML compatibility** — drop-in replacement API
- **You need type hints** — full `.pyi` stubs
- **You want a single wheel** — ABI3 works across Python 3.9–3.13

### Choose PyYAML when

- You're already using it and don't need round-trip preservation
- You need maximum compatibility with existing code
- 성능 is not a concern

### Choose ruamel.yaml when

- You need the most feature-complete YAML parser
- You're doing complex YAML manipulation
- 성능 is not a concern (it's the slowest option)

## Migration Path

```python
# Step 1: Install
pip install pyyaml-rs

# Step 2: Replace import
# Before:
import yaml

# After:
import pyyaml_rs as yaml

# Step 3: Test
# Run your existing tests to verify compatibility
```

Most code will work without changes. The main differences:

1. Round-trip output will preserve comments and formatting
2. Error messages are more detailed and can be localized
3. 성능 will be significantly better
