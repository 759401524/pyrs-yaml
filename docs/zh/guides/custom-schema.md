---
title: 自定义 Schema
description: 定义和使用自定义 YAML schema 来控制类型解析行为。
tags:
  - docs
status: new
---

## 自定义 Schema

默认情况下，pyrs-yaml 使用 YAML 1.2 Core schema 进行隐式类型解析。通过 YAML Schema Language，
您可以定义自定义 schema，控制 plain scalar 如何解析为 Python 类型。

### 为何需要自定义 Schema？

Core schema 会将 `0xFF` 解析为 `int(255)`，`2026-08-11` 解析为 `int(2026)`，
`hello` 解析为 `"hello"`。有时您需要不同的行为：

- 将日期保留为字符串（`"2026-08-11"` 而非 `2026`）
- 将十六进制/二进制字面量解析为整数
- 添加 YAML 1.1 风格的布尔词法（`yes`/`no`）
- 使用纯 JSON 子集（不含 `inf`、`nan`、`0x`）

### Schema 定义格式

Schema 定义为一个包含 `rules` 列表的 YAML 文件。每条规则包含一个 `pattern`（正则表达式）
和一个 `type`（`null`、`bool`、`int`、`float`、`str` 之一）。

```yaml
# hex_schema.yaml
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
  - pattern: ^0b[01]+$
    type: int
```

**`extends`** — 可选的基础 schema。规则优先匹配；若无匹配则回退到 `extends` 指定的 schema。
默认值：`core`。

**`rules`** — 有序列表。首个匹配的 pattern 决定类型。支持的类型：

| `type` | Python 结果 | 示例 |
|--------|------------|------|
| `null` | `None` | `~` |
| `bool` | `True` / `False` | `true`, `yes`, `on` |
| `int` | `int` | `42`, `0xFF`, `0o77`, `0b1010` |
| `float` | `float` | `3.14`, `1e10` |
| `str` | `str` | `2026-08-11` |

### 注册和使用 Schema

```python
import pyrs_yaml

# 从 YAML 字符串注册
pyrs_yaml.register_schema("hex", """
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
""")

# 使用 YAML 实例
y = pyrs_yaml.YAML(schema="hex")
doc = y.parse("addr: 0xFF")
assert doc.get("addr") == 255

# 使用模块级函数
d = pyrs_yaml.safe_load("addr: 0x1F", schema="hex")
assert d["addr"] == 31
```

#### 从文件加载 schema

`load_schema()` 从文件路径读取 schema 定义并注册：

```python
# hex.yaml 包含上述 schema YAML
pyrs_yaml.load_schema("hex", "path/to/hex.yaml")
```

#### 列出已注册 schema

`list_schemas()` 返回所有已注册的 schema 名称（内置 + 自定义）：

```python
print(pyrs_yaml.list_schemas())
# ['failsafe', 'json', 'core', 'yaml1.1', 'hex', ...]
```

#### 结构化校验

在 schema 定义中添加 `validate` 段，可在标量类型解析之上进行结构检查。使用 `validate_against_schema()` 在文档使用前校验：

```yaml
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
  - path: $.tags[*]
    type: str
  - path: $.numbers
    sequence_of: int
  - path: $.config
    mapping_of: str
```

```python
import pyrs_yaml

schema = """\
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
"""

pyrs_yaml.validate_against_schema("port: 80\n", schema)          # OK
# 列出所有失败项并抛 YamlValidateError：
pyrs_yaml.validate_against_schema("port: abc\n", schema)
```

- `path` — JSONPath 风格位置（`$.key`、`$.a.b`、`$.tags[*]`）；省略时应用于所有标量
- `type` — 标量必须解析为该 YAML 类型（`null`/`bool`/`int`/`float`/`str`）
- `sequence_of` / `mapping_of` — 每个元素 / 值必须为指定类型
- `required` — 路径必须存在且非 null（可与 `type` 组合）

### 内联 Dict Schema

无需预先注册，直接传入字典：

```python
d = pyrs_yaml.safe_load(
    "addr: 0xFF",
    schema={
        "extends": "core",
        "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}],
    },
)
assert d["addr"] == 255
```

### 常见模式

#### 将日期保留为字符串

```python
schema = {
    "extends": "core",
    "rules": [{"pattern": "^\\d{4}-\\d{2}-\\d{2}$", "type": "str"}],
}
```

#### 添加 YAML 1.1 布尔值

```python
schema = {
    "extends": "core",
    "rules": [{"pattern": "^(yes|no|Yes|No|YES|NO)$", "type": "bool"}],
}
```

#### 严格 JSON 模式

```python
schema = {
    "extends": "failsafe",
    "rules": [
        {"pattern": "^null$|^~$", "type": "null"},
        {"pattern": "^(true|false)$", "type": "bool"},
        {"pattern": "^-?\\d+$", "type": "int"},
        {"pattern": "^-?\\d+\\.\\d+$", "type": "float"},
    ],
}
```

### 性能

自定义 schema 使用基于正则表达式的规则引擎。每个 scalar 按顺序匹配规则。
为获得最佳性能：

- 规则数量控制在 20 条以内
- 将最常用的模式放在前面
- 使用 `extends: core` 避免重复实现完整的 Core 解析逻辑

内置 Core schema 不受影响——它仍然使用零开销的 `match` 分发，
不受自定义 schema 注册的影响。
