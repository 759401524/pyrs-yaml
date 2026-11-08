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
默认值：`core`。名称必须是内置 schema（`failsafe`、`json`、`core`、`yaml1.1`）或已注册的
schema；未知名会被拒绝，错误信息列出合法集合。

**`rules`** — 必填、非空的有序列表。首个匹配的 pattern 决定类型。
文本入口（`register_schema`）与内联字典入口都会拒绝 `rules` 缺失或为空的 schema。
支持的类型：

| `type` | Python 结果 | 示例 |
|--------|------------|------|
| :material-null: `null` | `None` | `~` |
| :material-toggle-switch: `bool` | `True` / `False` | `true`, `yes`, `on` |
| :material-numeric: `int` | `int` | `42`, `0xFF`, `0o77`, `0b1010` |
| :material-decimal: `float` | `float` | `3.14`, `1e10` |
| :material-format-text: `str` | `str` | `2026-08-11` |

### 注册和使用 Schema

```python title="注册并使用 schema"
import pyrs_yaml

# 从 YAML 字符串注册
pyrs_yaml.register_schema(
    "hex",
    """
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
""",
)

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

```python title="从文件加载 schema"
# hex.yaml 包含上述 schema YAML
pyrs_yaml.load_schema("hex", "path/to/hex.yaml")
```

#### 列出已注册 schema

`list_schemas()` 返回所有已注册的 schema 名称（内置 + 自定义）：

```python title="列出已注册 schema"
print(pyrs_yaml.list_schemas())
# ['failsafe', 'json', 'core', 'yaml1.1', 'hex', ...]
```

#### 结构化校验

结构化校验不属于标量解析。`Document.validate()` 是本包唯一的校验入口：它接受
[JSON Schema](https://json-schema.org/)，并把求值委托给 Python 的 `jsonschema`。
schema 定义中的 `validate:` 段不会被读取。

```python title="用 JSON Schema 校验"
import pyrs_yaml

doc = pyrs_yaml.parse("port: 8080\ntags: [web, api]\n")
schema = {
    "type": "object",
    "properties": {"port": {"type": "integer"}},
    "required": ["port"],
}
doc.validate(schema)  # 返回 None；文档不合格时抛出 YamlValidateError
```

错误信息直接来自参考引擎（`'port' is a required property`）；同一份 schema 与实例交给
`jsonschema.validate()` 会得到相同结论。校验文件请参阅 [CLI 指南](cli.md)。

委托还带来两条边界，本版本两条都保留：它们是本项目已登记的已知边界，不是缺陷，而自建一个属于项目自己的求值器是另一个设计问题。

**这里的任何安装途径都不会带入求值器。** pyrs-yaml 只依赖 `typing-extensions`，其 extras 是 `cli`、`pydantic` 与 `settings`，所以在只有本库的环境里 `doc.validate(schema)` 抛 `ModuleNotFoundError: No module named 'jsonschema'` — 没有任何 `Yaml*` 类应答，即便文档本会通过这份 schema。途径是 `pip install jsonschema`。

**schema 文本是 JSON，从来不是 YAML。** 与本页 `register_schema()` 和 `load_schema()` 读取的定义不同，交给 `validate()` 的 `str` 在询问任何求值器之前会先经过 `json.loads`，所以 `doc.validate("type: object\nrequired: [port]\n")` 抛 `json.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)` — 无论装没装 `jsonschema` 都一样。CLI 对同一个文件给出同样的拒绝：`pyrs-yaml validate doc.yaml --schema-file schema.yaml` 把那段 traceback 打到 stderr，退出 1，而它的 `except YamlValidateError` 分支永远看不到这次失败。

### 内联 Dict Schema

无需预先注册，直接传入字典：

```python title="内联 dict schema"
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

=== "将日期保留为字符串"

    ```python
    schema = {
        "extends": "core",
        "rules": [{"pattern": "^\\d{4}-\\d{2}-\\d{2}$", "type": "str"}],
    }
    ```

=== "添加 YAML 1.1 布尔值"

    ```python
    schema = {
        "extends": "core",
        "rules": [{"pattern": "^(yes|no|Yes|No|YES|NO)$", "type": "bool"}],
    }
    ```

=== "严格 JSON 模式"

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

内置 Core schema 不受影响——它仍然使用零开销的 `match` 分发，不受自定义 schema 注册的影响。

---

### 另请参阅

- [插件开发](plugin-development.md) — 构建自定义标签处理器
- [i18n 错误消息](i18n.md) — 为你的 schema 本地化错误消息
- [Schema API 参考](../api/reference.md#yaml-schema-language) — `register_schema()` 和内联 schema 文档
