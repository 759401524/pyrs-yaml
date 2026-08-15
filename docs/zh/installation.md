---
title: Installation
description: pyrs-yaml 的安装方法和系统要求，包括从源码安装、自由线程 Python 支持说明和验证步骤。
tags:
  - docs
status: new
---

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

CPython 3.14t 的自由线程（无 GIL）wheel 包含 NumPy 集成。当环境中安装了 NumPy 时，`safe_dump` / `from_dict` 正常序列化 `numpy.ndarray`；当 NumPy 不存在时，集成自动停用，调用回退到默认对象处理。GIL 构建（Python 3.8–3.15）保留完整的 ndarray 序列化支持。

!!! note "NumPy 运行时自动检测"
    NumPy 集成编译进每个 wheel（GIL 和自由线程），但仅在 NumPy 可导入时激活。若未安装 NumPy，对 `numpy.ndarray` 调用 `safe_dump` 会抛出 `YamlTypeError`（值不是受支持类型）。

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

???+ tip "验证安装"
    运行以上代码可快速验证 pyrs-yaml 是否正确安装。如果输出版本号且断言通过，说明安装成功。

## 运行测试

```bash
# Rust 测试
cargo test

# Python 测试
uv run --frozen pytest tests/
```
