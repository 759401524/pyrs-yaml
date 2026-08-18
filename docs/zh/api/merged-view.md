---
title: MergedView 类
description: pyrs-yaml 中 MergedView 类的 API 参考文档，提供合并键解析后的只读字典视图。
tags:
  - docs
status: new
---

## MergedView 类

`MergedView` 类提供 `YamlDocument` 的只读视图，其中合并键（`<<: *anchor`）已被解析。通过 `doc.merged()` 访问。

### 概述

```python
class MergedView(Mapping):
    """Read-only view of a YAML document with merge keys resolved."""
```

该视图从 `YamlDocument.to_dict()` 延迟构建，在序列化过程中解析锚点和合并键。原始 AST 永远不会被修改。

### 构造函数

#### `MergedView.__init__()`

```python
MergedView.__init__(document: YamlDocument) -> None
```

**Parameters:**

- `document` — 一个 `YamlDocument` 实例

如果文档根节点是序列，则视图将其转换为整数键映射（`{0: item0, 1: item1, ...}`）。

### 方法

#### `__getitem__()`

按键访问值。

```python
__getitem__(key: str | int) -> Any
```

子字典和列表分别递归包装为 `MergedView._DictView` 和 `MergedView._ListView`。

**Example:**

```python
doc = pyrs_yaml.parse("""
defaults: &defaults
  timeout: 30
  retries: 3

config:
  <<: *defaults
  timeout: 60
""")

view = doc.merged()
print(view["config"]["timeout"])  # 60 (覆盖合并值)
print(view["config"]["retries"])  # 3 (从合并继承)
```

#### `__len__()`

返回顶层条目数量。

```python
__len__() -> int
```

#### `__iter__()`

遍历顶层键。

```python
__iter__() -> Iterator[str | int]
```

#### `__repr__()`

```python
__repr__() -> str
```

返回 `MergedView({...})` 及内部字典表示。

#### `get()`

`get()` 继承自 `collections.abc.Mapping` — 提供 `get(key, default=None)`。

```python
get(key: str | int, default: Any = None) -> Any
```

### 合并键解析

键的解析优先级如下（最高优先）：

1. 直接在合并文档中定义的键
2. 来自合并锚点的键（按在 `<<:` 中出现的顺序）
3. 后出现的锚点覆盖先出现的锚点

### 根类型支持

| Root Type | 行为 |
| --- | --- |
| Mapping | 键为映射的键 |
| Sequence | 键为整数索引（`0`、`1`、...） |
| Scalar/Null | `__len__()` 返回 `0`；`__getitem__()` 抛出 `KeyError` |

### 示例

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
base: &base
  host: localhost
  port: 8080

prod:
  <<: *base
  host: prod.example.com
  debug: false
""")

merged = doc.merged()
assert merged["base"]["host"] == "localhost"
assert merged["prod"]["host"] == "prod.example.com"  # 被覆盖
assert merged["prod"]["port"] == 8080  # 继承
assert merged["prod"]["debug"] is False  # 自有键
```
