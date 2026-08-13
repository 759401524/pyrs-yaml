---
title: YamlDocument 类
description: pyrs-yaml 核心类 YamlDocument 的完整文档，包括解析、编辑、序列化和特殊方法。
tags:
  - docs
status: new
---

## 概述

`YamlDocument` 是 pyrs-yaml 的核心类，保存已解析的 YAML 文档。它使用基于 `IndexMap` 的自定义 AST，实现 **100% 往返**、**完整的键顺序保留**、**嵌套注释保留** 和 **详细元数据**。

```python
class YamlDocument:
    """已解析的 YAML 文档，支持完美的往返。"""
```

## 方法

### `to_yaml()`

将文档转换为 YAML 字符串。

```python
to_yaml() -> str
```

**返回值:** 完整的 YAML 文档字符串，以换行符结尾。

**示例:**

```python
doc = pyrs_yaml.parse("key: value")
print(doc.to_yaml())  # key: value\n
```

### `to_yaml_with_options()`

使用自定义选项转换为 YAML。

```python
to_yaml_with_options(
    indent_size: int = 2,
    explicit_start: bool = False,
    explicit_end: bool = False,
    sort_keys: bool = False,
) -> str
```

**参数:**

- `indent_size` — 每级缩进空格数（默认：2）
- `explicit_start` — 在文档开头添加 `---`（默认：False）
- `explicit_end` — 在文档末尾添加 `...`（默认：False）
- `sort_keys` — 按字母顺序对键排序（默认：False）

**示例:**

```python
yaml_str = doc.to_yaml_with_options(
    indent_size=4,
    explicit_start=True,
    sort_keys=True,
)
```

### `to_dict()`

转换为 Python dict/list。解析别名引用，返回原生 Python 类型。

```python
to_dict() -> dict[str, Any] | list[Any]
```

**返回值:** 字典或列表

**示例:**

```python
doc = pyrs_yaml.parse("key: value")
data = doc.to_dict()  # {'key': 'value'}
```

### `get()`

按顶层映射键进行字面键查找获取值（与 `__getitem__`/`__setitem__` 一致）。键包含 `.`、`[`、`]` 或 `$` 时也始终按字面键处理，不会解析为路径。

```python
get(key: str, default: Any = None) -> Any
```

**返回值:** 值，未找到则返回默认值

