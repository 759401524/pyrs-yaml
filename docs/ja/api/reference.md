---
title: モジュール リファレンス
description: pyrs_yaml モジュールの完全な API リファレンス。コア関数、変換関数、タグレジストリ、例外をカバーします。
tags:
  - docs
status: new
---

!!! tip "バージョン互換性"
    pyrs-yaml は ABI3 アプローチにより、単一のホイールで Python 3.8 から 3.15 まで対応しています。再コンパイルは不要です。

`pyrs_yaml` モジュールの完全な API リファレンス。

## :material-code-braces: コア関数

### `parse()`

YAML 文字列またはバイト列をパースして `YamlDocument` に変換します。

```python
parse(yaml: str | bytes, resolve_merges: bool = True, schema: str | dict = "core", max_depth: int = 1000, allow_duplicate_keys: bool = False) -> YamlDocument
```

**パラメータ:**

- `yaml` — `str` または `bytes` の YAML コンテンツ
- `resolve_merges` — パース後にマージキー (`<<: *alias`) を解決するかどうか (デフォルト: `True`)
- `schema` — スキーマ名 (`"core"`, `"json"`, `"failsafe"`, `"yaml1.1"` または登録済みカスタム名)、またはインラインスキーマ dict（[YAML スキーマ言語](#yaml-schema-language) 参照）
- `max_depth` — 最大ネスト深度 (デフォルト: `1000`)
- `allow_duplicate_keys` — 重複マッピングキーを許可するかどうか (デフォルト: `False`)

**戻り値:** パースされた YAML を含む `YamlDocument`

**スロー:**

- `YamlParseError` — 無効な YAML 構文
- `YamlTypeError` — 指定されたスキーマが見つかりません
- `TypeError` — 入力が `str` または `bytes` でない

**例:**

```python
doc = pyrs_yaml.parse("key: value")
doc = pyrs_yaml.parse(b"key: value")
doc = pyrs_yaml.parse(yaml_str, schema="json")
doc = pyrs_yaml.parse("addr: 0xFF", schema={"extends": "core", "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}]})
```

### `parse_file()`

YAML ファイルをパースします。

```python
parse_file(path: str) -> YamlDocument
```

**パラメータ:**

- `path` — YAML ファイルへのパス

**戻り値:** `YamlDocument`

**スロー:**

- `IOError` — ファイルが見つからないまたは読み取り不可
- `YamlParseError` — 無効な YAML

**例:**

```python
doc = pyrs_yaml.parse_file("config.yaml")
```

### `parse_all_docs()`

文字列から複数の YAML ドキュメントをパースします。

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**戻り値:** `YamlDocument` オブジェクトのリスト

**例:**

```python
docs = pyrs_yaml.parse_all_docs("a: 1\n---\nb: 2")
```

## :material-swap-horizontal: PyYAML 互換関数

### `safe_load()`

YAML をパースしてネイティブ Python 型を返します。

```python
safe_load(yaml: str) -> dict[str, Any] | list[Any]
```

**以下と同等:** PyYAML の `yaml.safe_load()`

**例:**

```python
data = pyrs_yaml.safe_load("key: value")  # {'key': 'value'}
```

### `safe_loads()`

複数の YAML ドキュメントをパースします。

```python
safe_loads(yaml: str) -> list[dict[str, Any] | list[Any]]
```

**以下と同等:** PyYAML の `yaml.safe_loads()`

### `safe_dump()`

Python オブジェクトを YAML にシリアライズします。

```python
safe_dump(data: dict[str, Any] | list[Any] | ndarray) -> str
```

**以下と同等:** PyYAML の `yaml.safe_dump()`

**サポートされる入力型:** `dict`, `list`, `str`, `int`, `float`, `bool`, `None`, および **`numpy.ndarray`** (すべての次元と数値 dtype: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`)

### `safe_dumps()`

`safe_dump()` のエイリアス。

```python
safe_dumps(data: dict[str, Any] | list[Any] | ndarray) -> str
```

## :material-json: 変換関数

### `from_dict()`

Python dict を YAML 文字列に変換します。dict の値として `numpy.ndarray` も受け付けます。

```python
from_dict(data: dict[str, Any]) -> str
```

### `from_json()`

JSON 文字列を YAML 文字列に変換します。

```python
from_json(json_str: str) -> str
```

### `dump_file()`

Python オブジェクトを YAML にシリアライズしてファイルに書き込みます。`dict`, `list`, または `numpy.ndarray` を受け付けます。

```python
dump_file(data: Any, path: str) -> None
```

## :material-pillar: Pydantic 統合

### `dump_pydantic()`

Pydantic モデルを YAML 文字列にシリアライズします。

```python
dump_pydantic(model: BaseModel) -> str
```

`model_dump(mode='json')` を使用して文字列型を保持してから（例：`"10001"` の郵便番号は文字列のまま）、`safe_dump` に委譲します。

**スロー:**

- `ImportError` — pydantic がインストールされていない
- `TypeError` — `model` が Pydantic の `BaseModel` インスタンスでない

**例:**

```python
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


yaml_str = pyrs_yaml.dump_pydantic(User(name="Alice", age=30))
```

### `parse_as()`

YAML 文字列をパースし、Pydantic モデルに対して検証します。

```python
parse_as(model: type[BaseModel], src: str, **yaml_kwargs: Any) -> BaseModel
```

**パラメータ:**

- `model` — Pydantic の `BaseModel` サブクラス
- `src` — パースする YAML 文字列
- `**yaml_kwargs` — `YAML()` コンストラクタに転送されるキーワード引数

**スロー:**

- `ImportError` — pydantic がインストールされていない
- `TypeError` — `model` が Pydantic の `BaseModel` サブクラスでない
- `pydantic.ValidationError` — パースされたデータがモデル検証に失敗

**例:**

```python
user = pyrs_yaml.parse_as(User, "name: Alice\nage: 30")
print(user.name)  # Alice
```

## :material-tag: タグレジストリ {#tag-registry}

### `register_tag()`

カスタムタグハンドラを登録します。デコレータ形式と命令形式の両方をサポートします。

```python
register_tag(name: str, handler: Callable | None = None, priority: int = 0) -> Callable
```

=== "デコレータ"

    ```python
    @pyrs_yaml.register_tag("!custom")
    def handler(node):
        return f"custom:{node}"
    ```

=== "命令形式"

    ```python
    pyrs_yaml.register_tag("!custom", handler_fn, priority=1)
    ```

### `remove_tag()`

タグハンドラを削除します。

```python
remove_tag(name: str) -> None
```

### `clear_tag_handlers()`

登録済みのすべてのタグハンドラを削除します。

```python
clear_tag_handlers() -> None
```

## :material-file-document: YAML スキーマ言語 {#yaml-schema-language}

カスタムスキーマを定義して、プレーンスカラーが Python 型にどのように解決されるかを制御します。

### `register_schema()`

カスタムスキーマを登録します。

```python
register_schema(name: str, schema: str | dict) -> None
```

**パラメーター:**

- `name` — スキーマ名
- `schema` — YAML 文字列または dict（`extends`、`rules`、`validate` キーを含む）

**例:**

```python
import pyrs_yaml

# YAML 文字列からカスタムスキーマを登録
pyrs_yaml.register_schema("hex", """
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
""")

# カスタムスキーマを使用
y = pyrs_yaml.YAML(schema="hex")
doc = y.parse("addr: 0xFF")
assert doc.get("addr") == 255

d = pyrs_yaml.safe_load("addr: 0x1F", schema="hex")
assert d["addr"] == 31
```

### インラインスキーマ dict

登録せずに dict を直接渡す：

```python
d = pyrs_yaml.safe_load(
    "addr: 0xFF",
    schema={
        "extends": "core",
        "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}],
    },
)
```

- **`extends`** — オプションのベーススキーマ（`core`、`json`、`failsafe`、`yaml1.1`）
- **`rules`** — 順序付きの `{pattern, type}` リスト；最初にマッチしたものが適用
- **`validate`** — オプションの構造検証ルール：パス修飾型（`$.port: int`）、コンテナチェック（`sequence_of`、`mapping_of`）、`required` 存在確認；`validate_against_schema(data, schema_yaml)` でドキュメントを検証
- **対応型**：`null`、`bool`、`int`、`float`、`str`
- 組み込み Core スキーマは引き続きゼロコスト `match` ディスパッチを使用（影響なし）
- **ファイル I/O** — `load_schema(name, path)` で YAML ファイルからスキーマを読み込み；`list_schemas()` で登録済みの全スキーマを取得

## :material-puzzle: コミュニティプラグイン {#community-plugins}

カスタム YAML ノードタイプを定義して、シリアライズとデシリアライズに統合します。

### `CustomType`

カスタムタイプの基底クラス。

```python
class CustomType:
    python_type: type

    def from_yaml(self, value: str) -> Any: ...
    def to_yaml(self, obj: Any) -> str: ...
    def can_parse(self, node: CustomNode) -> bool: ...
    def validate(self, obj: Any) -> bool: ...
```

### `register_type()`

カスタムタイプを登録します。

```python
register_type(tag: str, type_handler: CustomType, priority: int = 0) -> None
```

**例:**

```python
from datetime import datetime

class TimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value: str):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj) -> str:
        return obj.isoformat()

