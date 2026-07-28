# ---

---

title: pyyaml-rs
lang: zh-CN

## pyyaml-rs

# 高性能的 Python YAML 库，支持完美的往返（Round-Trip）解析，由 Rust 和 PyO3 构建。

---

### 为什么选择 pyyaml-rs？

大多数 Python YAML 库都在性能和保真度之间做出权衡。 pyyaml-rs 同时提供两者:

- **PyYAML** (Python) — slow, **loses comments/anchors/tags** on round-trip
- **ruamel.yaml** (Python) — preserves formatting, but **5–10× slower** than pyyaml-rs
- **pyyaml-rs** (Rust) — **25–40× faster than PyYAML** while preserving everything

### 核心特性

- **YAML 1.2 compliant** — powered by saphyr-parser (98.1% YAML Test Suite pass rate)
- **Perfect Round-Trip** — preserves comments, anchors, tags, chomping, scalar styles, and flow/block formatting
- **25–40× faster** than PyYAML — Rust backend with zero-copy parsing
- **Custom AST** — extensible AST for advanced YAML manipulation and custom formatting
- **PyYAML compatible** — drop-in replacement with `safe_load` / `safe_dump` API
- **Type hints** — PEP 561 compliant with full `.pyi` stubs
- **ABI3** — single wheel works across Python 3.9–3.13
- **i18n error messages** — `set_language("zh-CN")` for bilingual error reporting
- **NumPy ndarray support** — serialize `numpy.ndarray` of any dimension to YAML with zero-copy Rust dispatch

### 快速开始

```bash
pip install pyyaml-rs
```

```python
import pyyaml_rs

# Parse YAML
doc = pyyaml_rs.parse("key: value")
print(doc.to_yaml())  # key: value\n

# PyYAML compatible API
data = pyyaml_rs.safe_load("key: value")
print(data)  # {'key': 'value'}

# Round-trip preserves comments
original = "# Comment\nkey: value  # inline\n"
doc = pyyaml_rs.parse(original)
assert doc.to_yaml() == original
```

### 与 PyYAML 的性能对比

| Operation | pyyaml-rs | PyYAML | Speedup |
|-----------|-----------|--------|---------|
| Parse (small) | 0.00 ms | 0.11 ms | **25×** |
| Parse (medium) | 0.03 ms | 0.75 ms | **28×** |
| Parse (large) | 0.07 ms | 1.83 ms | **26×** |
| Serialize (small) | 0.01 ms | 0.19 ms | **36×** |
| Serialize (medium) | 0.03 ms | 1.21 ms | **40×** |
| Serialize (large) | 0.08 ms | 2.96 ms | **37×** |

---

# [开始使用 →](quick-start.md)
# [浈西 API 参考 →](api/reference.md)
# [在 GitHub 上看看 →](https://github.com/759401524/pyyaml-rs)
