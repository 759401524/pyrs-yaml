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

<div class="grid cards" markdown>

- :material-lightning-bolt: **Blazing Fast** — 21–43× faster parsing, 55–177× faster serialization than PyYAML, powered by a Rust zero-copy backend
- :material-sync: **Perfect Round-Trip** — preserves comments, anchors, tags, chomping, scalar styles, and flow/block formatting
- :material-pencil: **In-Place Editing** — edit parsed documents via JSONPath-style paths (`doc.set("$.a.b", v)`) or the `Node` tree API, without losing formatting
- :material-check-decagram: **YAML 1.2 Compliant** — powered by granit-parser (99.75% YAML Test Suite pass rate, 405/406)
- :material-swap-horizontal: **PyYAML Compatible** — drop-in replacement with `safe_load` / `safe_dump` API
- :material-language-python: **Type Hints** — PEP 561 compliant with full `.pyi` stubs
- :material-package-variant-closed: **ABI3 Wheel** — single wheel works across Python 3.8–3.15
- :material-translate: **i18n Errors** — `set_language("zh-CN")` for bilingual error reporting
- :material-numeric: **NumPy ndarray** — serialize `numpy.ndarray` of any dimension with zero-copy Rust dispatch

</div>

### Quick Start

```bash title="Install"
pip install pyrs-yaml
```

```python title="Quick start"
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

[Get Started :material-arrow-right:](quick-start.md){ .md-button .md-button--primary }
[Browse API Reference :material-code-braces:](api/reference.md){ .md-button }
[View on GitHub :fontawesome-brands-github:](https://github.com/759401524/pyrs-yaml){ .md-button }
