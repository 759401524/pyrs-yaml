---

title: シリアライゼーション
lang: ja

## シリアライゼーション

Python オブジェクトと `YamlDocument` インスタンスを YAML 文字列に変換します。

### 基本シリアライゼーション

#### `YamlDocument.to_yaml()`

```python
doc = pyrs_yaml.parse("key: value")
yaml_str = doc.to_yaml()
print(yaml_str)  # key: value\n
```

#### `YamlDocument.to_yaml_with_options()`

```python
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

```python
# dict を YAML 文字列に
yaml_str = pyrs_yaml.safe_dump({"database": {"host": "localhost", "port": 5432}})

# safe_dumps (エイリアス) も利用可能
yaml_str = pyrs_yaml.safe_dumps({"key": "value"})
```

### Python オブジェクトを YAML に変換

#### `from_dict()`

```python
yaml_str = pyrs_yaml.from_dict({"name": "Alice", "age": 30, "tags": ["admin", "user"]})
```

#### `from_json()`

```python
yaml_str = pyrs_yaml.from_json('{"key": "value"}')
```

#### `dump_file()`

```python
# Python オブジェクトを直接 YAML ファイルに書き込み
pyrs_yaml.dump_file({"config": {"debug": True, "log_level": "info"}}, "output.yaml")
```

### サポートされる入力型

| Python 型 | YAML 出力 |
|-----------|----------|
| `dict` | YAML マッピング |
| `list` | YAML シーケンス |
| `str` | Plain または引用符付きスカラー |
| `int` | Plain 整数 |
| `float` | Plain 浮動小数点 |
| `bool` | `true` / `false` |
| `None` | `null` |

### 往復保存

```python
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
