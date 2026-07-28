---

title: Installation
lang: ja

## インストール

### 必須要件

- **Python** ≥ 3.8 (CPython)
- **プラットフォーム**: Linux、macOS、Windows

### ソースからインストール

パッケージはまだ PyPI に掲載されていません。ソースからインストール：

```bash
uv run --frozen maturin develop --release
```

パッケージは **ABI3 ホイール** としてビルドされており、単一のホイールで Python 3.8 から 3.15 まで対応 — 再コンパイル不要。

### 開発用インストール

ソースからインストール（開発またはテスト用）：

```bash
git clone https://github.com/759401524/pyyaml-rs.git
cd pyyaml-rs
uv run --frozen maturin develop --release
```

### インストールの確認

```python
import pyyaml_rs

# Check version
print(pyyaml_rs.__version__)  # e.g., "0.2.0"

# Quick smoke test
doc = pyyaml_rs.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ Installation verified")
```
