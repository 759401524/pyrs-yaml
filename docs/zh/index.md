---
title: pyrs-yaml
description: pyrs-yaml 高性能 Python YAML 库的首页，介绍核心特性和快速开始示例。
tags:
  - docs
status: new
---

## pyrs-yaml

**高性能 Python YAML 库，完美往返支持，基于 Rust 和 PyO3 构建。**

### 为什么选择 pyrs-yaml？

大多数 Python YAML 库都在性能和保真度之间做出权衡。pyrs-yaml 同时提供两者：

- **PyYAML** (Python) — 慢，往返时**丢失注释/锚点/标签**
- **ruamel.yaml** (Python) — 保留格式，但比 pyrs-yaml **解析慢 48–100 倍、序列化慢 123–371 倍**
- **pyrs-yaml** (Rust) — 比 PyYAML **解析快 21–43 倍、序列化快 55–177 倍**，同时保留所有内容

### 核心特性

<div class="grid cards" markdown>

- :material-lightning-bolt: **极速** — 解析比 PyYAML 快 21–43 倍、序列化快 55–177 倍，Rust 零拷贝后端驱动
- :material-sync: **完美往返** — 保留注释、锚点、标签、chomping 指示符、标量样式和流式/块式格式
- :material-pencil: **就地编辑** — 通过 JSONPath 风格路径（`doc.set("$.a.b", v)`）或 `Node` 树 API 编辑已解析文档，不丢失格式
- :material-check-decagram: **YAML 1.2 合规** — 由 granit-parser 驱动（YAML 测试套件通过率 99.75%，405/406）
- :material-swap-horizontal: **PyYAML 兼容** — 可直接替换，提供 `safe_load` / `safe_dump` API
- :material-language-python: **类型提示** — PEP 561 合规，提供完整的 `.pyi` 存根文件
- :material-package-variant-closed: **ABI3 Wheel** — 单个 wheel 支持 Python 3.8–3.15
- :material-translate: **国际化错误** — `set_language("zh-CN")` 支持双语错误报告
- :material-numeric: **NumPy ndarray** — 将任意维度的 `numpy.ndarray` 序列化为 YAML，零拷贝 Rust 调度

</div>

### 快速开始

```bash title="安装"
pip install pyrs-yaml
```

```python title="快速开始"
import pyrs_yaml

# 解析 YAML
doc = pyrs_yaml.parse("key: value")
print(doc.to_yaml())  # key: value\n

# PyYAML 兼容 API
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# 往返保留注释
original = "# Comment\nkey: value  # inline\n"
doc = pyrs_yaml.parse(original)
assert doc.to_yaml() == original
```

### 与 PyYAML 的性能对比

| 操作 | pyrs-yaml | PyYAML | 速度提升 |
|-----------|-----------|--------|---------|
| Parse (small) | 0.18 ms | 3.8 ms | **21×** |
| Parse (medium) | 0.56 ms | 24.2 ms | **43×** |
| Parse (large) | 1.5 ms | 57.7 ms | **38×** |
| Serialize (small) | 0.04 ms | 2.2 ms | **55×** |
| Serialize (medium) | 0.08 ms | 12.6 ms | **159×** |
| Serialize (large) | 0.17 ms | 30.2 ms | **177×** |

---

[开始使用 :material-arrow-right:](quick-start.md){ .md-button .md-button--primary }
[浏览 API 参考 :material-code-braces:](api/reference.md){ .md-button }
[在 GitHub 上看看 :fontawesome-brands-github:](https://github.com/759401524/pyrs-yaml){ .md-button }
