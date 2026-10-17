---
title: Installation
description: pyrs-yaml のインストール方法、必須要件、フリースレッドビルドの制約、インストール確認手順を説明します。
tags:
  - docs
status: new
---

## 必須要件

| :material-language-python: 要件 | 詳細 |
|---|---|
| **Python** | ≥ 3.8 (CPython)、free-threaded 3.14t（ビルド済みホイール）と 3.15t（ソースビルド）を含む |
| :material-monitor: **プラットフォーム** | Linux、macOS、Windows |
| :material-hammer-wrench: **ビルド** | Rust ツールチェーン（ソースビルドのみ） |

## PyPI からインストール

パッケージは PyPI に公開されています。pip でインストール：

```bash title="PyPI からインストール"
pip install pyrs-yaml
```

GIL ビルドは単一の **`cp38-abi3` ホイール**として提供され、Python 3.8 から 3.15 まで再コンパイル不要で動作します。

## ソースからインストール

開発用または最新の未公開変更用：

```bash title="ソースからインストール"
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

## フリースレッド Python (cp314t と cp315t)

CPython 3.14t と 3.15t 向けのフリースレッド（GIL なし）ビルドは NumPy 統合を含みます。環境に NumPy がインストールされている場合、`safe_dump` / `from_dict` は `numpy.ndarray` 値を通常通りシリアライズします。NumPy が存在しない場合、統合は非アクティブになり、呼び出しはデフォルトのオブジェクトハンドラにフォールスルーします。GIL ビルド（Python 3.8–3.15）では完全な ndarray シリアライズが利用できます。

フリースレッドビルドの安定 ABI（`abi3t`）は 3.15 から始まるため、3.14t と 3.15t は同じホイールを共有できません。公開されているフリースレッドホイールは Windows（x64/x86/arm64）、macOS（x86_64/arm64）、Linux x86_64 向けの `cp314t` で、PyPI には 3.15t 向けのビルド済みホイールがまだありません。CPython 3.15t はソースビルド（`uv run --frozen maturin develop --release`）でサポートされ、CI もこの経路を実行しています。

!!! note "NumPy は実行時に自動検出されます"
    NumPy 統合はすべてのホイール（GIL およびフリースレッド）にコンパイルされていますが、NumPy がインポート可能な場合にのみアクティブになります。NumPy がインストールされていない場合、`numpy.ndarray` に `safe_dump` を呼ぶと `YamlTypeError` が発生します（値が認識された型ではないため）。

## 開発用インストール

ソースからインストール（開発またはテスト用）：

```bash title="ソースからインストール"
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

## インストールの確認

???+ tip "インストールの確認"
    以下のコードスニペットを実行して、インストールが正しく完了したか確認できます。

```python title="インストールの確認"
import pyrs_yaml

# Check version
print(pyrs_yaml.__version__)

# Quick smoke test
doc = pyrs_yaml.parse("key: value")
assert doc.to_yaml() == "key: value\n"
print("✓ Installation verified")
```
