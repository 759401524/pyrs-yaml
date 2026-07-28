---

title: 解析 YAML
lang: zh

## 解析 YAML

本指南介绍使用 pyyaml-rs 解析 YAML 的所有方法。

### 基本解析

#### 解析 YAML 字符串

```python
import pyyaml_rs

doc = pyyaml_rs.parse("key: value")
print(doc.get("key"))  # value
```

#### 使用选项解析

```python
# 禁用合并键解析（保留 <<: *alias 原样）
doc = pyyaml_rs.parse(yaml_text, resolve_merges=False)
```

#### 解析 YAML 文件

```python
doc = pyyaml_rs.parse_file("config.yaml")
print(doc.get("name"))
```

#### 解析多个文档

```python
# 使用 --- 分隔的 YAML
yaml_text = """
---

name: first
---
name: second
"""

docs = pyyaml_rs.parse_all_docs(yaml_text)
print(len(docs))  # 2
print(docs[0].get("name"))  # first
print(docs[1].get("name"))  # second
```

## PyYAML 兼容解析

```python
# 返回原生 Python 类型（dict, list, str, int 等）
data = pyyaml_rs.safe_load("key: value")
print(data)  # {'key': 'value'}

# 多个文档
docs = pyyaml_rs.safe_loads("a: 1\n---\nb: 2")
print(len(docs))  # 2
```

### 支持的输入类型

- `str` — 标准 YAML 字符串
- `bytes` — 有效的 UTF-8 编码字节
- `str` 带 BOM — 正确处理

```python
# 同时接受 str 和 bytes
doc1 = pyyaml_rs.parse("key: value")
doc2 = pyyaml_rs.parse(b"key: value")
```

### 错误处理

```python
try:
    doc = pyyaml_rs.parse("invalid: yaml: [")
except pyyaml_rs.YamlParseError as e:
    print(f"解析错误: {e}")
```

### 支持的数据类型

pyyaml-rs 正确解析所有 YAML 1.2 标量类型：

| 类型 | 示例 | Python 类型 |
|------|------|------------|
| 字符串 | `hello` | `str` |
| 整数 | `42`, `0x1A`, `0o77` | `int` |
| 浮点数 | `3.14`, `1.23e-4` | `float` |
| 布尔值 | `true`, `false` | `bool` |
| 空值 | `null`, `~` | `None` |
| 无穷大 | `.inf`, `-.inf` | `float` |
| NaN | `.nan` | `float` |
