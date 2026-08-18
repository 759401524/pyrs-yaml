---
title: MergedView クラス
description: MergedView クラスの API リファレンス。マージキー解決とルート型サポートをカバーします。
tags:
  - docs
status: new
---

## MergedView クラス

`MergedView` クラスは、マージキー（`<<: *anchor`）を解決した `YamlDocument` の読み取り専用ビューを提供します。`doc.merged()` でアクセスできます。

### 概要

```python
class MergedView(Mapping):
    """Read-only view of a YAML document with merge keys resolved."""
```

このビューは `YamlDocument.to_dict()` から遅延構築され、シリアライズ時にアンカーとマージキーを解決します。元の AST は決して変更されません。

### コンストラクタ

#### `MergedView.__init__()`

```python
MergedView.__init__(document: YamlDocument) -> None
```

**Parameters:**

- `document` — A `YamlDocument` インスタンス

ドキュメントのルートがシーケンスの場合、ビューは整数キーのマッピング（`{0: item0, 1: item1, ...}`）に変換します。

### メソッド

#### `__getitem__()`

キーで値にアクセスします。

```python
__getitem__(key: str | int) -> Any
```

子の dict と list は、それぞれ再帰的に `MergedView._DictView` と `MergedView._ListView` でラップされます。

**Example:**

```python
doc = pyrs_yaml.parse("""
defaults: &defaults
  timeout: 30
  retries: 3

config:
  <<: *defaults
  timeout: 60
""")

view = doc.merged()
print(view["config"]["timeout"])  # 60 (overrides merged value)
print(view["config"]["retries"])  # 3 (inherited from merge)
```

#### `__len__()`

トップレベルのアイテム数を返します。

```python
__len__() -> int
```

#### `__iter__()`

トップレベルのキーを反復処理します。

```python
__iter__() -> Iterator[str | int]
```

#### `__repr__()`

```python
__repr__() -> str
```

内部の dict 表現とともに `MergedView({...})` を返します。

#### `get()`

`get()` は `collections.abc.Mapping` から継承され、`get(key, default=None)` を提供します。

```python
get(key: str | int, default: Any = None) -> Any
```

### マージキー解決

キーは以下の優先順位で解決されます（高いほど優先）:

1. マージ元のドキュメントに直接定義されたキー
2. マージされたアンカーからのキー（`<<:` に出現する順序）
3. 後方のアンカーが前方のアンカーを上書き

### ルート型サポート

| Root Type | Behavior |
| --- | --- |
| Mapping | キーはマッピングのキー |
| Sequence | キーは整数インデックス（`0`, `1`, ...） |
| Scalar/Null | `__len__()` は `0` を返す；`__getitem__()` は `KeyError` を発生 |

### 例

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
base: &base
  host: localhost
  port: 8080

prod:
  <<: *base
  host: prod.example.com
  debug: false
""")

merged = doc.merged()
assert merged["base"]["host"] == "localhost"
assert merged["prod"]["host"] == "prod.example.com"  # overridden
assert merged["prod"]["port"] == 8080  # inherited
assert merged["prod"]["debug"] is False  # own key
```
