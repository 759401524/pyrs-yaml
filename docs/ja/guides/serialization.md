---
title: シリアライゼーション
description: Python オブジェクトと YamlDocument インスタンスを YAML 文字列に変換する方法を説明します。
tags:
  - docs
status: new
---

Python オブジェクトと `YamlDocument` インスタンスを YAML 文字列に変換します。

## 基本シリアライゼーション

### `YamlDocument.to_yaml()`

```python title="to_yaml()"
doc = pyrs_yaml.parse("key: value")
yaml_str = doc.to_yaml()  # (1)!
print(yaml_str)  # key: value\n
```

1. `to_yaml()` はすべてのコメント・アンカー・フォーマットを保持してシリアライズします。

#### `YamlDocument.to_yaml_with_options()`

```python title="to_yaml_with_options()"
doc = pyrs_yaml.parse("key: value")

# カスタムインデントとドキュメントマーカー
yaml_str = doc.to_yaml_with_options(
    indent_size=4,  # インデントレベルあたり 4 スペース
    explicit_start=True,  # 先頭に "---" を追加
    explicit_end=True,  # 末尾に "..." を追加
    sort_keys=True,  # キーをアルファベット順にソート
)
```

#### PyYAML 互換シリアライゼーション

```python title="PyYAML 互換シリアライゼーション"
# dict を YAML 文字列に
yaml_str = pyrs_yaml.safe_dump({"database": {"host": "localhost", "port": 5432}})

# safe_dumps (エイリアス) も利用可能
yaml_str = pyrs_yaml.safe_dumps({"key": "value"})
```

## Python オブジェクトを YAML に変換

### `from_dict()`

```python title="from_dict()"
yaml_str = pyrs_yaml.from_dict({"name": "Alice", "age": 30, "tags": ["admin", "user"]})
```

#### `from_json()`

```python title="from_json()"
yaml_str = pyrs_yaml.from_json('{"key": "value"}')
```

#### `dump_file()`

```python title="dump_file()"
# Python オブジェクトを直接 YAML ファイルに書き込み
pyrs_yaml.dump_file({"config": {"debug": True, "log_level": "info"}}, "output.yaml")
```

## 出力形式

pyrs-yaml は複数の出力先にシリアライズできます。

=== "文字列"

    ```python title="YAML 文字列"
    yaml_str = pyrs_yaml.safe_dump({"key": "value"})
    ```

=== "ファイル"

    ```python title="YAML ファイル"
    pyrs_yaml.dump_file({"key": "value"}, "output.yaml")
    ```

=== "ドキュメント"

    ```python title="YamlDocument"
    doc = pyrs_yaml.parse("key: value")
    yaml_str = doc.to_yaml()
    ```

## サポートされる入力型

| Python 型 | YAML 出力 |
|-----------|----------|
| :material-language-python: `dict` | YAML マッピング |
| :material-format-list-numbered: `list` | YAML シーケンス |
| :material-format-text: `str` | Plain または引用符付きスカラー |
| :material-numeric: `int` | Plain 整数 |
| :material-decimal: `float` | Plain 浮動小数点 |
| :material-toggle-switch: `bool` | `true` / `false` |
| :material-null: `None` | `null` |

## 往復保存

```python title="往復保存"
# 最大の利点：フォーマットが保持される
original = """
# サーバー設定
server:
  host: 0.0.0.0
  port: 8080  # メインポート

database: &db
  host: localhost

api:
  <<: *db
"""

doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# コメント、アンカー、マージキーが保持される
assert "# サーバー設定" in output
assert "&db" in output
assert "<<: *db" in output
```
