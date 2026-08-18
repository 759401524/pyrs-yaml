---
title: YAML 类
description: pyrs-yaml 中 YAML 类的 API 参考文档，用于配置解析和序列化行为。
tags:
  - docs
status: new
---

## YAML 类

`YAML` 类是一个可配置的解析器实例，通过 `typ`、`schema`、`max_depth` 和 `allow_duplicate_keys` 设置控制解析行为。支持往返（`rt`）、安全和完整 YAML 解析模式。

### 概述

```python
class YAML:
    """Configured YAML parser instance (rt / safe / full)."""
```

### 构造函数

#### `__init__()`

创建一个可配置的 YAML 解析器实例。

```python
__init__(
    typ: str = "rt",
    schema: str = "core",
    max_depth: int = 1000,
    allow_duplicate_keys: bool = False,
) -> None
```

**Parameters:**

| Parameter | Type | Default | 描述 |
|-----------|------|---------|-------------|
| `typ` | `str` | `"rt"` | 解析器类型。可选 `"rt"`（往返）、`"safe"`（安全）、`"full"`（完整）。 |
| `schema` | `str` | `"core"` | YAML 模式。可选 `"core"`、`"yaml1.1"`、`"failsafe"`、`"json"`。 |
| `max_depth` | `int` | `1000` | 解析的最大嵌套深度。 |
| `allow_duplicate_keys` | `bool` | `False` | 是否允许重复的映射键。 |

**Raises:** 如果 `typ` 或 `schema` 无效，抛出 `YamlTypeError`。

**Example:**

```python
from pyrs_yaml import YAML

# 往返解析器（默认）
yaml = YAML()

# 安全解析器（不解析合并）
yaml_safe = YAML(typ="safe")

# 使用 YAML 1.1 模式的完整解析器
yaml_full = YAML(typ="full", schema="yaml1.1")
```

### 方法

#### `parse()`

解析 YAML 字符串并返回保留完整元数据的 `YamlDocument`。

```python
parse(yaml: str | bytes) -> YamlDocument
```

**Parameters:**

| Parameter | Type | 描述 |
|-----------|------|-------------|
| `yaml` | `str \| bytes` | 要解析的 YAML 内容。 |

**Returns:** 支持往返编辑、注释保留和源跟踪的 `YamlDocument`。

**Notes:**

- 当 `typ` 为 `"rt"` 或 `"full"` 时，启用合并解析（`<<`）。
- 返回的文档保留注释、锚点和格式。

**Example:**

```python
yaml = YAML(typ="rt")
doc = yaml.parse("name: Alice\nage: 30\n")
print(doc.root_type())  # mapping
print(doc["name"])  # Alice
```

#### `safe_load()`

将 YAML 解析为纯 Python `dict` 或 `list`，解析锚点和合并。

```python
safe_load(yaml: str) -> dict[str, Any] | list[Any]
```

**Parameters:**

| Parameter | Type | 描述 |
|-----------|------|-------------|
| `yaml` | `str` | 要解析的 YAML 内容。 |

**Returns:** 所有 YAML 锚点已解析的纯 Python `dict` 或 `list`。

**Notes:**

- 此方法不保留注释、格式或源跟踪。
- 所有锚点引用均被解析 — 结果为纯 Python 对象。
- 解析错误时抛出 `YamlTypeError`。

**Example:**

```python
yaml = YAML(typ="safe")
data = yaml.safe_load("""
person: &ref
  name: Alice
alias: *ref
""")
# data == {"person": {"name": "Alice"}, "alias": {"name": "Alice"}}
```

#### `safe_loads()`

将多文档 YAML 字符串解析为 `dict`/`list` 对象的列表。

```python
safe_loads(yaml: str) -> list[dict[str, Any] | list[Any]]
```

**Parameters:**

| Parameter | Type | 描述 |
|-----------|------|-------------|
| `yaml` | `str` | 多文档 YAML 内容。 |

**Returns:** 纯 Python `dict` 或 `list` 对象的列表，每个文档一个。

