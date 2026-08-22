---
title: 命令行工具
description: 使用 pyrs-yaml CLI 在终端中格式化、查询、编辑、校验和转换 YAML 文件，并保持往返保真。
tags:
  - docs
status: new
---

## 命令行工具

pyrs-yaml 提供可选的命令行工具 `pyrs-yaml`，将库的核心能力——往返格式化、JSONPath 查询、就地编辑、Schema 校验与格式转换——直接带到你的终端。

!!! note "环境要求"
    CLI 依赖可选的 `cli` extra，且需要 **Python >= 3.10**。库本身继续支持更旧的解释器版本。

### 安装

=== "pip"

    ```bash
    pip install "pyrs-yaml[cli]"
    ```

=== "uv"

    ```bash
    uv add --optional cli pyrs-yaml
    ```

验证安装：

```bash
pyrs-yaml --version
```

### 命令总览

| 命令 | 用途 |
|------|------|
| [`fmt`](#cmd-fmt) | 重新格式化 YAML，保留注释、锚点与顺序 |
| [`get`](#cmd-get) | 按 JSONPath 表达式查询值 |
| [`set`](#cmd-edit) | 设置路径处的值 |
| [`delete`](#cmd-edit) | 删除路径处的节点 |
| [`rename`](#cmd-edit) | 重命名映射键 |
| [`sort-keys`](#cmd-edit) | 对路径处的映射键排序 |
| [`move`](#cmd-edit) | 将子树移动到另一个已存在的路径 |
| [`frontmatter`](#cmd-frontmatter) | 提取 Markdown front matter 为 YAML |
| [`validate`](#cmd-validate) | 按 Schema 校验 YAML |
| [`to-json`](#cmd-convert) | 将 YAML 转为 JSON |
| [`from-json`](#cmd-convert) | 将 JSON 转为 YAML |
| [`compliance`](#cmd-compliance) | 报告 YAML Test Suite 合规率 |

文件参数为 `-` 或省略时从 **stdin** 读取；除非指定 `-o/--output` 或 `-i/--inplace`，结果一律输出到 **stdout**。流式输入通过 [`-A/--all-docs`](#multi-doc) 处理。

### 格式化（`fmt`） { #cmd-fmt }

`fmt` 通过往返 AST 重新序列化文档——注释、锚点、键顺序与样式全部保留：

```bash
$ echo "a:    1 # keep me" | pyrs-yaml fmt -
a: 1  # keep me
```

常用选项：

```bash
pyrs-yaml fmt config.yaml --indent 4        # 4 空格缩进
pyrs-yaml fmt config.yaml --inplace         # 就地重写文件（-i）
pyrs-yaml fmt config.yaml -o formatted.yaml # 输出到其他文件
```

### 查询（`get`） { #cmd-get }

`get` 计算 [JSONPath](editing.md) 风格的表达式并打印每个匹配项：

```bash
$ pyrs-yaml get deploy.yaml '$.servers[0].host'
db.example.com

$ pyrs-yaml get deploy.yaml '$..name' --format text   # 深度扫描
web
db

$ pyrs-yaml get deploy.yaml '$.servers[*]'            # 子树以 YAML 输出（默认）
```

通过 `--format/-f` 指定输出格式：`yaml`（默认）、`json` 或 `text`（原始标量值）。

### 编辑（`set`、`delete`、`rename`） { #cmd-edit }

编辑命令要求路径精确命中单个节点（不允许通配符）：

```bash
# VALUE 按 YAML 解析——数字、布尔与嵌套结构开箱即用
pyrs-yaml set config.yaml "$.retries" 5
pyrs-yaml set config.yaml "$.tags" '[a, b]'
pyrs-yaml set config.yaml "$.token" '12345' --string          # 强制按字符串处理
pyrs-yaml set config.yaml "$.a.b.c" new --create-missing      # 自动创建父级

pyrs-yaml delete config.yaml "$.legacy_key"
pyrs-yaml rename config.yaml "$.old_name" new_name

pyrs-yaml set config.yaml "$.port" 8080 --inplace             # 就地修改文件

pyrs-yaml sort-keys config.yaml                               # 对根映射键排序
pyrs-yaml sort-keys config.yaml "$.meta"                      # 对单个嵌套映射排序
pyrs-yaml move deploy.yaml "$.staging" "$.environments.dev"   # 移动子树
```

编辑会保留周边元数据——被编辑节点上方或行内的注释原样不动。

注意：

- 父级已存在时，`set` 即使不加 `--create-missing` 也会添加路径的末级键；该标志仅用于缺失的**中间**键。
- `sort-keys` 对 `path` 处映射的键排序（默认根节点）；不递归。
- `move` 的目标必须已存在，其值会被移动来的子树替换；两端均不允许通配符。

### 校验（`validate`） { #cmd-validate }

`validate` 按 Schema 定义文件或已注册的 Schema 名称检查文档——两个选项互斥：

```bash
pyrs-yaml validate app.yaml --schema-file schema.yaml
pyrs-yaml validate app.yaml --schema my_schema        # 经 register_schema() 注册
```

成功时静默退出 `0`；失败时所有违规项打印到 stderr 且退出码为 `1`——非常适合 CI：

```yaml
# schema.yaml
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
```

完整的 Schema 语言参见[自定义 Schema](custom-schema.md)。

### 转换（`to-json`、`from-json`） { #cmd-convert }

两个方向的命令都能自然地组合进管道：

```bash
$ pyrs-yaml to-json config.yaml
{
  "b": {
    "c": 2
  }
}

$ echo '{"name": "x"}' | pyrs-yaml from-json -
name: x
```

### 多文档流 { #multi-doc }

添加 `-A/--all-docs` 将输入视为 `---` 分隔的文档流，而非仅处理第一个文档：

```bash
pyrs-yaml fmt stream.yaml -A                              # 重新格式化每个文档
pyrs-yaml get stream.yaml '$..name' --format text -A      # 跨文档查询
pyrs-yaml to-json stream.yaml -A                          # 输出文档 JSON 数组
pyrs-yaml set stream.yaml "$.retries" 5 -A                # 编辑每个文档
pyrs-yaml validate stream.yaml --schema-file s.yaml -A    # 失败时报 "document N"
```

支持 `fmt`、`get`、`set`、`delete`、`rename`、`sort-keys`、`validate` 与 `to-json`。输出以标准 `---` 分隔符连接；编辑命令在路径可解析的文档上生效，仅当无任何文档匹配时失败。

### Markdown front matter（`frontmatter`） { #cmd-frontmatter }

```bash
$ pyrs-yaml frontmatter post.md
title: Hello

$ pyrs-yaml frontmatter post.md --body-out body.md   # 同时拆分正文
```

页面没有 front matter 时退出码为 `1`。库 API 详见 [Markdown 头信息](frontmatter.md)。

### YAML Test Suite 合规率（`compliance`） { #cmd-compliance }

```bash
pyrs-yaml compliance [--json] [SUITE_DIR]
```

针对 [yaml-test-suite](https://github.com/yaml/yaml-test-suite) 语料运行解析器（默认检出位置：`./Reference/yaml-test-suite`），按套件分区打印通过/失败统计——便于将 pyrs-yaml 与其他 YAML 实现进行对比评估。

### 退出码

| 退出码 | 含义 |
|--------|------|
| `0` | 成功 |
| `1` | 运行时错误——输入不可读、解析失败、无匹配、校验失败 |
| `2` | 用法错误——未知命令或选项 |

!!! tip "脚本化"
    错误走 stderr、数据走 stdout，因此 `pyrs-yaml` 可以干净地组合进管道：`pyrs-yaml get deploy.yaml '$..host' | sort -u`。
