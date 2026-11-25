---
title: TOML、JSON、CSV/TSV 与 INI 格式
description: 使用 pyrs-yaml 在 TOML、JSON、CSV/TSV 与 INI 之间交换数据，YAML 仍是唯一可编辑表示。
tags:
  - docs
status: new
---

读写 TOML、JSON、CSV/TSV 与 INI — 以 YAML 为中枢。

## 轮毂模型

pyrs-yaml 支持多种配置格式，但 **YAML 是唯一可编辑的表示**：往返编辑
（注释、锚点、原位拼装）为 YAML 专属。交换格式负责进出转换：

```text
load_toml / load_ini / load_csv / JSON 文本 ──▶ 值与 YAML ──▶ parse() / edit() / dump
to_toml  ◀── YAML 文本 ◀──────────────────────────────┘
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

### 行分隔 JSON：`load_json_lines` { #json-lines }

一个 `.jsonl` / NDJSON 文件不是一个 JSON 文档，所以任何接收文档的入口都读不了
它 — 对两行记录的文件调用 `load_json`，会把第二行当作尾部字符拒绝掉。
`load_json_lines` 是那条能读的路线，它返回一个 `JsonLines` 迭代器，每个
`__next__` 只解析出一个文档，因此同一时刻只有一行是活的：

```python title="JSON Lines"
import pyrs_yaml

records = pyrs_yaml.load_json_lines('{"a": 1}\n{"b": 2}\n')
next(records)  # {'a': 1}
list(pyrs_yaml.load_json_lines('{"a": 1}\n{"b": 2}\n'))
# [{'a': 1}, {'b': 2}]
```

每个取值都是 `load_json` 对那一行返回的结果，而这条流守着引擎的约定：格式未定义
的行会抛出 `YamlParseError`，消息里点名它在整个输入中的位置，而游标不会移动，
于是再次索取报告的是同一条记录，而不是它后面那一条。

```python title="被拒绝的一行会说什么"
list(pyrs_yaml.load_json_lines('{"a": 1}\n\n'))
# pyrs_yaml.YamlParseError: a blank line carries no document in
# a line-delimited JSON stream at line 2 column 1

next(pyrs_yaml.load_json_lines("// note\n"))
# pyrs_yaml.YamlParseError: expected a JSON value at line 1 column 1

next(pyrs_yaml.load_json_lines('{"a": 1,\n'))
# pyrs_yaml.YamlParseError: expected a quoted object key at line 1 column 9
```

空行无论位于哪里都会被拒绝，包括文件末尾多出来的那一个，因为对
`{"a": 1}\n\n` 回答 `[]` 的流是在猜作者本想丢掉哪一行。一行承载一个完整的值，
既必要也充分：没有分隔符，没有尾逗号，每行一个文档。

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

    TOML datetime 经内建 `!!timestamp` 插件直接成为真正的
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
- 锚点 / 别名，以及 `!!timestamp` 之外的 tag
- 非标量映射键

## CSV / TSV

它是一张表而不是一份文档：表头行给出列名，每条数据记录成为一个映射。
单元格按 YAML 核心 schema 定型，`42` 是整数、`true` 是布尔值 —— 而行走的
始终是单元格的**源文本**，于是 CSV → YAML → CSV 一轮往返能逐字节复现原文：

```python title="CSV 输入"
import pyrs_yaml

table = pyrs_yaml.load_csv("name,port\nweb-1,8080\n")
# [{'name': 'web-1', 'port': 8080}]

yaml_text = pyrs_yaml.from_csv("name,port\nweb-1,8080\n")
# - name: web-1\n  port: 8080\n
```

行为要点：

- 分隔符默认是 `,`，TSV 用 `\t`；`;` 与 `|` 也接受（Python 入口用
  `delimiter=`，`pyq` 用 `--input csv|tsv`）；
- 引号遵循 RFC 4180 —— 引号内的字段可以含分隔符、换行，`""` 表示一个引号；
- 开头的 UTF-8 BOM 会被剥掉，CRLF 与 LF 都接受；
- 空单元格是 null；带前导零的单元格保留拼写（`007` 仍是 `007`），并解析为整数；
- 引擎自身的往返是逐字节的；经 YAML **文本**中转时有三处归一——CRLF 变 LF、BOM 被剥掉且不再写回、
  空单元格与写作 `null` 的单元格是同一份文档（YAML 只有一种 null 写法）；
- 引号在这里只是语法，不是类型断言：写作 `"true"` 的单元格会被推断为布尔值，因此引号不会
  经文本中转保留。若要求所有单元格都是字符串，请使用 `--no-infer`（或 `infer=False`）；
  看起来像数字或日期的字符串同理；
- 列数与表头不一致的行、以及重复出现的列名，会**带位置被拒绝**，绝不悄悄补齐或改写；
- TSV 是同一个引擎换用制表符，不是第二种格式。

写回同理，并拒绝 CSV 装不下的结构 —— 单元格里的嵌套映射没有 CSV 写法：

```python title="YAML 转 CSV"
import pyrs_yaml

csv_text = pyrs_yaml.to_csv("- name: web-1\n  port: 8080\n")
# name,port\nweb-1,8080\n
```

## XML

只读：本切片只把 XML 转成 YAML，不做反向。元素、属性、注释、处理指令和字符数据会保留，
XML 声明、BOM、CDATA 分隔符、实体拼写、自闭合形式和属性引号风格会被规范化掉。

```python title="XML 输入"
import pyrs_yaml

values = pyrs_yaml.load_xml('<root id="7"><name>Ada</name></root>')
# {'root': {'+@id': '7', 'name': 'Ada'}}

yaml_text = pyrs_yaml.from_xml("<root><item>a</item><item>b</item></root>")
# root:\n  item:\n    - a\n    - b\n
```

行为要点：

- 保留键一律带 `+` 前缀：`+@attr`、`+content`，注释是 `+c_<n>`，处理指令是 `+p_<n>`；
  没有属性的纯文本元素仍是普通标量。
- `007`、`true`、`null` 这类字符数据保持字符串，经过 YAML 枢纽时不会被悄悄改型。
- DTD、`<!ENTITY` 和未声明的 `&name;` 会以稳定的 `E-XML-*` 代码被拒绝：没有实体展开，
  也没有外部读取。
- `comments=False` / `pi=False`（CLI 对应 `--no-comments` / `--no-pi`）可丢弃相应部分。
- CLI 动词只有 `pyq from-xml`；没有 `to-xml` 或 `to_xml`。

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
- [命令行工具](cli.md) — `to-json` / `from-json` / `to-csv` / `from-csv` 子命令
- [往返保真](round-trip.md) — YAML 为何是编辑中枢
