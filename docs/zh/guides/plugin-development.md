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

### 统一 TagHandler API

`TagHandler` 将标签注册表（`register_tag`）与社区插件（`register_type`）统一到一个注册点。处理器可任意组合标量、容器与生命周期方法。

```python
class HexInt(pyrs_yaml.TagHandler):
    python_type = int

    def from_yaml(self, value, tag):
        return int(value, 16)

    def to_yaml(self, obj, tag):
        return hex(obj)


pyrs_yaml.register_tag_handler("!hex", HexInt())
```

容器标签（`from_yaml_mapping`/`from_yaml_sequence`/`to_yaml_mapping`/`to_yaml_sequence`）仅在 AST 往返路径生效；`safe_dump` 快速路径始终输出普通容器。

### API 参考

| 函数 | 说明 |
|---------|------|
| :material-code-braces: `register_type(name, handler)` | 注册 `CustomType` 实例 |
| :material-close: `clear_type_handlers()` | 移除所有已注册类型 |
| :material-close: `remove_type(name)` | 移除特定类型 |
| :material-check-decagram: `validate_custom_types(obj)` | 验证对象是否通过所有注册类型的校验 |
| :material-link-variant: `register_tag_handler(name, handler)` | 注册统一的 `TagHandler` |
| :material-close: `clear_tag_handlers()` | 移除所有标签处理器 |
| :material-close: `remove_tag_handler(name)` | 移除指定标签的处理器 |
| :material-code-braces: `list_tag_handlers()` | 列出所有已注册标签处理器 |
| :material-code-braces: `get_tag_handler(name)` | 获取标签的处理器实例 |
| :material-export: `serialize(doc, ...)` | 带生命周期钩子的序列化 |

### 从 CustomType 迁移到 TagHandler

`CustomType`、`register_type` 和 `register_tag` 现已**弃用**，但仍可正常使用（会发出 `DeprecationWarning`）。`register_type("!x", handler)` 会自动将 `CustomType` 包装为 `TagHandler`，因此现有注册代码无需修改。

**API 对照：**

| 旧 API | 新 API |
|---------|--------|
| `register_type("!x", T())` | `register_tag_handler("!x", THandler())` |
| `from_yaml(value)` | `from_yaml(value, tag)` |
| `to_yaml(obj)` | `to_yaml(obj, tag)` |
| `can_parse` / `validate` / `python_type` | 保持不变 |
| — | `resolve` / `from_yaml_mapping` / `to_yaml_mapping` / 生命周期钩子（TagHandler 新增） |

!!! note "容器类型"
    若标签作用于 Mapping/Sequence（如 `!set`），需在 `TagHandler` 中实现 `from_yaml_mapping` / `to_yaml_mapping`，而非标量 `from_yaml`。

---

### 另请参阅

- [社区插件](community-plugins.md) — 可扩展的内置类型
- [自定义 Schema](custom-schema.md) — 定义类型解析规则
- [标签注册 API](../api/reference.md#tag-registry) — `register_tag()` 及相关函数
