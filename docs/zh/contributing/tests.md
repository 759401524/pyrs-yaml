---
title: 运行测试
description: pyrs-yaml 的测试指南，涵盖 Rust 测试、Python 测试、CI 测试和 YAML Test Suite 合规性验证。
tags:
  - docs
status: new
---

pyrs-yaml 同时具有 Rust 单元测试和 Python 集成测试。

## Rust 测试

```bash title="运行 Rust 测试"
# 使用 nextest 运行所有 Rust 测试（推荐）
cargo nextest run --all

# 使用 cargo test 运行所有 Rust 测试
cargo test --all

# 运行纯 Rust 核心测试（无需 Python 运行时）
cargo test --all --no-default-features

# 带输出运行
cargo test --all -- --nocapture
```

### 测试覆盖

- **`crates/pyrs-yaml-core/src/ast.rs`** — 节点构建、元数据、等值性
- **`crates/pyrs-yaml-core/src/parser/`** — 解析各种 YAML 构造
- **`crates/pyrs-yaml-core/src/serializer.rs`** — 序列化往返
- **`crates/pyrs-yaml-core/src/editing/`** — 编辑原语（navigate、region、dirty、metadata）
- **`crates/pyrs-yaml-core/src/integration/`** — YAML Test Suite 集成
- **`crates/pyrs-yaml/src/fidelity.rs`** — 基于属性的模糊测试

## Python 测试

```bash title="运行 Python 测试"
# 运行所有 Python 测试
uv run pytest tests/ -v

# 运行特定测试文件
uv run pytest tests/test_edit.py -v

# 运行特定测试类
uv run pytest tests/test_node_api.py::TestDocWalk -v

# 带覆盖率运行
uv run pytest tests/ -v --cov=pyrs_yaml

# 运行合规性套件
uv run pytest tests/test_yaml_suite.py -v

# 运行基准测试
uv run pytest tests/ --codspeed
```

## Maturin 构建

```bash title="构建并安装"
# 构建并安装（使用 monorepo manifest-path）
uv run maturin develop --release

# 生成 stubs 用于 .pyi 文件
uv run maturin build --release --generate-stubs
```

## 测试文件

| 文件 | 覆盖范围 |
|------|---------|
| `test_parse.py` | 解析、数据类型、特殊字符 |
| `test_serialize.py` | 序列化、往返 |
| `test_edge_cases.py` | 边缘情况、错误处理 |
| `test_errors.py` | 自定义异常类型、文件 I/O |
| `test_features.py` | Markdown Front Matter、from_dict/from_json |
| `test_json.py` | JSON ↔ YAML 转换 |
| `test_tabs.py` | 制表符处理 |
| `test_yaml_suite.py` | YAML Test Suite 集成 |
| `test_performance.py` | 性能健全性检查 |
| **`test_numpy.py`** | **NumPy ndarray 序列化（0 维到 N 维，所有 dtype）** |

## CI 测试

GitHub Actions 在每次推送和 PR 上运行：

- **Rust**: `cargo nextest run --all`、`cargo clippy --all -- -D warnings`、`cargo fmt --check`
- **Python**: 在 4 个 Python 版本 × 3 个操作系统上运行 `uv run pytest tests/`
- **Maturin**: 为每个 Python 版本构建 wheel（通过 `crates/pyrs-yaml/Cargo.toml`）

## 添加新测试

### Rust 测试

```rust title="Rust 测试模板"
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_feature() {
        // 在这里写测试
    }
}
```

#### Python 测试

```python title="Python 测试模板"
import pyrs_yaml
import pytest


class TestNewFeature:
    def test_basic(self):
        result = pyrs_yaml.parse("key: value")
        assert result.get("key") == "value"

    def test_edge_case(self):
        # 边缘情况测试
        pass
```

## 测试类别

- **单元测试** — 单个函数、小输入
- **集成测试** — 完整的解析 → 序列化往返
- **边缘情况测试** — 特殊字符、空输入、格式错误的 YAML
- **性能测试** — 健全性检查（不是基准测试）
- **YAML Test Suite** — YAML 合规性的外部测试套件

## YAML Test Suite 已知偏差

套件由 `tests/test_yaml_suite.py` 的三道点名门禁读取，而不是按百分比：所有有效用例都必须解析，
只除外本项目有意的那一次拒绝；任何无效用例都不得通过；JSON 比对除一份记下五个用例及各自成因的
账本之外必须全部一致。`test_compliance_report` 把报告自身的数字地板定在 99.5%。少数用例有意不去
追逐，因为拒绝它们符合规范且与参考解析器（特别是 PyYAML/libyaml）一致：

用例按门禁的命名方式书写。套件的 id 指的是一个*文件*，351 个文件里有 18 个承载不止一个用例，
因此名字还带上该用例在文件中的位置。

| ID | 输入 | 接受为偏差的原因 |
|:---|:-----|:---------------|
| `ZYU8#3` | `%YAML 1.1 1.2` | 带尾随内容的版本指令在 YAML 1.2 语法（`ns-yaml-version ::= ns-dec-digit+ '.' ns-dec-digit+`）下**无效**。PyYAML 也拒绝它。套件自己的注释说这些指令变体"根本没有用"，不鼓励支持它们。 |

其他所有套件用例均通过（当前 405/406 = 99.75%）。

JSON 账本是两个未修缺陷加一个比对写法差异，账目只能靠引擎学会规则来销，绝不会因为集合变大。
缺陷：内容行全为空却要求保留换行的块标量多保留了一个换行（`JEF9#1`、`K858#1`）。另一项只是写法之差：
套件把 `!!binary` 节点写成标量自身的源码文本，而比对把载入的字节写成规范 base64（`565N#1`）——两种写法
构造出同样的 185 字节，这正是 `test_binary_spellings_construct_the_same_bytes` 钉住的事实。

已经有条目这样销过两次账。别名原本绑定同名锚点的*最后*一次定义，而不是紧邻它之前的那一次，所以 `3GZX`
读到 `Bar`，而套件自己的事件树与 ruamel 都读 `Foo`；PyYAML 直接拒绝重复使用的锚点名（`found duplicate
anchor`），所以权威是语料和接受它的那个库，分歧由
`test_a_reused_anchor_name_is_read_and_not_refused` 钉住。另一次是裸 `!` 原本像无标签标量一样被 schema
解析，于是 `! 12` 读成整数：YAML 1.2 §6.8 把该节点的标签留在 `!`，§10.5.2 把应用侧的解析规则限定在
`?` 标签，所以标量就是它自己的文本——spec Example 6.28 与套件都写作 `"12"`。两个 Python 库都改用隐式
解析（ruamel 有一个未关的工单 #573），所以这个分歧把两种读数一起记下，并由
`tests/test_cross_format_tags.py` 里的一张表把加载器与 `to_json`、`to_jsonc`、`to_json5`、`to_toml`
钉到同一个答案。
