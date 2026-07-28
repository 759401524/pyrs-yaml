---

title: YamlDocument クラス
lang: ja

## YamlDocument クラス

### 概要

`YamlDocument` は pyyaml-rs のコアクラスで、解析済みの YAML ドキュメントを保持します。`IndexMap` を使用したカスタム AST により、**100% ラウンドトリップ**、**完全なキー順序保持**、**ネストされたコメントの保持**、**詳細なメタデータ**を実現します。

```python
class YamlDocument:
    """pyyaml-rs のコアクラス。"""
    # ... C 拡張で実装 ...
```

### コンストラクター

#### `YamlDocument()`

内部コンストラクター。ユーザーが直接呼び出すことはありません。`pyyaml_rs.parse()` から返されます。

### プロパティ

- `version` — YAML ドキュメントバージョン
- `schema` — スキーマ（`core`, `failsafe`, `json`）
- `tags` — タグ一覧
- `anchors` — アンカー一覧
- `source` — YAML ソーステキスト

### メソッド

#### `to_yaml()`

ドキュメントを YAML 文字列に変換します。

```python
to_yaml(
    indent: int = 2,
    allow_unicode: bool = True,
    default_flow_style: bool = False,
    sort_keys: bool = False,
    width: int = 80,
    resolve_aliases: bool = True,
    strip_comments: bool = False,
    preserve_quotes: bool = True,
) -> str
```

**パラメータ:**

- `indent` — インデントスペース数（デフォルト: 2）
- `allow_unicode` — Unicode 文字を許可（デフォルト: True）
- `default_flow_style` — デフォルトでフロースタイルを使用（デフォルト: False）
- `sort_keys` — キーをソート（デフォルト: False）
- `width` — 折り返し幅（デフォルト: 80）
- `resolve_aliases` — エイリアスを解決（デフォルト: True）
- `strip_comments` — コメントを除去（デフォルト: False）
- `preserve_quotes` — クォートを保持（デフォルト: True）

**戻り値:** YAML 文字列

**例:**

```python
doc = pyyaml_rs.parse("key: value\n# comment")
yaml_str = doc.to_yaml()
```

#### `to_dict()`

Python dict/list に変換します。エイリアス参照を解決し、ネイティブ Python タイプを返します。

```python
to_dict() -> dict[str, Any] | list[Any]
```

**戻り値:** 辞書またはリスト

**例:**

```python
doc = pyyaml_rs.parse("key: value")
data = doc.to_dict()  # {'key': 'value'}
```

#### `get()`

キーで値を取得します（マッピングルート用）。

```python
get(key: str, default: Any = None) -> Any
```

**戻り値:** 値、見つからない場合はデフォルト

#### `type()`

ルートノードの型を文字列で取得します。

```python
type() -> str
```

**戻り値:** 型名（`"mapping"`, `"sequence"`, `"scalar"`）

#### `to_json()`

ドキュメントを JSON 文字列にシリアライズします。

```python
to_json(indent: int = 2) -> str
```

**戻り値:** JSON 文字列

#### `validate()`

JSON Schema に基づいてドキュメントの内容を検証します。

```python
validate(schema: dict[str, Any]) -> None
```

**スロー:** `YamlValidateError` — 検証エラー

#### `reload()`

保存されたソーステキストをその場で再パースし、スキーマやマージ動作の変更を可能にします。

```python
reload(schema: str = "core", resolve_merges: bool = True) -> None
```

#### `source_text()`

このドキュメントの作成に使用された元の YAML ソーステキストを返します。

```python
source_text() -> str
```

**戻り値:** YAML ソース文字列

### ダンダー メソッド

#### `__getitem__()`

キー（マッピング）またはインデックス（シーケンス）でアクセスします。

```python
doc = pyyaml_rs.parse("key: value")
value = doc["key"]  # 'value'
```

#### `__contains__()`

キーが存在するか確認します。

```python
"key" in doc  # True
```

#### `__len__()`

アイテム数を取得します。

```python
len(doc)
```

#### `__iter__()`

キー（マッピング）または値（シーケンス）を反復します。

```python
for key in doc:
    print(key)
```

#### `__repr__()`

デバッグ表現。

```python
repr(doc)  # "YamlDocument({key: value})"
```

#### `__str__()`

文字列表現。

```python
str(doc)  # "YamlDocument({key: value})"
```

#### `__eq__()`

等値比較。2つの `YamlDocument` が同じ内容を持つ場合、true を返します。

```python
doc1 == doc2  # True or False
```

**例:**

```python
import pyyaml_rs

# マッピング
doc = pyyaml_rs.parse("name: Alice\nage: 30")
print(doc["name"])  # Alice
print(len(doc))     # 2

# シーケンス
doc = pyyaml_rs.parse("- item1\n- item2")
print(doc[0])  # item1

# ネストされたアクセス
doc = pyyaml_rs.parse("user:\n  name: Alice")
print(doc["user"]["name"])  # Alice
```
