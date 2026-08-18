---
title: 解析 YAML
description: 使用 pyrs-yaml 解析 YAML 的完整指南，涵盖基本解析、文件解析、多文档解析和 PyYAML 兼容解析。
tags:
  - docs
status: new
---

## 解析 YAML

本指南介绍使用 pyrs-yaml 解析 YAML 的所有方法。

### 基本解析

#### 解析 YAML 字符串

```python title="解析字符串"
import pyrs_yaml

doc = pyrs_yaml.parse("key: value")  # (1)!
print(doc.get("key"))  # value
```

1. `parse()` 返回一个 [`YamlDocument`](../api/yaml-document.md)，保留注释、锚点和格式。

#### 使用选项解析

```python title="带选项解析"
# 禁用合并键解析（保留 <<: *alias 原样）
doc = pyrs_yaml.parse(yaml_text, resolve_merges=False)
```

#### 解析 YAML 文件

```python title="解析文件"
doc = pyrs_yaml.parse_file("config.yaml")
print(doc.get("name"))
```

#### 解析多个文档

```python title="解析多个文档"
# 使用 --- 分隔的 YAML
yaml_text = """
---
name: first
---
name: second
"""

docs = pyrs_yaml.parse_all_docs(yaml_text)
print(len(docs))  # 2
print(docs[0].get("name"))  # first
print(docs[1].get("name"))  # second
```

#### PyYAML 兼容解析

!!! tip "PyYAML 兼容解析"
    使用 `safe_load` 可将 YAML 解析为原生 Python 类型，与 PyYAML 的 `yaml.safe_load()` 行为一致。

```python title="PyYAML 兼容解析"
# 返回原生 Python 类型（dict, list, str, int 等）
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# 多个文档
docs = pyrs_yaml.safe_loads("a: 1\n---\nb: 2")
print(len(docs))  # 2
```

### 支持的输入类型

pyrs-yaml 支持三种输入形式：

- :material-language-python: **`str`** — 标准 YAML 字符串
- :material-binary: **`bytes`** — 有效的 UTF-8 编码字节
- :material-format-list-bulleted: **`str` 带 BOM** — 正确处理

=== "str"

    ```python title="str 输入"
    doc = pyrs_yaml.parse("key: value")
    ```

=== "bytes"

    ```python title="bytes 输入"
    doc = pyrs_yaml.parse(b"key: value")
    ```

### 错误处理

```python title="错误处理"
try:
    doc = pyrs_yaml.parse("invalid: yaml: [")
except pyrs_yaml.YamlParseError as e:
    print(f"解析错误: {e}")
```

### 支持的数据类型

pyrs-yaml 正确解析所有 YAML 1.2 标量类型：

| 类型 | 示例 | Python 类型 |
|------|------|------------|
| :material-format-text: 字符串 | `hello` | `str` |
| :material-numeric: 整数 | `42`, `0x1A`, `0o77` | `int` |
| :material-decimal: 浮点数 | `3.14`, `1.23e-4` | `float` |
| :material-toggle-switch: 布尔值 | `true`, `false` | `bool` |
| :material-null: 空值 | `null`, `~` | `None` |
| :material-infinity: 无穷大 | `.inf`, `-.inf` | `float` |
| :material-alphabetical: NaN | `.nan` | `float` |

---

### 另请参阅

- [序列化](serialization.md) — 将文档转换回 YAML 字符串
- [就地编辑](editing.md) — 修改已解析文档而不丢失格式
- [自定义 Schema](custom-schema.md) — 定义自定义类型解析规则
