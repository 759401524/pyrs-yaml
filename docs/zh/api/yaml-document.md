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
    note_placement: str = "note_first",
) -> str
```

**参数:**

- `indent_size` — 每级缩进空格数（默认：2）
- `explicit_start` — 在文档开头添加 `---`（默认：False）
- `explicit_end` — 在文档末尾添加 `...`（默认：False）
- `sort_keys` — 按字母顺序对键排序（默认：False）
- `note_placement` — 注释无法与其行同处一行、且其上方是 `&a` / `!tag` 头行时的落点：`note_first`（默认）把它写在头行之上，即读取所归档处，于是首次发射即收敛；`header_first` 把它留在间隙里。其他取值一律拒绝

**示例:**

```python
yaml_str = doc.to_yaml_with_options(
    indent_size=4,
    explicit_start=True,
    sort_keys=True,
)
```

### `to_dict()`

转换为 Python dict/list。解析别名引用，返回原生 Python 类型。同一套解析同样作用于映射的键：`~: 1` 的键是 `None`，`1: a` 的键是整数 1——同一段文本，无论在 `:` 的哪一侧，类型一致。

```python
to_dict() -> dict[Any, Any] | list[Any]
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

**返回值:** `"scalar"`、`"mapping"`、`"sequence"`、`"null"` 之一。

`"alias"` 是绑定为一个别名根保留的拼法，但没有任何文档读取能到达它：别名需要同一个文档里更早出现的锚点，所以根上的别名会被解析器拒绝。`pyrs_yaml.parse("*x\n")` 回 `YamlParseError: YAML parse error: while parsing node, found unknown anchor at char 0 line 1 column 1`，而跨文档的写法 `"--- &a 1\n--- *a\n"` 在第 2 行遭到同样的拒绝。

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

**引发:**

- `YamlValidateError` — 文档不符合 schema。既不是 `str` 也不是映射的 schema 同样落到这里，带着引擎自己那句话作为它的文本：`doc.validate(3)` 回 `TypeError: argument of type 'int' is not iterable`

有两道拒绝来自这个类之外，因为求值委托给 Python 的 `jsonschema` 包，而本项目发布的任何 extra 都不安装它：

- `jsonschema` 无法导入时，`doc.validate({"type": "object"})` 回一个裸的 `ModuleNotFoundError: No module named 'jsonschema'`
- `str` 形式的 schema 在问求值器之前会先跑 `json.loads`，所以 YAML 拼法的 schema 文本回 `json.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)` — 装不装 `jsonschema` 都是同一句

