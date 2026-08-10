---

title: 运行测试
lang: zh

pyrs-yaml 同时具有 Rust 单元测试和 Python 集成测试。

## Rust 测试

```bash
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

```bash
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

```bash
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

```rust
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

```python
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

测试套件通过率门限为 **95%**（参见 `test_compliance_report`）。少数用例有意不去追逐，因为拒绝它们符合规范且与参考解析器（特别是 PyYAML/libyaml）一致：

| ID | 输入 | 接受为偏差的原因 |
|:---|:-----|:---------------|
| `ZYU8` | `%YAML 1.1 1.2` | 带尾随内容的版本指令在 YAML 1.2 语法（`ns-yaml-version ::= ns-dec-digit+ '.' ns-dec-digit+`）下**无效**。PyYAML 也拒绝它。套件自己的注释说这些指令变体"根本没有用"，不鼓励支持它们。 |

其他所有套件用例均通过（当前 405/406 = 99.75%）。
