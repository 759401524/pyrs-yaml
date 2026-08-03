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
