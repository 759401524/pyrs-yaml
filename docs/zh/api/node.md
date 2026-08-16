---
title: Node 类
description: pyrs-yaml 中 Node 类的 API 参考文档，用于遍历和操作 YAML AST 节点。
tags:
  - docs
status: new
---

## Node 类

`Node` 类提供对 `YamlDocument` AST 的借用视图，支持树遍历、查询和修改操作。节点通过 `doc.node()`、`doc.find("$.path")` 或 `doc.walk()` 创建。

### Overview

```python
class Node:
    """A node in the YAML AST, backed by a YamlDocument and a path."""
```

每个 `Node` 存储对其父 `YamlDocument` 的引用以及一个路径元组，该元组定位到文档 AST 中的目标节点。当文档被修改或释放时，节点会过期。

### Constructor

#### `Node.__init__()`

```python
Node.__init__(document: YamlDocument, path: tuple = ()) -> None
```

**Parameters:**

- `document` — 父 `YamlDocument`
- `path` — 定位到目标节点的路径段（键/索引）元组

### Properties

#### `value`

获取此节点的标量值。

```python
value -> Any | None
```

对于非标量节点（映射、序列）返回 `None`。

#### `root_type`

获取此节点的类型。

```python
root_type -> str
```

返回 `"scalar"`、`"mapping"`、`"sequence"`、`"null"` 之一。

#### `_path`

定位到文档 AST 中此节点的路径元组。

```python
_path -> tuple
```

#### `children`

获取此节点的子节点。

```python
children -> list[Node]
```

对于标量/空节点返回空列表。

#### `parent`

获取父 `Node`，如果是根节点则返回 `None`。

```python
parent -> Node | None
```

#### `comment`

获取此节点的注释文本，无注释时返回 `None`。

```python
comment -> str | None
```

#### `anchor`

获取此节点的锚点名称，无锚点时返回 `None`。

```python
anchor -> str | None
```

#### `tag`

获取此节点的 YAML 标签字符串（如 `!!str`），无标签时返回 `None`。

```python
tag -> str | None
```

#### `scalar_style`

获取标量 style（`"plain"`、`"single_quoted"`、`"double_quoted"`、`"literal"`、`"folded"`），非标量节点返回 `None`。

```python
scalar_style -> str | None
```

#### `flow_style`

获取 flow style（`True` = flow `{}`/`[]`，`False` = block），非容器节点返回 `None`。

```python
flow_style -> bool | None
```

#### `chomping`

获取 chomping 指示符（`"strip"`、`"clip"`、`"keep"`），非标量节点返回 `None`。

```python
chomping -> str | None
```

### Methods

#### `find()`

通过 JSONPath 类路径查找节点。

```python
find(path: str) -> Node | list[Node]
```

**支持的路径语法：**

| Pattern | 描述 |
| --- | --- |
| `$.key` | 根键 |
| `$.key.subkey` | 嵌套键 |
| `$.arr[0]` | 序列索引 |
| `$.arr[*]` | 序列中所有项 |
| `$..key` | 任意深度搜索键 |
| `$..*` | 所有后代节点 |

**返回：** 精确路径返回单个 `Node`，通配符/深度扫描查询返回 `list[Node]`。

#### `walk()`

遍历所有后代节点（深度优先前序）。

```python
walk() -> Iterator[Node]
```

**生成：** 节点自身，然后递归所有后代。

#### `filter()`

通过谓词函数过滤后代节点。

```python
filter(predicate: Callable[[Node], bool]) -> list[Node]
```

**Parameters:**

- `predicate` — 接收 `Node` 并返回 `bool` 的函数

**Example:**

```python
scalars = root.filter(lambda n: n.root_type == "scalar")
```

#### `set_value()`

替换此节点的值，保留其元数据（注释、锚点、标签、样式）。

```python
set_value(value: Any, create_missing: bool = False) -> None
```

当 `create_missing=True` 时，路径上缺失的中间映射键会被创建为嵌套映射。索引段缺失仍然会报错。

#### `append()`

向序列节点追加值。

```python
append(value: Any) -> None
```

#### `insert()`

在序列节点的指定索引处插入值。

```python
insert(index: int, value: Any) -> None
```

#### `delete()`

移除该节点及其注释。之后节点变为过期。

```python
delete() -> None
```

#### `rename()`

