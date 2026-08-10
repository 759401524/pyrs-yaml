---
title: NumPy ndarray シリアライズガイド
description: NumPy 配列を YAML リストにシリアライズする方法を説明します。ゼロコピー Rust 処理に対応。
tags:
  - docs
status: new
---

!!! warning "フリースレッドビルドは NumPy 非対応"
    フリースレッド（GIL なし）ビルドでは NumPy 統合が含まれないため、`numpy.ndarray` に `safe_dump` を呼ぶと `YamlTypeError` が発生します。GIL ビルド（Python 3.8–3.15）では完全な ndarray シリアライズが利用できます。

NumPy 配列を YAML リストにシリアライズします。ゼロコピー Rust 処理に対応。

## 基本的な使い方

```python
import numpy as np
import pyrs_yaml as y

# 1次元配列
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = y.safe_dump(arr)
# 出力:
# - 1
# - 2
# - 3

# Python リストに戻す
data = y.safe_load(yaml_str)
assert data == [1, 2, 3]
```

## 多次元配列

```python
# 2次元行列
matrix = np.array([[1.0, 2.0], [3.0, 4.0]], dtype="float64")
yaml_str = y.safe_dump(matrix)
data = y.safe_load(yaml_str)
assert data == [[1.0, 2.0], [3.0, 4.0]]

# 3次元キューブ
cube = np.array([[[1, 2], [3, 4]], [[5, 6], [7, 8]]], dtype="int64")
data = y.safe_load(y.safe_dump(cube))
assert data == [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
```

## サポートされる dtype

| NumPy dtype | YAML 出力 | 例 |
|-------------|-----------|------|
| `int8/16/32/64` | 整数 | `42` |
| `uint8/16/32/64` | 整数 | `42` |
| `float32/64` | 浮動小数点 | `3.14` |
| `bool` | ブール値 | `true` / `false` |
| `complex64/128` | 文字列 | `(1+2j)` |

!!! note "複素数"
    YAML にはネイティブの複素数型がありません。複素数値は `(re+imj)` 文字列としてシリアライズされます。`safe_load` は Python の `complex` 型ではなく文字列として返します。

## 特殊値

```python
# NaN
arr = np.array([1.0, float("nan"), 3.0])
data = y.safe_load(y.safe_dump(arr))
assert str(data[1]) == "nan"

# 無限大
arr = np.array([float("inf"), -float("inf")])
data = y.safe_load(y.safe_dump(arr))
assert data[0] == float("inf")
assert data[1] == float("-inf")
```

## 負数の処理

YAML 1.2 仕様では、ブロックシーケンスに `-` で始まるプレーンスカラーを含めることはできません。負数は自動的にシングルクォートで囲まれ、ラウンドトリップ時に正しく解析されます：

```python
arr = np.array([-100, 200], dtype="int16")
data = y.safe_load(y.safe_dump(arr))
assert data == [-100, 200]  # ラウンドトリップ正しく処理
```

## 0次元スカラ配列

0次元配列は1次元にリシェイプされてからシリアライズされ、単一要素リストになります：

```python
scalar = np.array(42, dtype="int32")
data = y.safe_load(y.safe_dump(scalar))
assert data == [42]
```

## 構造体内にネスト

NumPy 配列は dict や list に含めることができます：

```python
data = {"matrix": np.array([[1, 2], [3, 4]]), "label": "test"}
yaml_str = y.safe_dump(data)
loaded = y.safe_load(yaml_str)
assert loaded["matrix"] == [[1, 2], [3, 4]]
```

## サポートされない型

以下の型は `YamlTypeError` をスローします：

- 文字列配列
- オブジェクト配列
- 構造化配列
- 非数値カスタム dtype

## パフォーマンス

- `PyUntypedArray` を使用したゼロコピー dtype ディスパッチ
- `PyArrayDyn<T>` を使用したゼロコピー切片反復
- 切片イテレーション中の Python GIL リリース
- 任意次元を追加割り当てなしでサポート
