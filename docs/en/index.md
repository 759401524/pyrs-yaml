# pyrs-yaml

**High-performance Python YAML library with perfect round-trip support, built with Rust and PyO3.**

---

## Why pyrs-yaml?

Most Python YAML libraries sacrifice either performance or fidelity. pyrs-yaml delivers both:

- **PyYAML** (Python) — slow, **loses comments/anchors/tags** on round-trip
- **ruamel.yaml** (Python) — preserves formatting, but **5–10× slower** than pyrs-yaml
- **pyrs-yaml** (Rust) — **25–40× faster than PyYAML** while preserving everything

## Key Features

- **YAML 1.2 compliant** — powered by saphyr-parser (98.1% YAML Test Suite pass rate)
- **Perfect Round-Trip** — preserves comments, anchors, tags, chomping, scalar styles, and flow/block formatting
- **In-Place Editing** — edit parsed documents via JSONPath-style paths (`doc.set("$.a.b", v)`) or the `Node` tree API, without losing formatting
- **25–40× faster** than PyYAML — Rust backend with zero-copy parsing
- **Custom AST** — extensible AST for advanced YAML manipulation and custom formatting
- **PyYAML compatible** — drop-in replacement with `safe_load` / `safe_dump` API
- **Type hints** — PEP 561 compliant with full `.pyi` stubs
- **ABI3** — single wheel works across Python 3.9–3.13
- **i18n error messages** — `set_language("zh-CN")` for bilingual error reporting
- **NumPy ndarray support** — serialize `numpy.ndarray` of any dimension to YAML with zero-copy Rust dispatch

## Quick Start

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

## Performance vs PyYAML

| Operation | pyrs-yaml | PyYAML | Speedup |
|-----------|-----------|--------|---------|
| Parse (small) | 0.00 ms | 0.11 ms | **25×** |
| Parse (medium) | 0.03 ms | 0.75 ms | **28×** |
| Parse (large) | 0.07 ms | 1.83 ms | **26×** |
| Serialize (small) | 0.01 ms | 0.19 ms | **36×** |
| Serialize (medium) | 0.03 ms | 1.21 ms | **40×** |
| Serialize (large) | 0.08 ms | 2.96 ms | **37×** |

---

**[Get Started →](quick-start.md)**
**[Browse API Reference →](api/reference.md)**
**[View on GitHub →](https://github.com/759401524/pyrs-yaml)**
