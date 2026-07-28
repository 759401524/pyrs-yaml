---

title: Installation
lang: zh

## 安装

### 系统要求

- **Python** ≥ 3.8 (CPython)
- **Platform**: Linux, macOS, Windows

### 从源码安装

该包尚未发布到 PyPI。从源码安装：

```bash
git clone https://github.com/759401524/pyyaml-rs.git
cd pyyaml-rs
uv run --frozen maturin develop --release
```

包以 **ABI3 wheel** 格式构建，单个 wheel 支持 Python 3.8 到 3.15 — 无需重新编译。

### 快速验证

```python
import pyyaml_rs

# 检查版本
print(pyyaml_rs.__version__)

# 快速测试
doc = pyyaml_rs.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ 安装验证成功")
```

### 运行测试

```bash
# Rust 测试
cargo test

# Python 测试
uv run --frozen pytest tests/
```