pyrs_yaml.register_type("!timestamp", TimestampType())

# ロード：タグ付きスカラー → Python オブジェクト
doc = pyrs_yaml.parse("when: !timestamp 2026-08-11T10:30:00")
assert isinstance(doc.get("when"), datetime)

# ダンプ：Python オブジェクト → タグ付きスカラー
data = {"ts": datetime(2026, 8, 11, 10, 30)}
out = pyrs_yaml.safe_dump(data)
# out には次が含まれる：ts: !timestamp 2026-08-11T10:30:00
```

| メソッド | 説明 |
|---------|------|
| `can_parse(node)` | このタイプが指定された AST ノードを処理するかどうか |
| `from_yaml(value)` | YAML 文字列を Python オブジェクトに変換 |
| `to_yaml(obj)` | Python オブジェクトを YAML 文字列に変換 |
| `validate(obj)` | Python オブジェクトを検証（`bool` を返す） |

### `remove_type()`

登録済みのタイプを削除します。

```python
remove_type(name: str) -> None
```

### `clear_type_handlers()`

登録済みのすべてのタイプハンドラを削除します。

```python
clear_type_handlers() -> None
```

## :material-check-decagram: コンプライアンス

### `compliance_report()`

YAML テストスイートのコンプライアンスレポートを計算します。

```python
compliance_report() -> dict
```

YAML テストスイートの合格率とテストごとの結果を返します。

## :material-wave: ストリーミングイベント

### `parse_stream()`

YAML をインクリメンタルにパースし、生のイベント dict を生成します。

```python
parse_stream(yaml: str) -> StreamIterator
```

各ステップで 1 つのイベント dict を生成する `StreamIterator` を返します。`YAML().load_stream()`（Python 値に解決される）とは異なり、生のトークンストリームを公開します。

### `YamlStream` { #yamlstream }

`YamlStream` クラスは、`YAML().load_stream()` と `YAML().load_stream_file()` が返す遅延イベントイテレータです。ドキュメント全体をメモリに読み込まずに、パース済みのイベント dict を一度に 1 つ生成します。

```python
stream = yaml.load_stream_file("large.yaml")
for event in stream:
    print(event)
