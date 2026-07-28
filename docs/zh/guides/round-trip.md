---

title: 往返保存
lang: zh

## 往返保存

这是 pyyaml-rs 的**核心特性** — 在 Python YAML 库中独树一帜。

### 什么是往返保存？

往返保存意味着：**解析 YAML → 修改 → 序列化 → 输出与输入相同（或语义等效）。**

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

doc = pyyaml_rs.parse(original)
output = doc.to_yaml()

# 所有格式和元数据都被保留
assert "# 服务器配置" in output
assert "# 主端口" in output
assert "&db" in output
assert "<<: *db" in output
```

### 保留的内容

| 元素 | 是否保留 | 说明 |
|------|---------|------|
| 独立行注释 | ✅ | 键和值之前 |
| 行内注释 | ✅ | 行尾 |
| 锚点 (`&name`) | ✅ | 完整的锚点语法 |
| 别名 (`*name`) | ✅ | 别名引用被解析 |
| 合并键 (`<<`) | ✅ | 默认被解析 |
| 标签 (`!!str`, `!!int`) | ✅ | 显式标签被保留 |
| 标量样式 | ✅ | Plain、引号、字面量、折叠 |
| Chomping (`\|-`, `>-`) | ✅ | 块标量指示符 |
| 流式/块式风格 | ✅ | `[]`/`{}` 与块式被保留 |
| 键顺序 | ✅ | `IndexMap` 保证顺序 |

### PyYAML vs pyyaml-rs 往返保存

```python
original = "# 注释\nkey: value  # 行内注释\n"

# PyYAML: 丢失一切
yaml.safe_dump(yaml.safe_load(original))
# 输出: 'key: value\n'  ❌

# pyyaml-rs: 保留一切
doc = pyyaml_rs.parse(original)
doc.to_yaml()
# 输出: '# 注释\nkey: value  # 行内注释\n'  ✅
```

### 性能

与其他库的往返性能对比：

| 库 | 往返保存 (大文件) | 注释 | 锚点 | 标签 |
|---|------------------|------|------|------|
| **pyyaml-rs** | **0.08 ms** | ✅ | ✅ | ✅ |
| PyYAML | 2.98 ms | ❌ | ❌ | ❌ |
| ruamel.yaml | 6.79 ms | ✅ | ✅ | ✅ |

**pyyaml-rs 比 PyYAML 快 37 倍，比 ruamel.yaml 快 85 倍**，同时保留所有内容。
