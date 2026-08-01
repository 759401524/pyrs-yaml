---

title: YamlDocument 类
lang: zh

## YamlDocument 类

### 概述

`YamlDocument` 是 pyrs-yaml 的核心类，保存已解析的 YAML 文档。它使用基于 `IndexMap` 的自定义 AST，实现 **100% 往返解析**、**完整的键顺序保留**、**嵌套注释保留** 和 **详细元数据**。

```python
class YamlDocument:
    """pyrs-yaml 的核心类。"""

    # ... C 扩展实现 ...
```

### 构造函数

#### `YamlDocument()`

内部构造函数。用户不应直接调用。从 `pyrs_yaml.parse()` 返回。

### 属性

- `version` — YAML 文档版本
- `schema` — 模式（`core`、`failsafe`、`json`）
- `tags` — 标签列表
- `anchors` — 锚点列表
- `source` — YAML 源文本

### 方法

#### `to_yaml()`

将文档转换为 YAML 字符串。

```python
to_yaml(
    indent: int = 2,
    allow_unicode: bool = True,
    default_flow_style: bool = False,
    sort_keys: bool = False,
    width: int = 80,
    resolve_aliases: bool = True,
    strip_comments: bool = False,
    preserve_quotes: bool = True,
) -> str
```

**参数:**

- `indent` — 缩进空格数（默认：2）
- `allow_unicode` — 允许 Unicode 字符（默认：True）
- `default_flow_style` — 默认使用流式风格（默认：False）
- `sort_keys` — 对键排序（默认：False）
- `width` — 折行宽度（默认：80）
- `resolve_aliases` — 解析别名（默认：True）
- `strip_comments` — 去除注释（默认：False）
- `preserve_quotes` — 保留引号（默认：True）

**返回值:** YAML 字符串

**示例:**

```python
doc = pyrs_yaml.parse("key: value\n# comment")
yaml_str = doc.to_yaml()
```

#### `to_dict()`

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

#### `get()`

通过键获取值（用于映射根）。

```python
get(key: str, default: Any = None) -> Any
```

**返回值:** 值，未找到则返回默认值

#### `type()`

以字符串形式获取根节点类型。

```python
type() -> str
```

**返回值:** 类型名（`"mapping"`、`"sequence"`、`"scalar"`）

#### `to_json()`

将文档序列化为 JSON 字符串。

```python
to_json(indent: int = 2) -> str
```

**返回值:** JSON 字符串

#### `validate()`

根据 JSON Schema 验证文档内容。

```python
validate(schema: dict[str, Any]) -> None
```

**引发:** `YamlValidateError` — 验证错误

#### `reload()`

就地重新解析存储的源文本，允许更改模式或合并行为。

```python
reload(schema: str = "core", resolve_merges: bool = True) -> None
```

#### `source_text()`

返回用于创建此文档的原始 YAML 源文本。

```python
source_text() -> str
```

**返回值:** YAML 源字符串

### 编辑方法

就地编辑文档，同时保留所有元数据（注释、锚点、标签、样式）。编辑通过 JSONPath 风格路径（`$.a.b`、`$.items[0]`）定位节点，所有操作都是**原子**的 — 失败时文档（含修订号）不变。

#### `set()`

按路径替换值。

```python
set(path: str, value: Any) -> None
```

- 支持标量、`dict`、`list`；`tuple` 不支持（引发 `YamlEditError`）
- 替换现有标量时保留目标的元数据；路径不存在时在映射末尾添加新键

**示例:**

```python
doc = pyrs_yaml.parse("a:\n  b: 1")
doc.set("$.a.b", 42)
doc.set("$.a.c", True)  # 添加新键
doc.set("$", {"x": 1})  # 替换整个根
```

#### `insert()`

在序列的指定索引处插入值。

```python
insert(path: str, index: int, value: Any) -> None
```

`index` 最大可为序列当前长度（在 `len` 处插入等同于追加）。路径必须解析为序列节点。

#### `append()`

在序列末尾追加值。

```python
append(path: str, value: Any) -> None
```

#### `delete()`

按路径删除节点。映射顺序保留。

```python
delete(path: str) -> None
```

#### `rename()`

就地重命名映射键（保持位置和元数据）。

```python
rename(path: str, new_key: str) -> None
```

重命名根节点或复杂（非标量）键会引发 `YamlEditError`。

#### `node()`

返回文档根节点的 `Node`。

```python
node() -> Node
```

#### `find()`

按路径查找节点。支持通配符（`[*]`）和深度扫描（`..`）— 此时返回节点列表。

```python
find(path: str) -> Node | list[Node]
```

**引发:**

- `YamlPathError` — 路径格式错误，或在编辑路径中使用通配符/`..`
- `YamlEditError` — 编辑无法应用（`tuple`、负索引、通过别名编辑、重命名根/复杂键、导航进入标量、索引越界）
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

### 特殊方法

#### `__getitem__()`

通过键（映射）或索引（序列）访问。

```python
doc = pyrs_yaml.parse("key: value")
value = doc["key"]  # 'value'
```

#### `__setitem__()`

设置根映射键（`doc.set()` 的根节点语法糖）。

```python
doc["key"] = value
```

#### `__delitem__()`

删除根映射键（`doc.delete()` 的根节点语法糖）。

```python
del doc["key"]
```

#### `__contains__()`

检查键是否存在。

```python
"key" in doc  # True
```

#### `__len__()`

获取项目数量。

```python
len(doc)
```

#### `__iter__()`

遍历键（映射）或值（序列）。

```python
for key in doc:
    print(key)
```

#### `__repr__()`

调试表示。

```python
repr(doc)  # "YamlDocument({key: value})"
```

#### `__str__()`

字符串表示。

```python
str(doc)  # "YamlDocument({key: value})"
```

#### `__eq__()`

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
