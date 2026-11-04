# pyrs-yaml

[![PyPI 版本](https://img.shields.io/pypi/v/pyrs-yaml)](https://pypi.org/project/pyrs-yaml/) [![Python 版本](https://img.shields.io/pypi/pyversions/pyrs-yaml)](https://pypi.org/project/pyrs-yaml/) [![下载量](https://img.shields.io/pypi/dm/pyrs-yaml)](https://pypi.org/project/pyrs-yaml/)
[![许可证](https://img.shields.io/github/license/759401524/pyrs-yaml)](LICENSE-MIT) [![CI](https://img.shields.io/github/actions/workflow/status/759401524/pyrs-yaml/ci.yml?branch=main)](https://github.com/759401524/pyrs-yaml/actions) [![GitHub 发布](https://img.shields.io/github/v/release/759401524/pyrs-yaml)](https://github.com/759401524/pyrs-yaml/releases)
[![文档](https://img.shields.io/website?url=https%3A%2F%2F759401524.github.io%2Fpyrs-yaml%2F&label=docs&color=blue)](https://759401524.github.io/pyrs-yaml) [![GitHub 星标](https://img.shields.io/github/stars/759401524/pyrs-yaml)](https://github.com/759401524/pyrs-yaml) [![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://app.codspeed.io/759401524/pyrs-yaml?utm_source=badge)

[English](README.md) | **简体中文**

## 指标

[![GitHub 分支](https://img.shields.io/github/forks/759401524/pyrs-yaml)](https://github.com/759401524/pyrs-yaml) [![GitHub 关注](https://img.shields.io/github/watchers/759401524/pyrs-yaml)](https://github.com/759401524/pyrs-yaml) [![GitHub 问题](https://img.shields.io/github/issues/759401524/pyrs-yaml)](https://github.com/759401524/pyrs-yaml)

高性能 Python 库，支持 YAML、TOML、JSON、JSONC 与 JSON5，完美往返，基于 Rust 和 PyO3 构建。

## 特性

- **YAML 1.2 合规** — 使用 granit-parser 实现完整 YAML 1.2 支持，原生保留注释
- **五种格式，同一文档树** — 原生 TOML、JSON、JSONC、JSON5 读取器（`load_*`、`from_*`）与写入器（`to_toml`、`doc.to_json` / `to_jsonc` / `to_json5`）共享 YAML 文档树；只要目标方言能承载，注释与源码写法都会保留
- **行分隔 JSON** — `load_json_lines(text)` 返回一个 `JsonLines` 迭代器，每行解析为一个文档，因此读取 `.jsonl` / NDJSON 分片时同一时刻只有一条记录是活的
- **完美往返** — 保留注释、锚点、标签、chomping 指示符、标量样式和流式/块式格式
- **就地编辑** — 通过 JSONPath 风格路径（`doc.set("$.a.b", v)`）或 `Node` 树 API 编辑已解析文档，不丢失格式
- **高性能** — Rust 后端，`safe_dump`/`from_dict` 比 v0.10 快 7 倍（直接写入器，无中间 AST）；`safe_load`/`safe_loads` 快速路径在无锚点时跳过锚点追踪
- **深度限制解析** — `max_depth`（默认 1000）作用于 `parse`、`parse_file`、`parse_all_docs`、`parse_stream`、`safe_load`、`safe_loads`、`read_markdown`、`read_markdown_str`，防止深度嵌套攻击
- **NumPy ndarray 支持** — `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()` 将任意维度（0 维到 N 维）的 `numpy.ndarray` 零拷贝 Rust 调度序列化
- **JSON Schema 校验** — `YamlDocument.validate(schema)` 对 JSON Schema 校验已解析文档；失败时抛出 `YamlValidateError`；`jsonschema` 求值器需要另行安装（参见[安装](#安装)）
- **异步 I/O** — `safe_dumps_async` / `safe_dump_async` / `safe_loads_async` / `safe_load_async` 通过 `asyncio.run_in_executor`
- **增量重新解析** — `doc.source()` + `doc.reparse()` 用于以不同选项（如 `schema="yaml1.1"`）就地重新解析存储的 YAML
- **JSON 序列化** — `doc.to_json()` 将文档导出为标准 JSON
- **重复键** — `allow_duplicate_keys=True` 选择最后值优先；否则抛出 `YamlDuplicateKeyError`；`duplicate_keys="as_is"|"error"|"last_wins"|"first_wins"` 直接命名该策略
- **自定义标签处理器** — `register_tag` 支持基于优先级的链式调用，`YamlTagSkip`，`remove_tag`/`clear_tag_handlers`
- **Pydantic 模型** — `parse_as(Model, yaml)` 将已解析 YAML 对 Pydantic v2 模型校验
- **自定义 AST** — 可扩展 AST 用于高级 YAML 操作
- **PyYAML 兼容** — 直接替换，提供 `safe_load`/`safe_dump` API

## 安装

```bash
pip install pyrs-yaml
```

或使用 uv：

```bash
uv pip install pyrs-yaml
```

JSON Schema 校验不在上述两种安装的覆盖范围内：`Document.validate()` 把求值委托给 [`jsonschema`](https://pypi.org/project/jsonschema/)，而基础包（只依赖 `typing-extensions`）和它的 `cli`、`pydantic`、`settings` 三个 extras 都不会带上它。没有求值器时，这次调用抛 `ModuleNotFoundError: No module named 'jsonschema'`。

```bash
pip install jsonschema
```

### CLI

通过 `cli` extra 可获得可选的命令行工具（需 Python 3.10+）：

```bash
pip install "pyrs-yaml[cli]"
```

```bash
$ echo "a:    1 # keep me" | pyrs-yaml fmt -
a: 1  # keep me

$ pyrs-yaml get deploy.yaml '$..host' --format text
$ pyrs-yaml set config.yaml "$.port" 8080 --inplace
$ pyrs-yaml validate app.yaml --schema-file schema.yaml
$ pyrs-yaml to-json config.yaml
```

命令：`fmt` / `get` / `set` / `delete` / `rename` / `sort-keys` / `move` / `frontmatter` / `validate` / `to-json` / `from-json` / `compliance`，全部编辑/查询命令支持 `-A/--all-docs` 多文档模式——详见 [CLI 指南](https://759401524.github.io/pyrs-yaml/zh/guides/cli/)。

### 原生 CLI（`pyq`）

这些引擎也可以脱离 Python 使用，即 `pyq` 二进制——基于同一 crate 矩阵（`pyrs-yaml-core`、`pyrs-json`、`pyrs-toml`）的 jq/yq 风格工具：

```bash
cargo install --path crates/pyrs-yaml-cli
```

```bash
$ pyq get '.server.host' deploy.yaml
db.internal

$ pyq diff base.yaml head.yaml          # 语义对比；有差异时退出码 1
$ pyq merge base.yaml head.yaml         # 右偏深度合并
```

命令：`fmt` / `get` / `set` / `delete` / `rename` / `move` / `append` / `insert` / `sort-keys` / `frontmatter` / `validate` / `diff` / `merge` / `to-json` / `to-toml` / `from-json` / `from-toml` / `from-ini`。格式按文件扩展名自动识别，`--input jsonc|json5` 可读取 JSONC/JSON5 方言。详见 [pyq 指南](https://759401524.github.io/pyrs-yaml/zh/guides/pyq/)。

## 环境要求

- **支持的 Python 版本**（安装 wheel）：Python 3.8+（CPython）。GIL 构建使用单个 `cp38-abi3` wheel 覆盖 3.8–3.15；自由线程 `cp314t` wheel 以及面向 3.15t 及更高版本的 `cp315-abi3.abi3t` wheel 已针对 Windows（x64/x86/arm64）、macOS（x86_64/arm64）与 Linux glibc（x86_64/arm64）发布。musllinux 与其余 Linux 架构从源码构建。
- **Rust 工具链**（仅从源码构建时）：Rust 1.96 或更高（MSRV，edition 2024）。这高于 PyO3 自身的基线（PyO3 0.29 需 rustc 1.83+），是为了 std API 余量而有意选择的。安装 wheel 的终端用户无需 Rust。

## 文档

完整文档（English、简体中文、日本語、한국어）可在 [https://759401524.github.io/pyrs-yaml](https://759401524.github.io/pyrs-yaml) 查看。开发指南见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 快速开始

```python
import pyrs_yaml

# 解析 YAML
doc = pyrs_yaml.parse("key: value")
print(doc.to_yaml())  # key: value

# PyYAML 兼容 API
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# 往返保留注释
original = "# Comment\nkey: value  # inline\n"
doc = pyrs_yaml.parse(original)
assert doc.to_yaml() == original  # True

# 就地编辑而不丢失格式
doc.set("$.key", "edited")  # key: edited  # inline
doc.set("$.new", 1)  # 添加新键
print(doc.to_yaml())
```

### JSON Schema 校验

```python
doc = pyrs_yaml.parse("name: Alice\nage: 30")
doc.validate({"type": "object", "properties": {"name": {"type": "string"}}})
# None — 校验通过

# 无效 — 抛出 YamlValidateError
doc.validate({"type": "object", "required": ["email"]})
# pyrs_yaml.YamlValidateError: "email" 是必需属性
```

### 异步序列化

```python
import asyncio
import pyrs_yaml


async def main():
    yaml = await pyrs_yaml.safe_dumps_async({"a": 1})
    data = await pyrs_yaml.safe_loads_async(yaml)
    print(data)  # {'a': 1}


asyncio.run(main())
```

### 增量重新解析

```python
doc = pyrs_yaml.parse("x: on")
print(doc.get("x"))  # "on"（core schema：字符串）

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True（yaml1.1 schema：布尔值）
```

### JSON 导出

```python
doc = pyrs_yaml.parse("a: 1\nb: hello")
json_str = doc.to_json()  # '{"a": 1, "b": "hello"}'
```

### NumPy ndarray 支持

```python
import numpy as np
import pyrs_yaml

# 1 维数组
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyrs_yaml.safe_dump(arr)
print(yaml_str)
# - 1
# - 2
# - 3

# 2 维矩阵
matrix = np.array([[1, 2], [3, 4]], dtype="float64")
yaml_str = pyrs_yaml.safe_dump(matrix)
print(yaml_str)
# -
#   - 1.0
#   - 2.0
# -
#   - 3.0
#   - 4.0

# 往返
loaded = pyrs_yaml.safe_load(yaml_str)
assert loaded == [[1.0, 2.0], [3.0, 4.0]]
```

### 重复键

重复映射键默认抛出 `YamlDuplicateKeyError`：

```python
pyrs_yaml.parse("key: first\nkey: second")
# pyrs_yaml.YamlDuplicateKeyError: duplicate key: key
```

传入 `allow_duplicate_keys=True` 则保留**最后一个值**：

```python
doc = pyrs_yaml.parse("key: first\nkey: second", allow_duplicate_keys=True)
doc.get("key")  # "second"
```

该标志适用于 `parse`、`safe_load`、`safe_loads`、`parse_file`、`parse_all_docs` 和 `YAML(allow_duplicate_keys=True)`。在往返模式下，允许重复键的文档序列化时输出最后一个出现的键值对。

`duplicate_keys` 是同一决定的具名写法：`as_is`（默认，按键的身份类别给出今天各自的答案）、`error`（连空键与合并键一并拒绝）、`last_wins`（即 `allow_duplicate_keys=True` 的含义）与 `first_wins`（保留文档里先出现的一对）。同时传入两种写法会被拒绝（"pass one"），不在这四种之内的取值同样会被拒绝。命令行上，每个读取 YAML 的子命令都接受 `--duplicate-keys`。

两个键只差在读取器保留的标签或锚点上时，它们在 AST 里仍是两个键，于是删掉其中一个后，另一个还会顶着被删掉的那个名字作答。`doc.delete(path, on_duplicate=...)` 决定的就是这一点：`one`（默认）删掉路径解析到的那一对，`all` 删掉该键可能应答的每一对，`error` 则在还剩多于一对时拒绝。命令行 `delete` 接受同样三个取值，写作 `--on-duplicate`。

### 序列化选项

`to_yaml_with_options()` 控制缩进和换行：

```python
yaml_str = doc.to_yaml_with_options(
    indent_size=2,  # 基础缩进（省略按类型选项时使用）
    width=80,  # 换行宽度；0 表示不换行
    indent_mapping=4,  # 块映射每级缩进
    indent_sequence=2,  # 块序列每级缩进
    indent_offset=0,  # 整个文档的基础偏移
    note_placement="note_first",  # 或 "header_first"
)
```

省略时 `indent_mapping` / `indent_sequence` / `indent_offset` 分别默认为 `indent_size` / 0，因此 `indent_size=4` 仍然让所有层级缩进 4。

`note_placement` 决定注释行无法与自己那一行同处、而其正上方就是 `&a` / `!tag` 头行时的落点：`note_first`（默认）把它写在头行之上，也就是读回文本时注释所归档的位置，因此第一次发射就已经收敛；`header_first` 让头行留在上面，对该形状不收敛。其他任何取值都抛出 `YamlSerializeError`。

### 标签处理器

注册自定义 YAML 标签处理器以转换标量值：

```python
import pyrs_yaml


# 装饰器形式
@pyrs_yaml.register_tag("!custom")
def custom_handler(node):
    return f"custom:{node}"


# 命令式
pyrs_yaml.register_tag("!custom", lambda node: node.upper())

doc = pyrs_yaml.parse("name: !custom value")
doc.get("name")  # "custom:value"
```

- 同一标签的多个处理器按 `priority` 升序执行；抛出 `YamlTagSkip` 将控制权传递给下一个处理器。
- 处理器必须返回字符串 — 否则抛出 `YamlTagError`。
- `remove_tag("!custom")` 和 `clear_tag_handlers()` 注销处理器。

### Pydantic 模型

将 YAML 直接解析为 Pydantic v2 模型：

```python
from pydantic import BaseModel
import pyrs_yaml


class Config(BaseModel):
    name: str
    age: int


cfg = pyrs_yaml.parse_as(Config, "name: Alice\nage: 30")
cfg.name  # "Alice"
```

`parse_as` 对非 `BaseModel` 目标抛出 `TypeError`，当 YAML 不匹配模型时传播 Pydantic 的 `ValidationError`。

## 支持的特性

| 特性 | 支持 |
|---------|---------|
| YAML 1.2 | 完整 |
| 注释（独立 + 行内） | 保留 |
| 锚点（`&`）和别名（`*`） | 保留 |
| 标签（`!!str`、`!!int` 等） | 保留 |
| Chomping（`\|-`、`\|+`、`>-`、`>+`） | 保留 |
| 复合键（序列/映射作为键） | 支持 |
| 转义序列（`\n`、`\t`、`\uXXXX`） | 支持 |
| 流式集合（`{}`、`[]`） | 保留 |
| 块标量（`\|`、`>`） | 保留 |
| 合并键（`<<: *alias`） | 解析（可通过 `resolve_merges=False` 关闭） |
| **TOML / JSON / JSONC / JSON5** | **完整（读取、写入、往返；方言能承载的注释均保留）** |
| **事件流** | **五种格式均支持 `parse_stream(format=...)`** |
| **NumPy ndarray** | **完整（0 维到 N 维）** |
| **JSON Schema 校验** | **完整** |
| **异步 I/O** | **完整** |
| **增量重新解析** | **完整** |
| **JSON 导出** | **完整** |
| **重复键** | **经 `duplicate_keys` 命名的四种策略：`as_is` / `error` / `last_wins` / `first_wins`（默认的 `as_is` 按键的身份类别作答，普通碰撞时抛出 `YamlDuplicateKeyError`）** |
| **删除重复的键** | **`doc.delete(path, on_duplicate="one" \| "all" \| "error")`** |
| **自定义标签处理器** | **基于优先级链式 `register_tag`** |
| **Pydantic 模型** | **`parse_as()` 校验** |

## API 参考

### 核心函数

```python
# 解析 YAML 字符串（接受 str 或 bytes）
doc = pyrs_yaml.parse(yaml_str)
doc = pyrs_yaml.parse(yaml_bytes)

# 带选项解析（max_depth、schema、allow_duplicate_keys、duplicate_keys）
doc = pyrs_yaml.parse(yaml_str, resolve_merges=False, max_depth=500, schema="yaml1.1")
doc = pyrs_yaml.parse("key: first\nkey: second", duplicate_keys="first_wins")

# 解析 YAML 文件
doc = pyrs_yaml.parse_file("config.yaml")

# 解析多个 YAML 文档
docs = pyrs_yaml.parse_all_docs(yaml_str)


# 流式解析（on_event 回调）
def handler(event):
    print(event)
    return True  # 返回 False 停止


iter = pyrs_yaml.parse_stream(yaml_str, on_event=handler, max_depth=1000)

# 同一套事件词汇覆盖其余四种格式
iter = pyrs_yaml.parse_stream(toml_text, on_event=handler, format="toml")

# 转换为 YAML 字符串（带选项）
yaml_str = doc.to_yaml()
yaml_str = doc.to_yaml_with_options(indent_size=4, explicit_start=True, sort_keys=True)

# 按键获取值（带默认值）
value = doc.get("key")
value = doc.get("missing_key", "default")

# 获取根类型
doc.root_type()  # "mapping"、"sequence"、"scalar"、"null"

# 检查包含和长度
"key" in doc
len(doc)

# 迭代
for key in doc:
    print(key, doc[key])

# 从 dict 导出 YAML
yaml_str = pyrs_yaml.from_dict(data)

# i18n 语言管理
pyrs_yaml.set_language("zh-CN")
pyrs_yaml.get_language()  # "zh-CN"
pyrs_yaml.list_languages()  # ["en", "zh-CN"]
pyrs_yaml.detect_language()  # 从环境自动检测
pyrs_yaml.negotiate_language(["zh-CN", "en"], "en")  # "zh-CN"
```

### PyYAML 兼容 API

```python
# 加载 YAML 为 dict（支持 schema 和 max_depth）
data = pyrs_yaml.safe_load(yaml_str)
data = pyrs_yaml.safe_load(yaml_str, schema="yaml1.1", max_depth=500)

# 加载多个文档
docs = pyrs_yaml.safe_loads(yaml_str)
docs = pyrs_yaml.safe_loads(yaml_str, allow_duplicate_keys=True)

# 将 dict 导出为 YAML
yaml_str = pyrs_yaml.safe_dump(data)

# 将 dict 转换为 YAML
yaml_str = pyrs_yaml.from_dict(data)

# 将 JSON 转换为 YAML
yaml_str = pyrs_yaml.from_json(json_str)

# 将其余方言转换为 YAML
yaml_str = pyrs_yaml.from_toml(toml_str)
yaml_str = pyrs_yaml.from_jsonc(jsonc_str)
yaml_str = pyrs_yaml.from_json5(json5_str)

# 将其他方言解析为 Python 值
data = pyrs_yaml.load_toml(toml_str)
records = pyrs_yaml.load_json_lines(jsonl_str)  # JsonLines 迭代器，每行一个文档

# 将文档导出为其他方言
json_str = doc.to_json()
jsonc_str = doc.to_jsonc()
json5_str = doc.to_json5()
toml_str = pyrs_yaml.to_toml(doc.to_yaml())

# 导出到文件
pyrs_yaml.dump_file(data, "output.yaml")

# 从 markdown 提取 YAML 头信息
frontmatter, content = pyrs_yaml.read_markdown("post.md")
frontmatter, content = pyrs_yaml.read_markdown_str(markdown_text)
frontmatter, content = pyrs_yaml.read_markdown_str(markdown_text, max_depth=200)
```

## 性能

divan 基准测试在 `crates/pyrs-yaml-core/benches/yaml_bench.rs`（Rust）+ `pytest-codspeed` 在 `tests/test_benchmark_crosslib.py`（Python）。完整跨库对比（与 PyYAML 和 ruamel.yaml）见[基准测试文档](docs/en/performance/benchmarks.md)。

**v0.18 亮点：**

- **TOML 读取器：每个标量只分配一次，而非两次** — `from_toml_medium` 148 → 97 次分配、16,362 → 15,612 字节；IR 从扫描器起就携带 `Arc<str>`
- **JSON 读取器：无转义字符串借用源文本** — `from_json_medium` 74 → 48 次分配、9,675 → 9,444 字节；只有遇到转义才解码引号内文本
- **热路径的分配上限** — `crates/pyrs-yaml-core/tests/alloc_gate.rs` 钉住六个场景，防止每个键一个临时对象悄悄撑大工作集
- **指令数地板** — 固定 `rustc 1.97.1` 上对已提交基线实测：`from_toml_medium` −9.80%、`from_json_medium` −13.37%、`from_jsonc_medium` −11.28%、`from_json5_medium` −9.61%

## 开发

```bash
# 安装依赖
uv sync

# 构建 Python 扩展
uv run maturin develop --release

# 运行测试（Rust: cargo nextest; Python: uv run pytest）
cargo nextest run --all
uv run pytest tests/ -v --ignore=tests/benchmark_compare.py

# Lint 和格式化（Rust + Python）
cargo clippy -- -D warnings
cargo fmt
uv run ruff check .
uv run ruff format .

# 运行基准测试（Rust）
cargo bench

# 运行基准测试（Python）
uv run pytest tests/test_benchmark_crosslib.py tests/test_benchmark_api.py --codspeed

# 性能健全性检查
uv run pytest tests/test_performance.py -v

# Git 钩子
prek install --prepare-hooks
prek run --all-files
```

## 许可证

可选择以下任一许可证：

- [MIT 许可证](LICENSE-MIT)
- [Apache 许可证 2.0](LICENSE-APACHE)