**Notes:**

- 文档由 `---` 标记分隔。
- 每个文档内解析锚点和合并。
- 不保留注释和格式。

**Example:**

```python
yaml = YAML(typ="safe")
docs = yaml.safe_loads("""
---
a: 1
---
b: 2
""")
# docs == [{"a": 1}, {"b": 2}]
```

#### `parse_file()`

解析 YAML 文件并返回保留完整元数据的 `YamlDocument`。

```python
parse_file(path: str) -> YamlDocument
```

**Parameters:**

| Parameter | Type | 描述 |
|-----------|------|-------------|
| `path` | `str` | 要读取和解析的文件路径。 |

**Returns:** 支持往返编辑的 `YamlDocument`。

**Raises:** 如果文件无法读取，抛出 `IOError`。

**Notes:**

- 使用 Rust 的 `std::fs::read_to_string` 从磁盘读取文件 — 无 GIL 阻塞。
- 源内容存储在文档中以保证往返保真度。

**Example:**

```python
yaml = YAML(typ="rt")
doc = yaml.parse_file("config.yaml")
print(doc["database"]["host"])
```

#### `parse_all_docs()`

解析多文档 YAML 字符串并返回 `YamlDocument` 对象的列表。

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**Parameters:**

| Parameter | Type | 描述 |
|-----------|------|-------------|
| `yaml` | `str` | 多文档 YAML 内容。 |

**Returns:** `YamlDocument` 对象的列表，每个文档一个。

**Notes:**

- 文档由 `---` 标记分隔。
- 每个文档保留完整的往返支持（注释、锚点、格式）。
- 当 `typ` 为 `"rt"` 或 `"full"` 时，启用合并解析。

**Example:**

```python
yaml = YAML(typ="rt")
docs = yaml.parse_all_docs("""
---
a: 1
---
b: 2
""")
for doc in docs:
    print(doc.root_type())
```

#### `dump_stream()`

流式写入器：将 Python 对象序列化到类文件对象，使用常量内存。

```python
dump_stream(
    file_obj: Any,
    iterable: Any,
    explicit_start: bool = False,
    explicit_end: bool = False,
    sort_keys: bool = False,
) -> None
```

**Parameters:**

| Parameter | Type | Default | 描述 |
|-----------|------|---------|-------------|
| `file_obj` | `Any` | — | 具有 `write(str)` 方法的可写类文件对象。 |
| `iterable` | `Any` | — | 要序列化的 Python 对象的可迭代对象。 |
| `explicit_start` | `bool` | `False` | 是否在每个文档开头输出 `---`。 |
| `explicit_end` | `bool` | `False` | 是否在每个文档结尾输出 `...`。 |
| `sort_keys` | `bool` | `False` | 是否按字母顺序排序映射键。 |

**Raises:** 如果 `file_obj` 没有 `write` 方法，抛出 `YamlTypeError`。

**Notes:**

- 使用常量内存 — 无需将整个输出保留在内存中。
- 在 Rust 序列化阶段释放 GIL。
- 可迭代对象中的每个项目成为单独的 YAML 文档。

**Example:**

```python
import io
from pyrs_yaml import YAML

yaml = YAML()
buf = io.StringIO()
yaml.dump_stream(buf, [{"a": 1}, {"b": 2}], explicit_start=True)
print(buf.getvalue())
# ---
# a: 1
# ---
# b: 2
```

#### `dump_file()`

流式写入器：将 Python 对象直接序列化到磁盘文件。

```python
dump_file(
    path: str,
    iterable: Any,
    explicit_start: bool = False,
    explicit_end: bool = False,
    sort_keys: bool = False,
) -> None
```

**Parameters:**

| Parameter | Type | Default | 描述 |
|-----------|------|---------|-------------|
| `path` | `str` | — | 要写入的文件路径。 |
| `iterable` | `Any` | — | 要序列化的 Python 对象的可迭代对象。 |
| `explicit_start` | `bool` | `False` | 是否在每个文档开头输出 `---`。 |
| `explicit_end` | `bool` | `False` | 是否在每个文档结尾输出 `...`。 |
| `sort_keys` | `bool` | `False` | 是否按字母顺序排序映射键。 |

