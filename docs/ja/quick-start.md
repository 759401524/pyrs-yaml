---
title: Quick Start
description: pyrs-yaml を数分で使い始めるためのガイド。パース、シリアライズ、ラウンドトリップ、インプレース編集をカバーします。
tags:
  - docs
status: new
---

このガイドでは、pyrs-yaml を数分で使い始める方法を説明します。

## 1. インストール

パッケージはまだ PyPI に掲載されていません。ソースからインストール：

```bash
uv run --frozen maturin develop --release
```

## 2. YAML のパース

```python
import pyrs_yaml

# Parse a YAML string
doc = pyrs_yaml.parse("""
name: Alice
age: 30
email: alice@example.com
""")

# Access values
print(doc.get("name"))  # Alice
print(doc.get("age"))  # 30
print(doc.get("email"))  # alice@example.com
```

## 3. Python オブジェクトへの変換

```python
# Use safe_load for PyYAML-compatible behavior
data = pyrs_yaml.safe_load("""
users:
  - name: Alice
    role: admin
  - name: Bob
    role: user
""")

# Returns native Python types (dict, list, str, int, etc.)
print(data["users"][0]["name"])  # Alice
print(type(data["users"]))  # <class 'list'>
```

## 4. YAML へのシリアライズ

```python
# Convert a Python dict back to YAML
yaml_str = pyrs_yaml.safe_dump({"database": {"host": "localhost", "port": 5432, "name": "mydb"}})
print(yaml_str)
# database:
#   host: localhost
#   port: 5432
#   name: mydb
```

## 5. フォーマットの保持（ラウンドトリップ）

```python
# The key advantage of pyrs-yaml
original = """
# Server configuration
server:
  host: 0.0.0.0
  port: 8080

# Database settings
database: &db
  host: localhost
  port: 5432

# Use the database anchor
api:
  <<: *db
  endpoint: /api/v1
"""

# Parse and re-serialize — comments and anchors preserved
doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# The output matches the input (or is semantically equivalent)
assert "# Server configuration" in output
assert "&db" in output
```

## 6. インプレース編集

```python
# コメントやフォーマットを失わずに解析済みドキュメントを編集
doc = pyrs_yaml.parse("""
server:
  host: localhost  # bind address
  ports:
    - 8080
""")

doc.set("$.server.host", "0.0.0.0")  # パスで置換
doc.append("$.server.ports", 443)  # シーケンスに追加

print(doc.to_yaml())
# server:
#   host: 0.0.0.0  # bind address
#   ports:
#     - 8080
#     - 443
```

完全な API は [インプレース編集ガイド](guides/editing.md) を参照してください。

## 7. ファイルから YAML を読み込む

```python
# Parse a YAML file directly
doc = pyrs_yaml.parse_file("config.yaml")
print(doc.get("name"))
```

## 8. 複数ドキュメント

```python
# Parse multiple YAML documents
yaml_text = """
---

name: config1
value: 1
---
name: config2
value: 2
"""

docs = pyrs_yaml.parse_all_docs(yaml_text)
print(len(docs))  # 2
print(docs[0].get("name"))  # config1
```

## 9. NumPy ndarray サポート

pyrs-yaml は `numpy.ndarray` オブジェクトを直接 YAML にシリアライズできます。これは科学データ、モデルの重み、または多次元配列を人間が読める形式に保存するのに便利です。

```python
import numpy as np
import pyrs_yaml

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyrs_yaml.safe_dump(arr)
print(yaml_str)
# - 1
# - 2
# - 3

# 2-D matrix
matrix = np.array([[1.0, 2.0], [3.0, 4.0]], dtype="float64")
yaml_str = pyrs_yaml.safe_dump(matrix)
print(yaml_str)
# -
#   - 1.0
#   - 2.0
# -
#   - 3.0
#   - 4.0

# Round-trip preserves values
loaded = pyrs_yaml.safe_load(yaml_str)
assert loaded == [[1.0, 2.0], [3.0, 4.0]]
```

### サポートされる NumPy データ型

| NumPy dtype | YAML output | Notes |
|-------------|-------------|-------|
| `int8/16/32/64` | Plain integer | Quoted if negative |
| `uint8/16/32/64` | Plain integer | — |
| `float32/64` | Plain float | Quoted if negative |
| `complex64/128` | `(re+imj)` string | No native YAML complex type |
| `bool` | `true` / `false` | — |

### 10. メタデータの操作（comment, anchor, tag）

```python
doc = pyrs_yaml.parse("key: value")
node = doc.node().find("$.key")
node.set_comment("a note")
node.set_anchor("cfg")
node.set_tag("!custom")
print(doc.to_yaml())
# key: &cfg !custom value  # a note
```

### 11. フォーマットの制御

```python
doc = pyrs_yaml.parse("key: value")
doc.node().find("$.key").set_scalar_style("single_quoted")
```

### 12. スキーマで検証

```python
schema = """\
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
"""
pyrs_yaml.validate_against_schema("port: 8080\n", schema)
```

### 13. 高度な編集

```python
doc.set_many({"$.items[*].active": False})
doc.sort_keys()
```

## 次のステップ

- **[機能](features.md)** — サポートされているすべての YAML 機能を探索
- **[パースガイド](guides/parsing.md)** — 高度なパースオプション
- **[インプレース編集](guides/editing.md)** — フォーマットを失わずにドキュメントを編集
- **[API リファレンス](api/reference.md)** — 完全な API ドキュメント
