# 流式解析

`YAML.load_stream(file_obj)` 和 `YAML.load_stream_file(path)` 惰性迭代 YAML 事件——内存用量为 O(锚点数 + 64KB 块)，与输入大小无关。适用于 100MB+ 的文件。

```python
from pyrs_yaml import YAML

for event in YAML().load_stream_file("huge.yaml"):
    print(event["type"], event["value"])
```

## 与 parse_stream 的差异

| 行为 | load_stream | parse_stream |
| --- | --- | --- |
| 内存 | O(锚点数 + 块) | O(输入) |
| 注释 | 不产出 | 产出 |
| 锚点名称 | `anchor_{id}` | 原始名称 |
| 错误消息 | 无源码片段 | 有源码片段 |
| 空输入 | `[stream_start, stream_end]` | `[]` |
| 标签处理器 | 不应用 | 应用 (YAML.parse) |

## 资源管理

提前停止时调用 `close()`——这是唯一保证的释放点（PyPy 的延迟 GC 不保证 `Drop` 时机）。`close()` 是幂等的，且**不会**关闭你传入的文件对象。

## 流式写出

`YAML().dump_stream(file_obj, iterable, ...)` 和 `YAML().dump_file(path, iterable, ...)`
逐文档序列化，使用常量内存（O(单文档 + 64KB 块)），与文档总数无关。

```python
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

### 错误语义

中途失败（迭代器异常、序列化错误、写失败）时，已写出的文档保留在目标中，不做回滚。

### 与 safe_dump 的差异

| 方面 | dump_stream / dump_file | safe_dump |
|------|------------------------|-----------|
| 输出 | 多文档流 | 单文档 |
| 内存 | O(单文档 + 64KB) | O(输入) |
| 项类型 | `YamlDocument`（保留注释/锚点）或普通 Python 对象 | 单个 Python 对象 |

### 排序键

传递 `sort_keys=True` 以按排序顺序输出映射键，与 `safe_dump` 的 `sort_keys` 行为一致。