**Raises:** 如果文件无法创建或写入，抛出 `IOError`。

**Notes:**

- 直接使用 Rust 的 `std::fs::File` — I/O 期间无 GIL 阻塞。
- 可迭代对象中的每个项目成为单独的 YAML 文档。
- 使用常量内存，适合大输出。

**Example:**

```python
from pyrs_yaml import YAML

yaml = YAML()
yaml.dump_file("output.yaml", [{"x": 2}, {"x": 3}], sort_keys=True)
```

#### `load_stream()`

惰性事件迭代器：从类文件对象增量读取。

```python
load_stream(file_obj: Any) -> YamlStream
```

**Parameters:**

| Parameter | Type | 描述 |
|-----------|------|-------------|
| `file_obj` | `Any` | 具有返回 `str` 或 `bytes` 的 `read()` 方法的可读类文件对象。 |

**Returns:** 延迟生成解析事件字典的 `YamlStream` 迭代器。

**Raises:** 如果 `file_obj` 没有 `read` 方法，抛出 `YamlTypeError`。

**Notes:**

- 流式增量解析 — 无需将整个文件加载到内存中。
- 每个生成的事件是一个包含 `"type"`、`"key"`、`"value"`、`"start_mark"`、`"end_mark"` 等键的 `dict`。
- 当 `__next__` 返回 `None` 时流结束。

**Example:**

```python
import io
from pyrs_yaml import YAML

yaml = YAML()
buf = io.StringIO("key: value\n")
stream = yaml.load_stream(buf)
for event in stream:
    if event is None:
        break
    print(event["type"])
```

#### `load_stream_file()`

惰性事件迭代器：从文件路径增量读取。

```python
load_stream_file(path: str) -> YamlStream
```

**Parameters:**

| Parameter | Type | 描述 |
|-----------|------|-------------|
| `path` | `str` | 要增量读取的文件路径。 |

**Returns:** 延迟生成解析事件字典的 `YamlStream` 迭代器。

**Raises:** 如果文件无法打开，抛出 `IOError`。

**Notes:**

- 使用 Rust 的 `std::fs::File` 配合缓冲 I/O — 读取期间无 GIL 阻塞。
- 增量解析文件，适合大型 YAML 文件。

**Example:**

```python
from pyrs_yaml import YAML

yaml = YAML()
stream = yaml.load_stream_file("large.yaml")
for event in stream:
    if event is None:
        break
    print(event)
```

### 使用示例

#### 使用配置实例进行往返编辑

```python
from pyrs_yaml import YAML

yaml = YAML(typ="rt", schema="core")
doc = yaml.parse("""
# User configuration
user:
  name: Alice
  age: 30
  tags: [admin, user]
""")

# 编辑文档
doc["user"]["age"] = 31
doc["user"]["tags"].append("staff")

# 序列化输出 — 注释和格式被保留
print(doc.to_yaml())
```

#### 使用 JSON 模式进行安全解析

```python
from pyrs_yaml import YAML

yaml = YAML(typ="safe", schema="json")
data = yaml.safe_load("{name: Bob, age: 25}")
print(data["name"])  # Bob
```

#### 多文档流处理

```python
from pyrs_yaml import YAML

yaml = YAML(typ="rt")
docs = yaml.parse_all_docs("""
---
doc: first
---
doc: second
""")
for doc in docs:
    print(doc["doc"])

# 或转储多个文档
yaml.dump_file("multi.yaml", [{"id": 1}, {"id": 2}], explicit_start=True)
```

### 另请参阅

- [`YamlDocument`](yaml-document.md) — 支持往返编辑的文档对象
- [`YamlStream`](reference.md#yamlstream) — 惰性事件流迭代器
- [`parse()`](reference.md#parse) — 模块级便捷函数
- [`safe_load()`](reference.md#safe_load) — 模块级便捷函数
