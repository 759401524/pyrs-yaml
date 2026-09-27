---
title: TOML、JSON 与 INI 格式
description: 使用 pyrs-yaml 在 TOML、JSON 与 INI 之间交换数据，YAML 仍是唯一可编辑表示。
tags:
  - docs
status: new
---

读写 TOML、JSON 与 INI — 以 YAML 为中枢。

## 轮毂模型

pyrs-yaml 支持多种配置格式，但 **YAML 是唯一可编辑的表示**：往返编辑
（注释、锚点、原位拼装）为 YAML 专属。交换格式负责进出转换：

```text
load_toml / load_ini / JSON 文本 ──▶ 值与 YAML ──▶ parse() / edit() / dump
to_toml  ◀── YAML 文本 ◀──────────────────┘
```

## JSON：天然原生

YAML 1.2 是 JSON 的超集，任何 JSON 文档都能在所有接受 YAML 的入口直接
加载，无需单独接口：

```python title="JSON 输入"
import pyrs_yaml

data = pyrs_yaml.safe_load('{"a": [1, 2], "b": true}')
# {'a': [1, 2], 'b': True}
```

专用的 `json` schema（见 [自定义 Schema](custom-schema.md)）还将标量解析
限定为 JSON 兼容规则；`from_json` 与 CLI `to-json` 提供显式转换。

## TOML

三个函数，与 JSON 转换家族同构：

```python title="TOML 进出"
import pyrs_yaml

# TOML 文本 -> Python 值（快速路径，无中间文档）
config = pyrs_yaml.load_toml('s = "true"\nn = 42\n')
# {'s': 'true', 'n': 42}   <- TOML 字符串不会被重新解析："true" 仍是字符串

# TOML 文本 -> YAML 文本（之后照常编辑）
yaml_text = pyrs_yaml.from_toml('title = "app"\nport = 8080\n')
# 'title: "app"\nport: 8080\n'

# YAML 文本 -> TOML 文本
toml_text = pyrs_yaml.to_toml("name: app\ncount: 3\nnested:\n  a: 1\n  b: two\n")
# 'name = "app"\ncount = 3\nnested = { a = 1, b = "two" }\n'
```

!!! note "日期时间"

    TOML datetime 经内建 `!timestamp` 插件直接成为真正的
    `datetime.datetime` 对象：

    ```python
    pyrs_yaml.load_toml("when = 2026-01-02T03:04:05Z\n")
    # {'when': datetime.datetime(2026, 1, 2, 3, 4, 5, tzinfo=datetime.timezone.utc)}
    ```

### `to_toml` 的拒绝清单

TOML 无法表达部分 YAML 结构；以下情况抛出带稳定消息的 `ValueError`，
而不是静默丢数据：

- null 值（TOML 没有 null）
- 非表格文档（根节点是标量或数组）
- 锚点 / 别名，以及 `!timestamp` 之外的 tag
- 非标量映射键

## INI

按设计只读 — INI 没有官方语法，pyrs-yaml 用标准库解析器接收它，写出
交给你自己的工具：

```python title="INI 输入"
import pyrs_yaml

config = pyrs_yaml.load_ini("[server]\nHost = 127.0.0.1\nPort = 8080\n")
# {'server': {'Host': '127.0.0.1', 'Port': '8080'}}
```

行为说明：

- 键的大小写被保留（`Host` 而非 `host`）；
- 节、`;`/`#` 注释与多行值遵循严格模式的
  `configparser.RawConfigParser` 语义 — 重复键或缺少节头会抛
  `ValueError`；
- 所有值都是字符串；需要类型时自行转换，或先经 YAML 中转：

```python title="INI 转 YAML"
import pyrs_yaml

yaml_text = pyrs_yaml.safe_dump(pyrs_yaml.load_ini("[s]\nport = 8080\n"))
# s:\n  port: '8080'\n
```

## 另见

- [自定义 Schema](custom-schema.md) — `json` 标量解析 schema
- [命令行工具](cli.md) — `to-json` / `from-json` 子命令
- [往返保真](round-trip.md) — YAML 为何是编辑中枢
