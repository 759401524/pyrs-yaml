---
title: pyrs-yaml
description: pyrs-yaml 高性能 Python YAML 库的首页，介绍核心特性和快速开始示例。
tags:
  - docs
status: new
---

## 为什么选择 pyrs-yaml？

大多数 Python YAML 库都在性能和保真度之间做出权衡。 pyrs-yaml 同时提供两者:

- **PyYAML** (Python) — 慢，往返时**丢失注释/锚点/标签**
- **ruamel.yaml** (Python) — 保留格式，但比 pyrs-yaml **慢 5–10 倍**
- **pyrs-yaml** (Rust) — 比 PyYAML **快 25–40 倍**，同时保留所有内容

### 核心特性

- **YAML 1.2 合规** — 由 saphyr-parser 驱动（YAML 测试套件通过率 98.1%）
- **完美的往返** — 保留注释、锚点、标签、chomping 指示符、标量样式和流式/块式格式
- **就地编辑** — 通过 JSONPath 风格路径（`doc.set("$.a.b", v)`）或 `Node` 树 API 编辑已解析文档，不丢失格式
- **比 PyYAML 快 25–40 倍** — Rust 后端，零拷贝解析
- **自定义 AST** — 可扩展的 AST，用于高级 YAML 操作和自定义格式化
- **PyYAML 兼容** — 可直接替换，提供 `safe_load` / `safe_dump` API
- **类型提示** — PEP 561 合规，提供完整的 `.pyi` 存根文件
- **ABI3** — 单个 wheel 支持 Python 3.9–3.13
- **国际化错误消息** — `set_language("zh")` 支持双语错误报告
- **NumPy ndarray 支持** — 将任意维度的 `numpy.ndarray` 序列化为 YAML，零拷贝 Rust 调度

### 快速开始

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

### 与 PyYAML 的性能对比

| Operation | pyrs-yaml | PyYAML | Speedup |
|-----------|-----------|--------|---------|
| Parse (small) | 0.00 ms | 0.11 ms | **25×** |
| Parse (medium) | 0.03 ms | 0.75 ms | **28×** |
| Parse (large) | 0.07 ms | 1.83 ms | **26×** |
| Serialize (small) | 0.01 ms | 0.19 ms | **36×** |
| Serialize (medium) | 0.03 ms | 1.21 ms | **40×** |
| Serialize (large) | 0.08 ms | 2.96 ms | **37×** |

---

## [开始使用 →](quick-start.md)

## [浏览 API 参考 →](api/reference.md)

## [在 GitHub 上看看 →](https://github.com/759401524/pyrs-yaml)
