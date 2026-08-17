---
title: 开发设置
description: 设置 pyrs-yaml 开发环境的完整指南，包含前提条件、安装步骤和开发工作流。
tags:
  - docs
status: new
---

设置您的环境以贡献 pyrs-yaml。

## 前提条件

- **Python** ≥ 3.8 (CPython)
- **Rust** ≥ 1.70（通过 [rustup](https://rustup.rs/)）
- **Git**
- **uv**（推荐）或 **pip**
- **NumPy** — 运行 NumPy 序列化测试套件所需 (`pytest tests/test_numpy.py`)

## 克隆和安装

```bash
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml

# 使用 uv（推荐）
uv sync

# 或使用 pip
pip install maturin
maturin develop --release
```

## 验证安装

```bash
# 运行 Rust 测试
cargo test

# 运行 Python 测试（使用 uv lockfile 确保可重复依赖）
uv run --frozen pytest tests/

# 运行基准测试
cargo bench
```

## 项目结构

```text
pyrs-yaml/
├── src/
│   ├── lib.rs              # PyO3 模块定义
│   ├── ast.rs              # 自定义 AST (CustomNode)
│   ├── parser/
│   │   ├── mod.rs          # granit-parser 集成
│   │   └── yaml/           # YAML 特定解析
│   │       ├── comment.rs  # 注释提取
│   │       ├── merge.rs    # 合并键解析
│   │       ├── scalar.rs   # 标量解析
│   │       └── types.rs    # YAML 1.2 类型解析
│   └── serializer.rs       # YAML 序列化
├── python/pyrs_yaml/
│   ├── __init__.py         # Python 包初始化
│   ├── pyrs_yaml.pyi       # 类型桩文件
│   └── py.typed            # PEP 561 标记
├── tests/                  # Python 测试套件
├── benches/                # Rust 基准测试
└── docs/                   # 文档 (mkdocs)
```

## 构建命令

```bash
# 构建 Python 扩展（使用 uv lockfile）
uv run --frozen maturin develop --release

# 构建 wheel
uv run --frozen maturin build --release --out dist

# 带调试信息构建
cargo build
```

## 开发工作流

1. **首先编写测试** (TDD)
2. 在 `src/` 中**实现变更**
3. **运行 `cargo test`** 验证 Rust 测试
4. **运行 `uv run --frozen pytest tests/`** 验证 Python 测试
5. **运行 `cargo clippy -- -D warnings`** 检查代码质量
6. **运行 `cargo fmt`** 格式化代码
