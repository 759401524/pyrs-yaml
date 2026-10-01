---
title: pyq - Rust 原生 CLI
description: 基于 pyrs-yaml-core 的 jq/yq 风格 Rust 命令行工具，支持 YAML、JSON、TOML 与 INI，运行时无需 Python。
tags:
  - docs
status: new
---

`pyq` 是 Rust 原生命令行工具：`pyrs-yaml-core` 直接接入 jq/yq 风格接口，
运行时无需 Python。它与 Python 版 [pyrs-yaml CLI](cli.md) 互补；两者共享
同一内核以及一致的退出码与错误消息语义。

## 安装

```bash
cargo install --path crates/pyrs-yaml-cli   # 从源码检出安装
# 或在仓库内构建：
cargo build -p pyrs-yaml-cli --release      # -> target/release/pyq
```

## 查询（jq 风格）

```bash
# JSONPath-lite：点键、[n]、[-n]（Python 式负索引）、['key']、[*]
# 前导点可省略，单独 `.` 表示整个文档
$ pyq get '.servers[-1].host' inventory.yaml
web-3

# 通配符展开全部匹配，像 jq 一样流式输出：每个匹配一个
# YAML 文档（加 --json 则每行一个 JSON 值）
$ pyq get '.servers[*].port' --json inventory.yaml
8080
8081

# 文件为 `-` 或省略时读 stdin；--raw 输出裸标量
$ cat services.yaml | pyq get --raw .db.pool.size
20

# JSON 输出（保持键序）
$ pyq get '.servers' --json services.yaml
[ { "host": "web-1", "port": 8080 }, ... ]
```

## 过滤动词（jq 风格后处理）

结构化旗标，不是表达式语言——无论命令行旗标顺序如何，始终按固定管线
作用于匹配流：`select -> sort -> unique -> slice`，最后是 `join`。

```bash
pyq get '.servers[*]' --select 'port >= 1000' services.yaml
pyq get '.servers[*]' --sort-by host --desc services.yaml
pyq get '.tags[*]' --unique --skip 2 --take 5 blob.yaml
pyq get '.hosts[*]' --join ',' --raw inventory.yaml   # 单行裸文本
```

| 旗标 | jq 对应 | 说明 |
|------|---------|------|
| `--select 'PATH OP LITERAL'` | `select(.PATH OP LITERAL)` | OP 为 `== != > >= < <=`；字面量按 YAML 解析；路径缺失或类型不匹配一律 false（无 jq 全序） |
| `--sort-by PATH` / `--desc` | `sort_by(.PATH)` | 稳定排序；缺键排最后 |
| `--unique` | `unique` | 排序后去重，与 jq 一致 |
| `--first` / `--last` | `.[0]` / `.[-1]` | 互斥 |
| `--skip N` / `--take N` | `.[N:][…]` | 流切片 |
| `--join SEP` | `join(SEP)` | 仅限全标量流 |

## 编辑（yq 风格）

```bash
# 值是 YAML 表达式（JSON 也可，YAML 是其超集）
pyq set '.db.pool.size' 50 services.yaml          # 打印编辑后文档
pyq set -i '.db.pool.size' 50 services.yaml       # 原地回写文件
pyq set --create-missing '.a.b.c' 1 empty.yaml    # 自动创建中间层
pyq delete '.legacy_field' -i config.yaml
pyq sort-keys '$' -i config.yaml                  # 排序一层映射键
```

所有编辑都经由共享的 splice 引擎：只要文档布局符合条件，未触碰的行
（注释、空行、特殊空格）逐字节保持不变。

编辑后的文档经由与 `fmt` 相同的往返序列化器输出：注释、锚点与键序
全部保留；注入的值保持自身书写风格（`[1, two]` 保持 flow 风格，
`"true"` 保持带引号字符串）。

## 转换

```bash
pyq fmt k8s.yaml                 # 保留注释的规范化
pyq fmt --explicit-start cfg.yaml
pyq fmt --indent 4 --width 0 cfg.yaml   # 块缩进 4，纯量不换行
pyq fmt --sort-keys cfg.yaml     # 排序全部映射（整篇文档）
pyq fmt --indent 4 -i cfg.yaml   # 就地改写文件
pyq to-json config.yaml          # YAML -> JSON（保持键序）
pyq to-json --jsonc config.yaml  # AST 上保留的注释原样回写
pyq to-json --json5 config.yaml  # JSON5 写法（'x'、.5、+7、Infinity）
pyq to-toml compose.yaml
pyq from-toml Cargo.toml         # TOML -> YAML
pyq from-json package.json       # JSON -> YAML
pyq from-ini settings.ini        # INI -> YAML（值均为字符串）
pyq validate k8s.yaml --schema rules.yaml   # 解析检查 + schema 语言规则校验
pyq frontmatter README.md --body-out body.md # 分离 Markdown front matter
```

输入格式按文件扩展名识别（`.json`、`.jsonc`、`.json5`、`.toml`、`.ini`），
可用 `--input yaml|json|jsonc|json5|toml|ini` 覆盖。由于 YAML 是 JSON 的超集，
JSON 内容走 YAML 通道也能原样解析；JSONC/JSON5 内容则经由原生方言引擎，
注释与 JSON5 写法会挂在 AST 上——`pyq to-json --input jsonc --jsonc`
即可完成保留注释的往返。

## 退出码

- `0` 成功；
- `1`：路径缺失、解析失败或 TOML 无法表达的结构（null 值、非表格
  根）时向 stderr 输出 `pyq: <消息>`——与 Python API 相同的稳定消息。

## Shell 补全

```bash
pyq completion bash > /etc/bash_completion.d/pyq   # bash
pyq completion zsh  > "${functions[@]:0:1}/_pyq"   # zsh
pyq completion fish | source                        # fish
pyq completion powershell > pyq.ps1                 # PowerShell
```

## 能力范围

| 能力 | pyq | pyrs-yaml CLI（Python） |
|------|-----|--------------------------|
| 查询 / set / delete / 格式化 / 转换 | ✅ | ✅ |
| 动词后处理（`select`/`sort`/`unique`/…） | ✅ | — |
| 布局钉死编辑（splice 引擎） | ✅ | ✅ |
| 路径级 sort-keys | ✅ | ✅ |
| 多文档查询流（`-A`：get/fmt/to-json） | ✅ | ✅ |
| rename / move / append / insert / frontmatter / `validate`（schema 语言） | ✅ | ✅ |
| 多文档编辑（`-A` 覆盖全部编辑命令，`to-json -A`） | ✅ | ✅ |

两者均经由同一核心 plan/splice 引擎做版式钉死的编辑。对多文档流，`pyq -A` 更进一步：每个文档独立 splice 状态，未触碰的文档和所有 `---` 分隔行逐字节保持原样，布局异常的文档单独回退（`to-json -A` 输出 JSON 数组）。Python CLI 仅存的差异是按注册的 CustomType 校验——那是 Python 层的概念。

## 另见

- [命令行工具](cli.md) — Python 版 `pyrs-yaml` 命令
- [TOML、JSON 与 INI 格式](formats.md) — 库侧转换 API
- [原地编辑](editing.md) — `pyq` 编辑所依赖的往返模型
