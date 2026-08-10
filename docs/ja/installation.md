---

title: Installation
lang: ja

## 必須要件

- **Python** ≥ 3.8 (CPython)
- **プラットフォーム**: Linux、macOS、Windows

## ソースからインストール

パッケージはまだ PyPI に掲載されていません。ソースからインストール：

```bash
uv run --frozen maturin develop --release
```

パッケージは **ABI3 ホイール** としてビルドされており、単一のホイールで Python 3.8 から 3.15 まで対応 — 再コンパイル不要。

## フリースレッド Python (cp314t)

CPython 3.14t 向けのフリースレッド（GIL なし）ホイールは `--no-default-features` でビルドされるため、NumPy 統合は**含まれません**：フリースレッドビルドで `numpy.ndarray` に `safe_dump` を呼ぶと `YamlTypeError` が発生します。GIL ビルド（Python 3.8–3.15）では完全な ndarray シリアライズが利用できます。

## 開発用インストール

ソースからインストール（開発またはテスト用）：

```bash
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

## インストールの確認

```python
import pyrs_yaml

# Check version
print(pyrs_yaml.__version__)  # e.g., "0.2.0"

# Quick smoke test
doc = pyrs_yaml.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ Installation verified")
```
