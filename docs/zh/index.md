---

title: pyyaml-rs
lang: zh

## pyyaml-rs

# 高性能的 Python YAML 库，支持完美的往返（Round-Trip）解析，由 Rust 和 PyO3 构建。

---

## 为什么选择 pyyaml-rs？

大多数 Python YAML 库都在性能和保真度之间做出权衡。 pyyaml-rs 同时提供两者:

- **PyYAML** (Python) — 慢，往返解析时**丢失注释/锚点/标签**
- **ruamel.yaml** (Python) — 保留格式，但比 pyyaml-rs **慢 5–10 倍**
- **pyyaml-rs** (Rust) — 比 PyYAML **快 25–40 倍**，同时保留所有内容

### 核心特性

- **YAML 1.2 合规** — 由 saphyr-parser 驱动（YAML 测试套件通过率 98.1%）
- **完美的往返解析** — 保留注释、锚点、标签、修剪指示符、标量样式和流式/块式格式
- **比 PyYAML 快 25–40 倍** — Rust 后端，零拷贝解析
- **自定义 AST** — 可扩展的 AST，用于高级 YAML 操作和自定义格式化
- **PyYAML 兼容** — 可直接替换，提供 `safe_load` / `safe_dump` API
- **类型提示** — PEP 561 合规，提供完整的 `.pyi` 存根文件
- **ABI3** — 单个 wheel 支持 Python 3.9–3.13
- **国际化错误消息** — `set_language("zh")` 支持双语错误报告
- **NumPy ndarray 支持** — 将任意维度的 `numpy.ndarray` 序列化为 YAML，零拷贝 Rust 调度

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

## [开始使用 →](quick-start.md)

## [浏览 API 参考 →](api/reference.md)

## [在 GitHub 上看看 →](https://github.com/759401524/pyyaml-rs)