重命名此节点的映射键。该节点必须是映射值。

```python
rename(new_key: str) -> None
```

#### `set_comment()`

设置（或替换）此节点的注释。`standalone=True`（默认）时注释输出在节点上方独立行；`standalone=False` 时注释内联输出在节点之后。

```python
set_comment(text: str, standalone: bool = True) -> None
```

#### `remove_comment()`

删除此节点的注释。

```python
remove_comment() -> None
```

#### `set_anchor()`

设置（或替换）此节点的锚点。

```python
set_anchor(name: str) -> None
```

#### `remove_anchor()`

删除此节点的锚点。

```python
remove_anchor() -> None
```

#### `set_tag()`

设置（或替换）此节点的 YAML 标签。`"!custom"` 生成局部标签，`"!!int"` 生成主（`!!`）标签，`"!<tag:yaml.org,2002:str>"` 生成 verbatim 标签。

```python
set_tag(tag: str) -> None
```

#### `remove_tag()`

删除此节点的 YAML 标签。

```python
remove_tag() -> None
```

#### `set_scalar_style()`

设置（或替换）此节点的标量 style。非标量节点 no-op。值：`"plain"`、`"single_quoted"`、`"double_quoted"`、`"literal"`、`"folded"`。

```python
set_scalar_style(style: str) -> None
```

#### `set_flow_style()`

设置（或替换）此节点的 flow style。`True` 输出 flow（`{}`/`[]`），`False` 输出 block。非容器节点 no-op。

```python
set_flow_style(flow: bool) -> None
```

#### `set_chomping()`

设置（或替换）此节点的 chomping 指示符。值：`"strip"`（`-`）、`"clip"`（默认）、`"keep"`（`+`）。非标量节点 no-op。

```python
set_chomping(chomp: str) -> None
```

#### `to_yaml()`

将此子树序列化为 YAML 字符串。

```python
to_yaml() -> str
```

#### `copy()`

深度复制此子树为与文档分离的独立 Python 值（dict/list/scalar）。可用于通过 `set_value()` 将子树复制粘贴到别处。

```python
copy() -> Any
```

#### `is_valid()`

检查父文档是否仍然存活且未被修改。

```python
is_valid() -> bool
```

#### `release()`

释放对父文档的引用，将此节点标记为过期。

```python
release() -> None
```

调用 `release()` 后，对该节点的任何访问都会发出 `RuntimeWarning` 并抛出 `YamlDocumentError`。

### Dunder Methods

#### `__repr__()`

```python
__repr__() -> str
```

有效节点返回 `Node(root_type=<type>, path=<path>)`，已释放节点返回 `Node(released)`，过期节点返回 `Node(invalid)`。

#### `__eq__()`

```python
__eq__(other: object) -> bool
```

两个 `Node` 实例在共享相同文档、路径和存活状态时相等。

### Stale Node Behavior

!!! warning "过期节点"
    `Node` 与文档的修订号绑定，文档编辑后之前获取的 `Node` 会过期。每次编辑后重新查找节点以继续工作。

节点在以下情况下过期：

- 父 `YamlDocument` 被垃圾回收
- 显式调用 `release()`
- 创建节点后文档被修改

访问过期节点会发出 `RuntimeWarning` 并抛出 `YamlDocumentError`：

```python
>>> node = doc.node()
>>> doc.set("$.key", "new_value")
>>> node.value
RuntimeWarning: Node is stale: the document was modified after this node was created
YamlDocumentError: document has been modified; re-find the node
```

### Example

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
a:
  b: 1
  c: [2, 3, 4]
d: hello
""")

# 获取根节点
root = doc.node()
print(root.root_type)  # "mapping"

# 导航
node = root.find("$.a.c[1]")
print(node.value)  # 3

# 遍历
for n in root.walk():
    print(n._path, n.root_type)

# 过滤
numbers = root.filter(lambda n: n.root_type == "scalar" and isinstance(n.value, int))
for n in numbers:
    print(n._path, n.value)  # ('a', 'b') 1, ('a', 'c', 0) 2, ...

# 修改
root.find("$.a.b").set_value(42)
root.find("$.a.c").append(5)
root.find("$.d").rename("greeting")
root.find("$.a.c[0]").delete()

print(doc.to_yaml())
# a:
#   b: 42
#   c: [3, 4, 5]
# greeting: hello
```