!!! note "路径访问"
    JSONPath 风格的访问请使用 [`find()`](#find) / [`node()`](#node) /
    [`set()`](#set)（如 `$.a.b`、`$.items[-1]`）。

### `root_type()`

以字符串形式获取根节点类型。

```python
root_type() -> str
```

**返回值:** 类型名（`"scalar"`、`"mapping"`、`"sequence"`、`"null"`、`"alias"`）。

### `to_json()`

将文档序列化为 JSON 字符串（通过 Python `json.dumps`）。

```python
to_json(indent: int = 2) -> str
```

**参数:**

- `indent` — JSON 缩进空格数（默认：2）

**返回值:** 文档内容的 JSON 字符串。

### `validate()`

根据 JSON Schema 验证文档内容。

```python
validate(schema: str | dict[str, Any]) -> None
```

**参数:**

- `schema` — JSON Schema，可以是 JSON 字符串或 Python dict

**引发:** `YamlValidateError` — 文档不符合 schema

### `reparse()`

就地重新解析存储的源文本，允许更改模式或合并行为。

```python
reparse(resolve_merges: bool = True, schema: str = "core") -> None
```

**参数:**

- `resolve_merges` — 是否解析 `<<: *alias` 合并键（默认：`True`）
- `schema` — 类型解析模式：`"core"`、`"json"`、`"failsafe"` 或 `"yaml1.1"`（默认：`"core"`）

**引发:**

- `TypeError` — 没有存储的源文本
- `YamlParseError` — 重新解析失败

### `source()`

返回用于创建此文档的原始 YAML 源文本。如果文档已被编辑，源文本会在首次访问时从当前树懒加载重新序列化。

```python
source() -> str
```

**返回值:** YAML 字符串。如果文档不是通过 `parse()` 创建的（例如从 `from_dict()`），则返回空字符串。

## 编辑方法

!!! note "原子性编辑"
    所有编辑操作都是原子的 — 失败的编辑不会改动文档及其修订号，确保数据一致性。

就地编辑文档，同时保留所有元数据（注释、锚点、标签、样式）。编辑通过 JSONPath 风格路径（`$.a.b`、`$.items[0]`）定位节点，所有操作都是**原子**的 — 失败时文档（含修订号）不变。

### `set()`

按 JSONPath 路径设置值。

```python
set(path: str, value: Any, create_missing: bool = False) -> None
```

```python
doc = pyrs_yaml.parse("a:\n  b: 1")
doc.set("$.a.b", 42)  # 替换现有值
doc.set("$.a.c", True)  # 创建新键
doc.set("$", {"x": 1})  # 替换整个根

empty = pyrs_yaml.parse("")
empty.set("$.a", 1)  # 自动创建映射根：{a: 1}
```

**`create_missing` 参数：**

默认情况下，当路径中的中间键不存在时，`set()` 会抛出 `YamlEditError`。使用 `create_missing=True` 时，缺失的中间映射键会被自动创建为嵌套映射：

```python
doc = pyrs_yaml.parse("a: 1\n")
doc.set("$.b.c.d", 2, create_missing=True)
# a: 1
# b:
#   c:
#     d: 2
```

#### 引发：

- `YamlPathError` — 格式错误的路径（通配符/`..` 被拒绝）
- `YamlEditError` — 导航失败、不支持的值类型（`tuple`）、缺失中间键（当 `create_missing=False` 时）等

### `walk()`

深度优先、前序遍历 AST，产生 `Node` 对象。

```python
walk() -> Generator[Node, None, None]
```

与 `Node.walk()` 不同，此方法是 **Rust 后端**的 — 它直接遍历 AST，无需转换为 Python 字典，因此对于大型文档明显更快。

**生成：** 文档树中每个节点的 `Node` 对象，包括根节点。

#### 示例：

```python
doc = pyrs_yaml.parse("a:\n  b: 1\n  c: 2\n")
for node in doc.walk():
    print(node._path, node.root_type)
# ()       mapping
# ('a',)   mapping
# ('a', 'b') scalar
# ('a', 'c') scalar
```

### `scalars()`

与 `walk()` 类似，但仅生成标量/null 节点。

```python
scalars() -> Generator[Node, None, None]
```

**生成：** 文档树中每个标量或 null 节点的 `Node` 对象。

#### 示例：

```python
doc = pyrs_yaml.parse("a: hello\nb: null\n")
for node in doc.scalars():
    print(node._path, node.value)
# ('a',) hello
# ('b',) None
```

#### `insert()`

在序列的指定索引处插入值。

```python
insert(path: str, index: int, value: Any) -> None
```

`index` 最大可为序列当前长度（在 `len` 处插入等同于追加）。负索引从末尾计数（`-1` 在最后一个元素之前插入）。路径必须解析为序列节点。

### `append()`

在序列末尾追加值。

```python
append(path: str, value: Any) -> None
```

### `delete()`

按路径删除节点。映射顺序保留。

```python
delete(path: str) -> None
```

### `rename()`

就地重命名映射键（保持位置和元数据）。

```python
rename(path: str, new_key: str) -> None
```

重命名根节点或复杂（非标量）键会引发 `YamlEditError`。

### `node()`

返回文档根节点的 `Node`。

```python
node() -> Node
```

### `find()`

按路径查找节点。支持通配符（`[*]`）和深度扫描（`..`）— 此时返回节点列表。

```python
find(path: str) -> Node | list[Node]
```

**引发:**

- `YamlPathError` — 路径格式错误，或在编辑路径中使用通配符/`..`
- `YamlEditError` — 编辑无法应用（`tuple`、通过别名编辑、重命名根/复杂键、导航进入标量、索引越界）
- `YamlDocumentError` — 文档编辑后使用过期的 `Node`

**参见:** [就地编辑指南](../guides/editing.md)

**示例:**

```python
doc = pyrs_yaml.parse("items: [1, 2, 3]")
doc.set("$.items[1]", "two")
doc.insert("$.items", 1, "x")  # items: [1, x, 2, 3]
doc.append("$.items", 4)
doc.rename("$.items", "list")  # 重命名映射键
del doc["list"]  # 等价于 doc.delete("$.list")
```

## 特殊方法

### `__getitem__()`

通过键（映射）或索引（序列）访问。

```python
doc = pyrs_yaml.parse("key: value")
value = doc["key"]  # 'value'
```

### `__setitem__()`

设置根映射键（`doc.set()` 的根节点语法糖）。

```python
doc["key"] = value
```

### `__delitem__()`

删除根映射键（`doc.delete()` 的根节点语法糖）。

```python
del doc["key"]
```

### `__contains__()`

检查键是否存在。

```python
"key" in doc  # True
```

### `__len__()`

获取项目数量。

```python
len(doc)
```

### `__iter__()`

遍历键（映射）或值（序列）。

```python
for key in doc:
    print(key)
```

### `__repr__()`

调试表示。

```python
repr(doc)  # "YamlDocument({key: value})"
```

### `__str__()`

字符串表示。

```python
str(doc)  # "YamlDocument({key: value})"
```

### `__eq__()`

等值比较。当两个 `YamlDocument` 具有相同内容时返回 true。

```python
doc1 == doc2  # True or False
```

**示例:**

```python
import pyrs_yaml

# 映射
doc = pyrs_yaml.parse("name: Alice\nage: 30")
print(doc["name"])  # Alice
print(len(doc))  # 2

# 序列
doc = pyrs_yaml.parse("- item1\n- item2")
print(doc[0])  # item1

# 嵌套访问
doc = pyrs_yaml.parse("user:\n  name: Alice")
print(doc["user"]["name"])  # Alice
```

## 使用例

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
name: Alice
age: 30
""")

print(doc.get("name"))  # Alice
print(doc.root_type())  # mapping
print(len(doc))  # 2
print("name" in doc)  # True
for key in doc:
    print(key, doc[key])  # name Alice, age 30
```