```

完全な API の詳細は [`YamlStream`](yaml-instance.md) を参照してください。

## :material-clock-fast: 非同期関数

`asyncio.run_in_executor` を使用した非同期 I/O ラッパー。イベントループコンテキストではノンブロッキング。

### `safe_dumps_async()`

Python オブジェクトを YAML 文字列にシリアライズ (非同期)。

```python
async def safe_dumps_async(data: Any) -> str
```

### `safe_dump_async()`

Python オブジェクトを stdout に YAML として出力 (非同期)。

```python
async def safe_dump_async(data: Any) -> None
```

### `safe_loads_async()`

YAML 文字列をネイティブ Python オブジェクトにパース (非同期)。

```python
async def safe_loads_async(yaml: str, schema: str = "core") -> Any
```

### `safe_load_async()`

YAML 文字列をネイティブ Python オブジェクトにパース (非同期)。

```python
async def safe_load_async(yaml: str, schema: str = "core") -> Any
```

**例:**

```python
import asyncio, pyrs_yaml


async def main():
    yaml = await pyrs_yaml.safe_dumps_async({"a": 1})
    data = await pyrs_yaml.safe_loads_async(yaml)
    print(data)  # {'a': 1}


asyncio.run(main())
```

## :material-page-layout-body: Markdown Front Matter {#markdown-frontmatter}

### `read_markdown()`

Markdown ファイルから YAML Front Matterを抽出します。

```python
read_markdown(path: str, schema: str = "core", max_depth: int = 1000) -> tuple[dict[str, Any] | None, str]
```

**戻り値:** `(frontmatter_dict, content_string)`。Front Matterがない場合、`frontmatter` は `None`。

### `read_markdown_str()`

Markdown 文字列から YAML Front Matterを抽出します。

```python
read_markdown_str(content: str, schema: str = "core", max_depth: int = 1000) -> tuple[dict[str, Any] | None, str]
```

## :material-translate: i18n 関数 {#i18n-functions}

### `set_language()`

エラーメッセージの言語を設定します。

```python
set_language(lang: str) -> None
```

サポート: `"en"`, `"zh-CN"`, `"ja-JP"`, `"ko-KR"`

### `get_language()`

現在の言語を取得します。

```python
get_language() -> str
```

### `list_languages()`

すべてのサポートされる言語を一覧表示します。

```python
list_languages() -> list[str]
```

### `detect_language()`

環境変数からユーザーの優先言語を自動検出します。

```python
detect_language() -> str
```

### `negotiate_language()`

BCP 47 言語ネゴシエーション。

```python
negotiate_language(user_locales: list[str], default: str = "en") -> str
```

## :material-bug: 例外

- `YamlParseError` — YAML パースエラー (`ValueError` を継承)
- `YamlSerializeError` — YAML シリアライズエラー (`ValueError` を継承)
- `YamlTypeError` — 型変換エラー (`TypeError` を継承)
- `YamlValidateError` — JSON Schema 検証エラー (`ValueError` を継承)
- `YamlEditError` — インプレース編集エラー (`ValueError` を継承)
- `YamlPathError` — YAML パスエラー (`ValueError` を継承)
- `YamlDocumentError` — 陳腐化した `Node` アクセスエラー (`Exception` を継承)

詳細は [例外](exceptions.md) ページを参照してください。

## :material-information: バージョン

```python
__version__ = "0.14.0"
```
