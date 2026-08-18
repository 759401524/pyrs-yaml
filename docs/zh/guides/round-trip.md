---
title: 往返
description: pyrs-yaml 核心特性——往返（Round-Trip）解析的详细介绍，保留注释、锚点、标签等所有格式元数据。
tags:
  - docs
status: new
---

这是 pyrs-yaml 的**核心特性** — 在 Python YAML 库中独树一帜。

## 什么是往返？

往返意味着：**解析 YAML → 修改 → 序列化 → 输出与输入相同（或语义等效）。**

```python
original = """
# 服务器配置
server:
  host: 0.0.0.0
  port: 8080  # 主端口

# 数据库锚点
database: &db
  host: localhost
  port: 5432

api:
  <<: *db
  endpoint: /api/v1
"""

doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# 所有格式和元数据都被保留
assert "# 服务器配置" in output
assert "# 主端口" in output
assert "&db" in output
# 注意：合并键（<<）默认被解析（实体化），不会按原样输出；
# 要原样保留 <<: *db，请使用 resolve_merges=False
```

## 保留的内容

| 元素 | 是否保留 | 说明 |
|------|---------|------|
| 独立行注释 | :material-check: | 键和值之前 |
| 行内注释 | :material-check: | 行尾 |
| 锚点 (`&name`) | :material-check: | 完整的锚点语法 |
| 别名 (`*name`) | :material-check: | 别名引用被解析 |
| 合并键 (`<<`) | :material-alert: | 默认被解析；`resolve_merges=False` 时保留 |
| 标签 (`!!str`, `!!int`) | :material-check: | 显式标签被保留 |
| 标量样式 | :material-check: | Plain、引号、字面量、折叠 |
| Chomping (`\|-`, `>-`) | :material-check: | 块标量指示符 |
| 流式/块式风格 | :material-check: | `[]`/`{}` 与块式被保留 |
| 紧凑序列项 | :material-check: | `- host: a` 保持在破折号行（仅限无元数据的映射项） |
| 键顺序 | :material-check: | `IndexMap` 保证顺序 |

## PyYAML vs pyrs-yaml 往返

```python
original = "# 注释\nkey: value  # 行内注释\n"

# PyYAML: 丢失一切
yaml.safe_dump(yaml.safe_load(original))
# 输出: 'key: value\n'  :material-close:

# pyrs-yaml: 保留一切
doc = pyrs_yaml.parse(original)
doc.to_yaml()
# 输出: '# 注释\nkey: value  # 行内注释\n'  :material-check:
```

## 性能

与其他库的往返性能对比：

| 库 | 往返 (大文件) | 注释 | 锚点 | 标签 |
|---|------------------|------|------|------|
| **pyrs-yaml** | **0.08 ms** | :material-check: | :material-check: | :material-check: |
| PyYAML | 2.98 ms | :material-close: | :material-close: | :material-close: |
| ruamel.yaml | 6.79 ms | :material-check: | :material-check: | :material-check: |

**pyrs-yaml 比 PyYAML 快 37 倍，比 ruamel.yaml 快 85 倍**，同时保留所有内容。

---

### 另请参阅

- [序列化](serialization.md) — 不丢失格式地序列化文档
- [就地编辑](editing.md) — 编辑同时保留往返保真度
- [PyYAML 兼容](pyyaml-compat.md) — 从 PyYAML 迁移
