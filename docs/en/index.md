---
title: pyrs-yaml
description: Overview of pyrs-yaml, a high-performance Python YAML library with perfect round-trip support, built with Rust and PyO3.
tags:
  - docs
status: new
---

## pyrs-yaml

**High-performance Python YAML library with perfect round-trip support, built with Rust and PyO3.**

---

### Why pyrs-yaml?

Most Python YAML libraries sacrifice either performance or fidelity. pyrs-yaml delivers both:

- **PyYAML** (Python) — slow, **loses comments/anchors/tags** on round-trip
- **ruamel.yaml** (Python) — preserves formatting, but **48–100× slower parsing and 123–371× slower serialization** than pyrs-yaml
- **pyrs-yaml** (Rust) — **21–43× faster parsing and 55–177× faster serialization than PyYAML** while preserving everything

### Key Features

- **YAML 1.2 compliant** — powered by granit-parser (99.75% YAML Test Suite pass rate, 405/406)
- **Perfect Round-Trip** — preserves comments, anchors, tags, chomping, scalar styles, and flow/block formatting
- **In-Place Editing** — edit parsed documents via JSONPath-style paths (`doc.set("$.a.b", v)`) or the `Node` tree API, without losing formatting
- **21–43× faster parsing and 55–177× faster serialization** than PyYAML — Rust backend with zero-copy parsing
- **Custom AST** — extensible AST for advanced YAML manipulation and custom formatting
- **PyYAML compatible** — drop-in replacement with `safe_load` / `safe_dump` API
- **Type hints** — PEP 561 compliant with full `.pyi` stubs
- **ABI3** — single wheel works across Python 3.8–3.15
- **i18n error messages** — `set_language("zh-CN")` for bilingual error reporting
- **NumPy ndarray support** — serialize `numpy.ndarray` of any dimension to YAML with zero-copy Rust dispatch

### Quick Start

```bash
pip install pyrs-yaml
```

```python
import pyrs_yaml

# Parse YAML
doc = pyrs_yaml.parse("key: value")
print(doc.to_yaml())  # key: value\n

# PyYAML compatible API
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# Round-trip preserves comments
original = "# Comment\nkey: value  # inline\n"
doc = pyrs_yaml.parse(original)
assert doc.to_yaml() == original
```

### Performance vs PyYAML

| Operation | pyrs-yaml | PyYAML | Speedup |
|-----------|-----------|--------|---------|
| Parse (small) | 0.18 ms | 3.8 ms | **21×** |
| Parse (medium) | 0.56 ms | 24.2 ms | **43×** |
| Parse (large) | 1.5 ms | 57.7 ms | **38×** |
| Serialize (small) | 0.04 ms | 2.2 ms | **55×** |
| Serialize (medium) | 0.08 ms | 12.6 ms | **159×** |
| Serialize (large) | 0.17 ms | 30.2 ms | **177×** |

---

**[Get Started →](quick-start.md)**
**[Browse API Reference →](api/reference.md)**
**[View on GitHub →](https://github.com/759401524/pyrs-yaml)**
