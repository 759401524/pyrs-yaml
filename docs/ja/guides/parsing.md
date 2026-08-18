---
title: YAML のパース
description: pyrs-yaml で YAML をパースするすべての方法を説明します。基本パース、PyYAML 互換パース、入力型、エラーハンドリングをカバーします。
tags:
  - docs
status: new
---

このガイドでは、pyrs-yaml で YAML をパースするすべての方法を説明します。

## 基本パース

### YAML 文字列のパース

```python title="文字列をパース"
import pyrs_yaml

doc = pyrs_yaml.parse("key: value")  # (1)!
print(doc.get("key"))  # value
```

1. `parse()` は [`YamlDocument`](../api/yaml-document.md) を返します。コメント・アンカー・フォーマットを保持します。

#### オプション付きパース

```python title="オプション付きパース"
# マージキーの解決を無効化（<<: *alias をそのまま保持）
doc = pyrs_yaml.parse(yaml_text, resolve_merges=False)
```

#### YAML ファイルのパース

```python title="ファイルをパース"
doc = pyrs_yaml.parse_file("config.yaml")
print(doc.get("name"))
```

#### 複数ドキュメントのパース

```python title="複数ドキュメントをパース"
# --- 区切りの YAML
yaml_text = """
---
name: first
---
name: second
"""

docs = pyrs_yaml.parse_all_docs(yaml_text)
print(len(docs))  # 2
print(docs[0].get("name"))  # first
print(docs[1].get("name"))  # second
```

## PyYAML 互換パース

!!! tip "PyYAML 互換の解析"
    `safe_load` は PyYAML と同じ API を提供するため、既存のコードベースからの移行が容易です。

```python title="PyYAML 互換パース"
# ネイティブ Python 型を返す（dict, list, str, int など）
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# 複数ドキュメント
docs = pyrs_yaml.safe_loads("a: 1\n---\nb: 2")
print(len(docs))  # 2
```

## 受け付ける入力型

pyrs-yaml は 3 種類の入力をサポートします。

- :material-language-python: **`str`** — 標準 YAML 文字列
- :material-binary: **`bytes`** — 有効な UTF-8 エンコードバイト列
- :material-format-list-bulleted: **`str` に BOM あり** — 正しく処理される

=== "str"

    ```python title="str 入力"
    doc = pyrs_yaml.parse("key: value")
    ```

=== "bytes"

    ```python title="bytes 入力"
    doc = pyrs_yaml.parse(b"key: value")
    ```

## エラーハンドリング

```python title="エラーハンドリング"
try:
    doc = pyrs_yaml.parse("invalid: yaml: [")
except pyrs_yaml.YamlParseError as e:
    print(f"パースエラー: {e}")
```

## サポートされるデータ型

pyrs-yaml はすべての YAML 1.2 スカラータイプを正しくパースします。

| 型 | 例 | Python 型 |
|---|-----|----------|
| :material-format-text: 文字列 | `hello` | `str` |
| :material-numeric: 整数 | `42`, `0x1A`, `0o77` | `int` |
| :material-decimal: 浮動小数点 | `3.14`, `1.23e-4` | `float` |
| :material-toggle-switch: ブーリアン | `true`, `false` | `bool` |
| :material-null: Null | `null`, `~` | `None` |
| :material-infinity: 無限大 | `.inf`, `-.inf` | `float` |
| :material-alphabetical: NaN | `.nan` | `float` |

---

### 関連項目

- [シリアライズ](serialization.md) — ドキュメントを YAML 文字列に変換
- [インプレース編集](editing.md) — フォーマットを失わずに編集
- [カスタムスキーマ](custom-schema.md) — カスタム型解決ルールを定義
