---
title: 模块参考
description: pyrs-yaml 模块的完整 API 参考文档，涵盖核心函数、PyYAML 兼容函数、转换函数、Pydantic 集成等。
tags:
  - docs
status: new
---

`pyrs_yaml` 模块的完整 API 参考。

!!! tip "版本兼容"
    pyrs-yaml 以 ABI3 wheel 格式构建，单个 wheel 支持 Python 3.8 到 3.15，无需重新编译。

## :material-code-braces: 核心函数

### `parse()`

将 YAML 字符串或字节解析为 `YamlDocument`。

```python
parse(yaml: str | bytes, resolve_merges: bool = True, schema: str | dict = "core", max_depth: int = 1000, allow_duplicate_keys: bool = False) -> YamlDocument
```

**参数:**

- `yaml` — `str` 或 `bytes` 的 YAML 内容
- `resolve_merges` — 解析后是否解析合并键 (`<<: *alias`)（默认：`True`）
- `schema` — Schema 名称 (`"core"`, `"json"`, `"failsafe"`, `"yaml1.1"` 或已注册的自定义名称)，或内联 schema dict（参见 [YAML Schema Language](#yaml-schema-language)）
- `max_depth` — 最大嵌套深度（默认：`1000`）
- `allow_duplicate_keys` — 是否允许重复映射键（默认：`False`）

**返回值:** 包含解析后 YAML 的 `YamlDocument`

**引发:**

- `YamlParseError` — 无效的 YAML 语法
- `YamlTypeError` — 未找到指定的 Schema
- `TypeError` — 输入不是 `str` 或 `bytes`

**示例:**

```python
doc = pyrs_yaml.parse("key: value")
doc = pyrs_yaml.parse(b"key: value")
doc = pyrs_yaml.parse(yaml_str, schema="json")
doc = pyrs_yaml.parse("addr: 0xFF", schema={"extends": "core", "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}]})
```

### `parse_file()`

解析 YAML 文件。

```python
parse_file(path: str, schema: str | dict = "core", max_depth: int = 1000, allow_duplicate_keys: bool = False) -> YamlDocument
```

**参数:**

- `path` — YAML 文件的路径
- `schema` — Schema 名称或内联 dict（默认：`"core"`）
- `max_depth` — 最大嵌套深度（默认：`1000`）
- `allow_duplicate_keys` — 是否允许重复映射键（默认：`False`）

**返回值:** `YamlDocument`

**引发:**

- `IOError` — 文件未找到或无法读取
- `YamlParseError` — 无效的 YAML

**示例:**

```python
doc = pyrs_yaml.parse_file("config.yaml")
```

### `parse_all_docs()`

从字符串解析多个 YAML 文档。

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**返回值:** `YamlDocument` 对象列表

**示例:**

```python
docs = pyrs_yaml.parse_all_docs("a: 1\n---\nb: 2")
```

## :material-swap-horizontal: PyYAML 兼容函数

### `safe_load()`

解析 YAML 并返回原生 Python 类型。

```python
safe_load(yaml: str) -> dict[str, Any] | list[Any]
```

**等价于:** PyYAML 的 `yaml.safe_load()`

**示例:**

```python
data = pyrs_yaml.safe_load("key: value")  # {'key': 'value'}
```

### `safe_loads()`

解析多个 YAML 文档。

```python
safe_loads(yaml: str) -> list[dict[str, Any] | list[Any]]
```

**等价于:** PyYAML 的 `yaml.safe_loads()`

### `safe_dump()`

将 Python 对象序列化为 YAML。

```python
safe_dump(data: dict[str, Any] | list[Any] | ndarray) -> str
```

**等价于:** PyYAML 的 `yaml.safe_dump()`

**支持的输入类型:** `dict`, `list`, `str`, `int`, `float`, `bool`, `None`，以及 **`numpy.ndarray`**（所有维度和数值 dtype：`int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`）

### `safe_dumps()`

`safe_dump()` 的别名。

```python
safe_dumps(data: dict[str, Any] | list[Any] | ndarray) -> str
```

## :material-json: 转换函数

### `from_dict()`

将 Python dict 转换为 YAML 字符串。dict 的值中可以包含 `numpy.ndarray`。

```python
from_dict(data: dict[str, Any]) -> str
```

### `from_json()`

将 JSON 字符串转换为 YAML 字符串。

```python
from_json(json_str: str) -> str
```

### `dump_file()`

将 Python 对象序列化为 YAML 并写入文件。接受 `dict`、`list` 或 `numpy.ndarray`。

```python
dump_file(data: Any, path: str) -> None
```

## :material-pillar: Pydantic 集成

### `dump_pydantic()`

将 Pydantic 模型序列化为 YAML 字符串。

```python
dump_pydantic(model: BaseModel) -> str
```

使用 `model_dump(mode='json')` 保持字符串类型（例如 `"10001"` 的邮政编码保持为字符串），然后委托给 `safe_dump`。

**引发:**

- `ImportError` — 未安装 pydantic
- `TypeError` — `model` 不是 Pydantic 的 `BaseModel` 实例

**示例:**

```python
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


yaml_str = pyrs_yaml.dump_pydantic(User(name="Alice", age=30))
```

### `parse_as()`

解析 YAML 字符串并针对 Pydantic 模型进行验证。

```python
parse_as(model: type[BaseModel], src: str, **yaml_kwargs: Any) -> BaseModel
```

**参数:**

- `model` — Pydantic 的 `BaseModel` 子类
- `src` — 要解析的 YAML 字符串
- `**yaml_kwargs` — 转发给 `YAML()` 构造函数的关键字参数

**引发:**

- `ImportError` — 未安装 pydantic
- `TypeError` — `model` 不是 Pydantic 的 `BaseModel` 子类
- `pydantic.ValidationError` — 解析的数据未通过模型验证

**示例:**

```python
user = pyrs_yaml.parse_as(User, "name: Alice\nage: 30")
print(user.name)  # Alice
```

## :material-tag: 标签注册表

### `register_tag()`

注册自定义标签处理器。支持装饰器和命令式两种形式。

```python
register_tag(name: str, handler: Callable | None = None, priority: int = 0) -> Callable
```

=== "装饰器"

    ```python
    @pyrs_yaml.register_tag("!custom")
    def handler(node):
        return f"custom:{node}"
    ```

=== "命令式"

    ```python
    pyrs_yaml.register_tag("!custom", handler_fn, priority=1)
    ```

### `remove_tag()`

移除标签处理器。

```python
remove_tag(name: str) -> None
```

### `clear_tag_handlers()`

移除所有已注册的标签处理器。

```python
clear_tag_handlers() -> None
```

## :material-file-document: YAML Schema Language {#yaml-schema-language}

定义自定义 Schema，控制纯标量如何解析为 Python 类型。

### `register_schema()`

注册一个自定义 Schema。

```python
register_schema(name: str, schema: str | dict) -> None
```

**参数:**

- `name` — Schema 名称
- `schema` — YAML 字符串或 dict（包含 `extends`、`rules`、`validate` 键）

**示例:**

```python
import pyrs_yaml

# 从 YAML 字符串注册自定义 Schema
pyrs_yaml.register_schema("hex", """
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
""")

# 使用自定义 Schema
y = pyrs_yaml.YAML(schema="hex")
doc = y.parse("addr: 0xFF")
assert doc.get("addr") == 255

d = pyrs_yaml.safe_load("addr: 0x1F", schema="hex")
assert d["addr"] == 31
```

### 内联 Schema dict

直接传入 dict 代替注册：

```python
d = pyrs_yaml.safe_load(
    "addr: 0xFF",
    schema={
        "extends": "core",
        "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}],
    },
)
```

- **`extends`** — 可选的基 Schema（`core`、`json`、`failsafe`、`yaml1.1`）
- **`rules`** — 有序的 `{pattern, type}` 列表；首个匹配生效
- **`validate`** — 可选的结构校验规则：路径限定类型（`$.port: int`）、容器检查（`sequence_of`、`mapping_of`）和 `required` 存在性检查；使用 `validate_against_schema(data, schema_yaml)` 校验文档
- **支持的类型**：`null`、`bool`、`int`、`float`、`str`
- 内置 Core Schema 仍使用零成本 `match` 分发（不受影响）
- **文件 I/O** — `load_schema(name, path)` 从 YAML 文件加载 Schema；`list_schemas()` 返回所有已注册的 Schema

## :material-puzzle: 社区插件 {#community-plugins}

定义自定义 YAML 节点类型，集成序列化和反序列化。

### `CustomType`

自定义类型的基类。

```python
class CustomType:
    python_type: type

    def from_yaml(self, value: str) -> Any: ...
    def to_yaml(self, obj: Any) -> str: ...
    def can_parse(self, node: CustomNode) -> bool: ...
    def validate(self, obj: Any) -> bool: ...
```

### `register_type()`

注册自定义类型。

```python
register_type(tag: str, type_handler: CustomType, priority: int = 0) -> None
```

**示例:**

```python
from datetime import datetime

class TimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value: str):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj) -> str:
        return obj.isoformat()

pyrs_yaml.register_type("!timestamp", TimestampType())

# 加载：带标签的标量 → Python 对象
doc = pyrs_yaml.parse("when: !timestamp 2026-08-11T10:30:00")
assert isinstance(doc.get("when"), datetime)

# 导出：Python 对象 → 带标签的标量
data = {"ts": datetime(2026, 8, 11, 10, 30)}
out = pyrs_yaml.safe_dump(data)
# out 包含：ts: !timestamp 2026-08-11T10:30:00
```

| 方法 | 描述 |
|------|------|
| `can_parse(node)` | 该类型是否处理给定的 AST 节点 |
| `from_yaml(value)` | 将 YAML 字符串转换为 Python 对象 |
| `to_yaml(obj)` | 将 Python 对象转换为 YAML 字符串 |
| `validate(obj)` | 校验 Python 对象（返回 `bool`） |

### `remove_type()`

移除已注册的类型。

```python
remove_type(name: str) -> None
```

### `clear_type_handlers()`

移除所有已注册的类型处理器。

```python
clear_type_handlers() -> None
```

## :material-check-decagram: 合规性

### `compliance_report()`

计算 YAML 测试套件的合规性报告。

```python
compliance_report() -> dict
```

返回 YAML 测试套件的通过率及每个测试的结果。

## :material-wave: 流式事件

### `parse_stream()`

增量解析 YAML，产出原始事件字典。

```python
parse_stream(yaml: str) -> StreamIterator
```

返回一个 `StreamIterator`，每步产出一个事件字典。与 `YAML().load_stream()`（解析为 Python 值）不同，这暴露了原始令牌流。

### `YamlStream` { #yamlstream }

`YamlStream` 类是一个惰性事件迭代器，由 `YAML().load_stream()` 和 `YAML().load_stream_file()` 返回。它逐个产出解析后的事件字典，无需将整个文档加载到内存中。

```python
stream = yaml.load_stream_file("large.yaml")
for event in stream:
    print(event)
```

参见 [`YamlStream`](yaml-instance.md) 了解完整的 API 详情。

## :material-clock-fast: 异步函数

使用 `asyncio.run_in_executor` 的异步 I/O 包装器。在事件循环上下文中不阻塞。

### `safe_dumps_async()`

将 Python 对象序列化为 YAML 字符串（异步）。

```python
async def safe_dumps_async(data: Any) -> str
```

### `safe_dump_async()`

将 Python 对象以 YAML 格式输出到 stdout（异步）。

```python
async def safe_dump_async(data: Any) -> None
```

### `safe_loads_async()`

将 YAML 字符串解析为原生 Python 对象（异步）。

```python
async def safe_loads_async(yaml: str, schema: str = "core") -> Any
```

### `safe_load_async()`

将 YAML 字符串解析为原生 Python 对象（异步）。

```python
async def safe_load_async(yaml: str, schema: str = "core") -> Any
```

**示例:**

```python
import asyncio, pyrs_yaml


async def main():
    yaml = await pyrs_yaml.safe_dumps_async({"a": 1})
    data = await pyrs_yaml.safe_loads_async(yaml)
    print(data)  # {'a': 1}


asyncio.run(main())
```

## :material-page-layout-body: Markdown Front Matter

### `read_markdown()`

从 Markdown 文件提取 YAML Front Matter。

```python
read_markdown(path: str, schema: str = "core", max_depth: int = 1000) -> tuple[dict[str, Any] | None, str]
```

**返回值:** `(frontmatter_dict, content_string)`。没有Front Matter时，`frontmatter` 为 `None`。

### `read_markdown_str()`

从 Markdown 字符串提取 YAML Front Matter。

```python
read_markdown_str(content: str, schema: str = "core", max_depth: int = 1000) -> tuple[dict[str, Any] | None, str]
```

## :material-translate: i18n 函数

### `set_language()`

设置错误消息的语言。

```python
set_language(lang: str) -> None
```

支持：`"en"`, `"zh-CN"`, `"ja-JP"`, `"ko-KR"`

### `get_language()`

获取当前语言。

```python
get_language() -> str
```

### `list_languages()`

列出所有支持的语言。

```python
list_languages() -> list[str]
```

### `detect_language()`

从环境变量自动检测用户的首选语言。

```python
detect_language() -> str
```

### `negotiate_language()`

BCP 47 语言协商。

```python
negotiate_language(user_locales: list[str], default: str = "en") -> str
```

## :material-bug: 异常

- `YamlParseError` — YAML 解析错误（继承自 `ValueError`）
- `YamlSerializeError` — YAML 序列化错误（继承自 `ValueError`）
- `YamlTypeError` — 类型转换错误（继承自 `TypeError`）
- `YamlValidateError` — JSON Schema 验证错误（继承自 `ValueError`）
- `YamlEditError` — 就地编辑错误（继承自 `ValueError`）
- `YamlPathError` — YAML 路径错误（继承自 `ValueError`）
- `YamlDocumentError` — 过期的 `Node` 访问错误（继承自 `Exception`）

参见 [异常](exceptions.md) 页面了解完整详情。

## :material-information: 版本

```python
__version__ = "0.14.0"
```
