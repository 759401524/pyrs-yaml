---

title: 运行测试
lang: zh

## 运行测试

pyyaml-rs 同时具有 Rust 单元测试和 Python 集成测试。

### Rust 测试

```bash
# 运行所有 Rust 测试
cargo test

# 运行特定模块的测试
cargo test ast
cargo test parser
cargo test serializer

# 带输出运行
cargo test -- --nocapture

# 仅运行集成测试
cargo test --test integration
```

#### 测试覆盖

- **`src/ast.rs`** — 节点构建、元数据、等值性
- **`src/parser/`** — 解析各种 YAML 构造
- **`src/serializer.rs`** — 序列化往返保存
- **`src/integration/`** — YAML Test Suite 集成

### Python 测试

```bash
# 运行所有 Python 测试
pytest tests/

# 详细输出运行
pytest tests/ -v

# 运行特定测试文件
pytest tests/test_parse.py

# 运行匹配模式的测试
pytest tests/ -k "comment"

# 带覆盖率运行
pytest tests/ --cov=pyyaml_rs --cov-report=term-missing

# 运行基准测试
pytest tests/ --benchmark-only --benchmark-json=results.json
```

#### 测试文件

| 文件 | 覆盖范围 |
|------|---------|
| `test_parse.py` | 解析、数据类型、特殊字符 |
| `test_serialize.py` | 序列化、往返保存 |
| `test_edge_cases.py` | 边缘情况、错误处理 |
| `test_errors.py` | 自定义异常类型、文件 I/O |
| `test_features.py` | Markdown 前端元数据、from_dict/from_json |
| `test_json.py` | JSON ↔ YAML 转换 |
| `test_tabs.py` | 制表符处理 |
| `test_yaml_suite.py` | YAML Test Suite 集成 |
| `test_performance.py` | 性能健全性检查 |
| **`test_numpy.py`** | **NumPy ndarray 序列化（0 维到 N 维，所有 dtype）** |

### CI 测试

GitHub Actions 在每次推送和 PR 上运行：

- **Rust**: `cargo test`、`cargo clippy -- -D warnings`、`cargo fmt --check`
- **Python**: 在 4 个 Python 版本 × 3 个操作系统上运行 `pytest tests/`
- **Maturin**: 为每个 Python 版本构建 wheel

### 添加新测试

#### Rust 测试

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
import pyyaml_rs
import pytest

class TestNewFeature:
    def test_basic(self):
        result = pyyaml_rs.parse("key: value")
        assert result.get("key") == "value"

    def test_edge_case(self):
        # 边缘情况测试
        pass
```

### 测试类别

- **单元测试** — 单个函数、小输入
- **集成测试** — 完整的解析 → 序列化往返保存
- **边缘情况测试** — 特殊字符、空输入、格式错误的 YAML
- **性能测试** — 健全性检查（不是基准测试）
- **YAML Test Suite** — YAML 合规性的外部测试套件
