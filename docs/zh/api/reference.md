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

## 核心函数

### `parse()`

将 YAML 字符串或字节解析为 `YamlDocument`。

```python
parse(yaml: str | bytes, resolve_merges: bool = True) -> YamlDocument
```

**参数:**

- `yaml` — `str` 或 `bytes` 的 YAML 内容
- `resolve_merges` — 解析后是否解析合并键 (`<<: *alias`)（默认：`True`）

**返回值:** 包含解析后 YAML 的 `YamlDocument`

**引发:**

- `YamlParseError` — 无效的 YAML 语法
- `TypeError` — 输入不是 `str` 或 `bytes`

**示例:**

```python
doc = pyrs_yaml.parse("key: value")
doc = pyrs_yaml.parse(b"key: value")
doc = pyrs_yaml.parse(yaml_str, resolve_merges=False)
```

### `parse_file()`

解析 YAML 文件。

```python
parse_file(path: str) -> YamlDocument
```

**参数:**

- `path` — YAML 文件的路径

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

## PyYAML 兼容函数

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

## 转换函数

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

## Pydantic 集成

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

## 标签注册表

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

## 合规性

### `compliance_report()`

计算 YAML 测试套件的合规性报告。

```python
compliance_report() -> dict
```

返回 YAML 测试套件的通过率及每个测试的结果。

## 流式事件

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

## 异步函数

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

## Markdown Front Matter

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

## i18n 函数

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

## 异常

- `YamlParseError` — YAML 解析错误（继承自 `ValueError`）
- `YamlSerializeError` — YAML 序列化错误（继承自 `ValueError`）
- `YamlTypeError` — 类型转换错误（继承自 `TypeError`）
- `YamlValidateError` — JSON Schema 验证错误（继承自 `ValueError`）
- `YamlEditError` — 就地编辑错误（继承自 `ValueError`）
- `YamlPathError` — YAML 路径错误（继承自 `ValueError`）
- `YamlDocumentError` — 过期的 `Node` 访问错误（继承自 `Exception`）

参见 [异常](exceptions.md) 页面了解完整详情。

## 版本

```python
__version__ = "0.6.0"
```
