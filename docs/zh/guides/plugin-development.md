---
title: 插件开发
description: 使用 Community Plugins API 为 pyrs-yaml 创建第三方插件。
tags:
  - docs
status: new
---

## 插件开发

本指南说明如何使用 Community Plugins API 创建 pyrs-yaml 第三方插件。

### 插件结构

一个插件是一个 Python 模块，定义 `CustomType` 子类并注册它。

```python title="my_timestamp_plugin.py"
import pyrs_yaml
from datetime import datetime


class MyTimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj):
        return obj.isoformat()


def register():
    pyrs_yaml.register_type("!mytimestamp", MyTimestampType())
```

### 入口点自动发现

在 `pyproject.toml` 中添加入口点：

```toml title="pyproject.toml"
[project.entry-points."pyrs_yaml.plugins"]
mytimestamp = "my_timestamp_plugin:register"
```

### API 参考

| 函数 | 说明 |
|---------|------|
| :material-code-braces: `register_type(name, handler)` | 注册 `CustomType` 实例 |
| :material-close: `clear_type_handlers()` | 移除所有已注册类型 |
| :material-close: `remove_type(name)` | 移除特定类型 |
| :material-check-decagram: `validate_custom_types(obj)` | 验证对象是否通过所有注册类型的校验 |

---

### 另请参阅

- [社区插件](community-plugins.md) — 可扩展的内置类型
- [自定义 Schema](custom-schema.md) — 定义类型解析规则
- [标签注册 API](../api/reference.md#tag-registry) — `register_tag()` 及相关函数
