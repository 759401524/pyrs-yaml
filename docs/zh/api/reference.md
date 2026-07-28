---

title: 模块参考
lang: zh

## 模块参考

`pyyaml_rs` 模块的完整 API 参考。

### 核心函数

#### `parse()`

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
doc = pyyaml_rs.parse("key: value")
doc = pyyaml_rs.parse(b"key: value")
doc = pyyaml_rs.parse(yaml_str, resolve_merges=False)
```

#### `parse_file()`

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
doc = pyyaml_rs.parse_file("config.yaml")
```

#### `parse_all_docs()`

从字符串解析多个 YAML 文档。

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**返回值:** `YamlDocument` 对象列表

**示例:**

```python
docs = pyyaml_rs.parse_all_docs("a: 1\n---\nb: 2")
```

### PyYAML 兼容函数

#### `safe_load()`

解析 YAML 并返回原生 Python 类型。

```python
safe_load(yaml: str) -> dict[str, Any] | list[Any]
```

**等价于:** PyYAML 的 `yaml.safe_load()`

**示例:**

```python
data = pyyaml_rs.safe_load("key: value")  # {'key': 'value'}
```

#### `safe_loads()`

解析多个 YAML 文档。

```python
safe_loads(yaml: str) -> list[dict[str, Any] | list[Any]]
```

**等价于:** PyYAML 的 `yaml.safe_loads()`

#### `safe_dump()`

将 Python 对象序列化为 YAML。

```python
safe_dump(data: dict[str, Any] | list[Any] | ndarray) -> str
```

**等价于:** PyYAML 的 `yaml.safe_dump()`

**支持的输入类型:** `dict`, `list`, `str`, `int`, `float`, `bool`, `None`，以及 **`numpy.ndarray`**（所有维度和数值 dtype：`int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`）

#### `safe_dumps()`

`safe_dump()` 的别名。

```python
safe_dumps(data: dict[str, Any] | list[Any] | ndarray) -> str
```

### 转换函数

#### `from_dict()`

将 Python dict 转换为 YAML 字符串。dict 的值中可以包含 `numpy.ndarray`。

```python
from_dict(data: dict[str, Any]) -> str
```

#### `from_json()`

将 JSON 字符串转换为 YAML 字符串。

```python
from_json(json_str: str) -> str
```

#### `dump_file()`

将 Python 对象序列化为 YAML 并写入文件。接受 `dict`、`list` 或 `numpy.ndarray`。

```python
dump_file(data: Any, path: str) -> None
```

### 异步函数

使用 `asyncio.run_in_executor` 的异步 I/O 包装器。在事件循环上下文中不阻塞。

#### `safe_dumps_async()`

将 Python 对象序列化为 YAML 字符串（异步）。

```python
async def safe_dumps_async(data: Any) -> str
```

#### `safe_dump_async()`

将 Python 对象以 YAML 格式输出到 stdout（异步）。

```python
async def safe_dump_async(data: Any) -> None
```

#### `safe_loads_async()`

将 YAML 字符串解析为原生 Python 对象（异步）。

```python
async def safe_loads_async(yaml: str, schema: str = "core") -> Any
```

#### `safe_load_async()`

将 YAML 字符串解析为原生 Python 对象（异步）。

```python
async def safe_load_async(yaml: str, schema: str = "core") -> Any
```

**示例:**

```python
import asyncio, pyyaml_rs

async def main():
    yaml = await pyyaml_rs.safe_dumps_async({"a": 1})
    data = await pyyaml_rs.safe_loads_async(yaml)
    print(data)  # {'a': 1}

asyncio.run(main())
```

### Markdown 前端元数据

#### `read_markdown()`

从 Markdown 文件提取 YAML 前端元数据。

```python
read_markdown(path: str) -> tuple[dict[str, Any] | None, str]
```

**返回值:** `(frontmatter_dict, content_string)`。没有前端元数据时，`frontmatter` 为 `None`。

#### `read_markdown_str()`

从 Markdown 字符串提取 YAML 前端元数据。

```python
read_markdown_str(content: str) -> tuple[dict[str, Any] | None, str]
```

### i18n 函数

#### `set_language()`

设置错误消息的语言。

```python
set_language(lang: str) -> None
```

支持：`"en"`, `"zh-CN"`

#### `get_language()`

获取当前语言。

```python
get_language() -> str
```

#### `list_languages()`

列出所有支持的语言。

```python
list_languages() -> list[str]
```

#### `detect_language()`

从环境变量自动检测用户的首选语言。

```python
detect_language() -> str
```

#### `negotiate_language()`

BCP 47 语言协商。

```python
negotiate_language(user_locales: list[str], default: str = "en") -> str
```

### 异常

- `YamlParseError` — YAML 解析错误（继承自 `ValueError`）
- `YamlSerializeError` — YAML 序列化错误（继承自 `ValueError`）
- `YamlTypeError` — 类型转换错误（继承自 `TypeError`）
- `YamlValidateError` — JSON Schema 验证错误（继承自 `ValueError`）

### 版本

```python
__version__ = "0.6.0"
```
