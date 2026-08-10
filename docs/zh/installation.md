---

title: Installation
lang: zh

## 系统要求

- **Python** ≥ 3.8 (CPython)
- **Platform**: Linux, macOS, Windows

## 从源码安装

该包尚未发布到 PyPI。从源码安装：

```bash
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

包以 **ABI3 wheel** 格式构建，单个 wheel 支持 Python 3.8 到 3.15 — 无需重新编译。

## 自由线程 Python (cp314t)

CPython 3.14t 的自由线程（无 GIL）wheel 使用 `--no-default-features` 构建，因此**不包含** NumPy 集成：在自由线程构建上对 `numpy.ndarray` 调用 `safe_dump` 会抛出 `YamlTypeError`。GIL 构建（Python 3.8–3.15）保留完整的 ndarray 序列化支持。

## 快速验证

```python
import pyrs_yaml

# 检查版本
print(pyrs_yaml.__version__)

# 快速测试
doc = pyrs_yaml.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ 安装验证成功")
```

## 运行测试

```bash
# Rust 测试
cargo test

# Python 测试
uv run --frozen pytest tests/
```