两条都连同各自的复现记录在 [异常 — YamlValidateError](exceptions.md#yamlvalidateerror)。

### `flush_source()`

```python
flush_source() -> None
```

在发生过编辑之后，把 AST 惰性重新序列化进 `source`，使 `source` 反映当前语法树，而不必每次改动都强制全量导出。

### `reparse()`

就地重新解析存储的源文本，允许更改模式或合并行为。

```python
reparse(resolve_merges: bool = True, schema: str | None = None, honor_version_directive: bool | None = None) -> None
```

**参数:**

- `resolve_merges` — 是否解析 `<<: *alias` 合并键（默认：`True`）
- `schema` — 类型解析模式：`"core"`、`"json"`、`"failsafe"` 或 `"yaml1.1"`。传 `None` 则沿用文档读入时
  所用的模式。
- `honor_version_directive` — 对声明的 `%YAML` 做版本协商。取 `True` 时，声明 `%YAML 1.1` 的文档按 1.1
  档位解析，并重新发射该指令；取 `False` 时忽略声明，保持所选模式。取 `None` 时沿用文档当前设置。

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
- `YamlEditError` — 无法应用的路径步骤，消息里会点名它：穿过别名的步骤回 `edit failed: cannot-edit-alias`，进入标量或落到形状不对的容器上的步骤回 `create-needs-mapping`，`create_missing=True` 撞上标量时回 `cannot-descend-into-scalar:<segment>`，在被写入的键**之前**就已缺失的键回 `missing-path:<key>`，越过序列尽头的索引回 `index-out-of-range-edit:<index>`。缺失的**末级**键不是拒绝 — `parse("a: 1\n").set("$.zz", 3)` 发射 `a: 1\nzz: 3\n` — 写入一个存活锚点的源同样不是拒绝：在 `base: &x 1\nuse: *x\n` 上 `set("$.base", 2)` 重新发射 `base: &x 2\nuse: *x\n`，把 `&x` 留在它的别名仍然找得到的地方
- `YamlTypeError` — 写入器不认识的 Python 类型的值；`tuple` 会被接受，并按序列写出，而 `bytes` 会被接受成 `!!binary`

完整的实测词汇表按调用逐条列在 [编辑原因令牌](exceptions.md#edit-reason-tokens)。

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

**引发:**

- `YamlEditError` — 路径解析为映射或标量时回 `edit failed: not-a-sequence`，索引高于 `len` 或低于 `-len` 时回 `edit failed: index-out-of-range-edit`，路径解析不到时回 `edit failed: missing-path:<key>`，路径穿过别名时回 `edit failed: cannot-edit-alias`，路径位于标量值之下时回 `edit failed: cannot-descend-into-scalar:<segment>`

### `append()`

在序列末尾追加值。

```python
append(path: str, value: Any) -> None
```

**引发:** 与 `insert()` 相同的那些 `YamlEditError` 理由，实测下来非序列目标是 `edit failed: not-a-sequence`，别名父节点是 `edit failed: cannot-edit-alias`。

### `delete()`

按路径删除节点。映射顺序保留。

```python
delete(path: str, on_duplicate: str = "one") -> None
```

**引发:**

- `YamlEditError` — 文档里没有这个键时回 `edit failed: missing-path`（`del doc["zz"]` 也是这个答案），索引落在序列之外时回 `edit failed: index-out-of-range-edit`，节点的锚点仍被引用时回 `edit failed: anchor-is-referenced`，`on_duplicate="error"` 且多于一对键拼出这个名字时回 `edit failed: duplicate-key-target`，其他任何 `on_duplicate` 取值回 `edit failed: unknown on_duplicate "<value>"; expected one, all or error`，路径穿过别名时回 `edit failed: cannot-edit-alias`，而 `delete("$")` 回 `edit failed: edit-error` — 根节点已没有可移除的段

### `rename()`

就地重命名映射键（保持位置和元数据）。

```python
rename(path: str, new_key: str) -> None
```

重命名根节点或复杂（非标量）键会引发 `YamlEditError`。

**引发:**

- `YamlEditError` — `$` 回 `edit failed: cannot-rename-root`，复杂键或序列元素回 `edit failed: cannot-rename-complex-key`，映射已经持有的 `new_key` 回 `edit failed: rename-key-exists`，路径解析不到时回 `edit failed: missing-path`，路径穿过别名时回 `edit failed: cannot-edit-alias`

把键重命名为它自己不是拒绝：在 `a: 1` 上实测，`rename("$.a", "a")` 让文档仍是 `a: 1`。

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
- `YamlEditError` — 编辑无法应用（通过别名编辑、重命名根/复杂键、导航进入标量、索引越界）
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

**引发:**

- `KeyError` — 映射里没有这个键。实测文本是 `Key 'zz' not found`，而因为这就是内建的 `KeyError`，`str(exc)` 会连引号一起显示：`"Key 'zz' not found"`
- `IndexError` — 序列的索引超出范围。在 `pyrs_yaml.parse("- a\n- b\n")[99]` 上实测：`Index 99 out of range (len=2)`
- `TypeError` — 键的种类不对，或根节点根本不可下标，这两种都由 `YamlTypeError` 分支应答，各自实测：`parse("- a\n")["k"]` → `Index must be an integer`，`parse("a: 1\n")[99]` → `Key must be a string`，`parse("just a scalar")[0]` → `YamlDocument is not subscriptable`

上面每一句文本都跟随 `pyrs_yaml.set_language()` — 在 `zh-CN` 下实测，四个分支分别答 `索引必须是整数`、`键必须是字符串`、`YamlDocument 不支持下标操作` 与 `索引 99 超出范围（长度=2）`。参见 [i18n 支持](exceptions.md#i18n-support)。

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

## 生成的 API 参考

以下内容由 `mkdocstrings` 从库自身的 docstring 与生成的类型存根渲染，签名来自代码，不是手抄。

::: pyrs_yaml.YamlDocument
