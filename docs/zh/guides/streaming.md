---
title: 流式解析
description: 使用 pyrs-yaml 进行流式解析和流式写入的指南，支持常量内存处理大文件。
tags:
  - docs
status: new
---

!!! note "流式解析与完整解析"
    流式解析以惰性事件迭代方式处理 YAML，内存用量为 O(锚点数 + 64KB 块)，与输入大小无关，适用于 100MB+ 的大文件。完整解析则将整个文档加载到内存中构建 AST。

`YAML.load_stream(file_obj)` 和 `YAML.load_stream_file(path)` 惰性迭代 YAML 事件——内存用量为 O(锚点数 + 64KB 块)，与输入大小无关。适用于 100MB+ 的文件。

```python title="加载流"
from pyrs_yaml import YAML

for event in YAML().load_stream_file("huge.yaml"):
    print(event["type"], event["value"])
```

## 工作原理

```mermaid
graph LR
    A["YAML 文件 / 字符串"] --> B["惰性事件迭代器<br/>O(锚点数 + 64KB 块)"]
    B --> C["事件字典<br/>type, value, style, anchor, tag, line, column"]
    C --> D["消费者<br/>流式处理"]
```

## 与 parse_stream 的差异

| 行为 | load_stream | parse_stream |
| --- | --- | --- |
| 内存 | O(锚点数 + 块) | O(输入) |
| 注释 | :material-close: 不产出 | :material-check: 产出 |
| 锚点名称 | `anchor_{id}` | 原始名称 |
| 错误消息 | 无源码片段 | 有源码片段 |
| 空输入 | `[stream_start, stream_end]` | `[]` |
| 标签处理器 | 不应用 | 应用 (YAML.parse) |

## 资源管理

提前停止时调用 `close()`——这是唯一保证的释放点（PyPy 的延迟 GC 不保证 `Drop` 时机）。`close()` 是幂等的，且**不会**关闭你传入的文件对象。

## 流式写出

`YAML().dump_stream(file_obj, iterable, ...)` 和 `YAML().dump_file(path, iterable, ...)`
逐文档序列化，使用常量内存（O(单文档 + 64KB 块)），与文档总数无关。

```python title="写出流"
from pyrs_yaml import YAML

buf = io.StringIO()
YAML().dump_stream(buf, [{"a": 1}, {"b": 2}])
# buf.getvalue() == "a: 1\n---\nb: 2\n"
```

### 分隔符规则

- 首文档前无 `---`；后续每个文档前自动添加 `---`。
- `explicit_start=True` 在首文档前添加 `---`。
- `explicit_end=True` 在末文档后添加 `...`。
- 空 iterable 输出 0 字节。

#### 错误语义

中途失败（迭代器异常、序列化错误、写失败）时，已写出的文档保留在目标中，不做回滚。

#### 与 safe_dump 的差异

| 方面 | dump_stream / dump_file | safe_dump |
|------|------------------------|-----------|
| 输出 | 多文档流 | 单文档 |
| 内存 | O(单文档 + 64KB) | O(输入) |
| 项类型 | `YamlDocument`（保留注释/锚点）或普通 Python 对象 | 单个 Python 对象 |

#### 排序键

传递 `sort_keys=True` 以按排序顺序输出映射键，与 `safe_dump` 的 `sort_keys` 行为一致。

## StreamIterator

`StreamIterator` 类由 `parse_stream()` 和 `YAML().load_stream()` / `YAML().load_stream_file()` 产出。它实现迭代器协议，一次产出一个事件字典。

```python title="迭代事件"
from pyrs_yaml import parse_stream

iterator = parse_stream("key: value\n---\na: 1")
for event in iterator:
    print(event["type"], event["value"])
```

### 迭代器协议

`StreamIterator` 实现 `__iter__`（返回 `self`）和 `__next__`：

```python title="迭代器协议"
def __iter__() -> StreamIterator: ...
def __next__() -> dict | None: ...
```

当流被耗尽时，`__next__()` 返回 `None`（不会抛出 `StopIteration`）。

#### 事件字典键

| 键 | 类型 | 描述 |
| --- | --- | --- |
| `type` | `str` | 事件类型（见下文） |
| `value` | `str` 或 `None` | 标量值、别名名称或注释文本 |
| `style` | `str` 或 `None` | 标量引号样式：`"plain"`、`"single_quoted"`、`"double_quoted"`、`"literal"`、`"folded"`；注释为 `"standalone"` 或 `"inline"` |
| `anchor` | `str` 或 `None` | 锚点名称（`&name`） |
| `tag` | `str` 或 `None` | 标签字符串（`!!str`、`!custom`） |
| `line` | `int` | 行号（0 起始） |
| `column` | `int` | 列号（0 起始） |

#### 事件类型

| `type` | 何时产出 |
| --- | --- |
| `stream_start` | YAML 流的开始 |
| `stream_end` | 流的结束 |
| `document_start` | 文档的开始 |
| `document_end` | 文档的结束 |
| `mapping_start` | 映射的开始 |
| `mapping_end` | 映射的结束 |
| `sequence_start` | 序列的开始 |
| `sequence_end` | 序列的结束 |
| `scalar` | 标量值 |
| `alias` | 别名引用（`*name`） |
| `comment` | YAML 注释 |

#### 与 `load_stream` 的差异

`parse_stream()` 返回一个产出注释并保留原始锚点名称的 `StreamIterator`。`YAML().load_stream()` / `YAML().load_stream_file()` 返回具有不同默认值的 `YamlStream`（见上面的对照表）。
