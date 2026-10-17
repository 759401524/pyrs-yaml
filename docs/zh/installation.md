---
title: Installation
description: pyrs-yaml 的安装方法和系统要求，包括从源码安装、自由线程 Python 支持说明和验证步骤。
tags:
  - docs
status: new
---

## 系统要求

| :material-language-python: 要求 | 详情 |
|---|---|
| **Python** | ≥ 3.8 (CPython)，包括 free-threaded 3.14t 与 3.15t（提供预编译 wheel） |
| :material-monitor: **平台** | Linux、macOS、Windows |
| :material-hammer-wrench: **构建** | Rust 工具链（仅源码构建需要） |

### 从 PyPI 安装

该包已发布到 PyPI。使用 pip 安装：

```bash title="从 PyPI 安装"
pip install pyrs-yaml
```

GIL 构建发布单个 **`cp38-abi3` wheel**，覆盖 Python 3.8 到 3.15 — 无需重新编译。

### 从源码安装

用于开发或获取最新未发布更改：

```bash title="从源码安装"
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

## 自由线程 Python (cp314t 与 cp315t)

CPython 3.14t 与 3.15t 的自由线程（无 GIL）构建包含 NumPy 集成。当环境中安装了 NumPy 时，`safe_dump` / `from_dict` 正常序列化 `numpy.ndarray`；当 NumPy 不存在时，集成自动停用，调用回退到默认对象处理。GIL 构建（Python 3.8–3.15）保留完整的 ndarray 序列化支持。

自由线程构建的稳定 ABI（`abi3t`）从 3.15 才开始，因此 3.14t 与 3.15t 无法共用同一个 wheel。已发布的自由线程 wheel 为面向 3.14t 的 `cp314t` 与面向 3.15t 及更高版本的 `cp315-abi3.abi3t`，覆盖 Windows（x64/x86/arm64）、macOS（x86_64/arm64）和 Linux glibc（x86_64/arm64）。musllinux 与其余 Linux 架构通过源码构建支持（`uv run --frozen maturin develop --release`），这也是 CI 覆盖的路径。

!!! note "NumPy 运行时自动检测"
    NumPy 集成编译进每个 wheel（GIL 和自由线程），但仅在 NumPy 可导入时激活。若未安装 NumPy，对 `numpy.ndarray` 调用 `safe_dump` 会抛出 `YamlTypeError`（值不是受支持类型）。

## 快速验证

```python title="验证安装"
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

=== "Rust"

    ```bash
    cargo nextest run --all
    ```

=== "Python"

    ```bash
    uv run --frozen pytest tests/
    ```
