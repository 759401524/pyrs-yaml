---
title: 社区插件
description: 使用 Community Plugins API 扩展 pyrs-yaml，注册自定义节点类型。
tags:
  - docs
status: new
---

## 社区插件

Community Plugins API 允许您定义自定义 YAML 节点类型，与 pyrs-yaml 的序列化和反序列化集成。
自定义类型可以将 YAML 标签标量与任意 Python 对象互相转换。

### 内置插件

pyrs-yaml 内置了在导入时自动注册的插件：

| 标签 | Python 类型 | 说明 |
|-----|-------------|------|
| `!timestamp` | `datetime` | ISO 8601 时间戳往返 |
| `!set` | `str` | YAML 集合（无键映射） |

### 创建自定义类型

继承 `CustomType` 并实现 `from_yaml()` 和 `to_yaml()`：

```python
import pyrs_yaml
from datetime import datetime


class TimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value: str):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj) -> str:
        return obj.isoformat()
```

**`python_type`** — 可选的类型属性，用于序列化时的 `isinstance` 检查。

### 注册

**命令式：**

```python
pyrs_yaml.register_type("!timestamp", TimestampType())
```

**装饰器形式：**

```python
@pyrs_yaml.register_type("!timestamp")
class TimestampType(pyrs_yaml.CustomType):
    ...
```

### 使用

**加载标签标量：**

```python
doc = pyrs_yaml.parse("when: !timestamp 2026-08-11T10:30:00")
val = doc.get("when")
assert isinstance(val, datetime)
```

**转储 Python 对象：**

```python
data = {"ts": datetime(2026, 8, 11, 10, 30)}
out = pyrs_yaml.safe_dump(data)
# out 包含: ts: !timestamp 2026-08-11T10:30:00
```

### API 参考

| 方法 | 说明 |
|--------|------|
| `can_parse(node)` | 此类型是否处理给定 AST 节点 |
| `from_yaml(value)` | 将 YAML 字符串转换为 Python 对象 |
| `to_yaml(obj)` | 将 Python 对象转换为 YAML 字符串 |
| `validate(obj)` | 验证 Python 对象（返回 `bool`） |

### 示例：UUID 类型

```python
import uuid
import pyrs_yaml


class UUIDType(pyrs_yaml.CustomType):
    python_type = uuid.UUID

    def from_yaml(self, value):
        return uuid.UUID(value)

    def to_yaml(self, obj):
        return str(obj)


pyrs_yaml.register_type("!uuid", UUIDType())

doc = pyrs_yaml.parse("id: !uuid 550e8400-e29b-41d4-a716-446655440000")
assert isinstance(doc.get("id"), uuid.UUID)
```
