---

title: モジュール リファレンス
lang: ja

`pyrs_yaml` モジュールの完全な API リファレンス。

## コア関数

### `parse()`

YAML 文字列またはバイト列をパースして `YamlDocument` に変換します。

```python
parse(yaml: str | bytes, resolve_merges: bool = True) -> YamlDocument
```

**パラメータ:**

- `yaml` — `str` または `bytes` の YAML コンテンツ
- `resolve_merges` — パース後にマージキー (`<<: *alias`) を解決するかどうか (デフォルト: `True`)

**戻り値:** パースされた YAML を含む `YamlDocument`

**スロー:**

- `YamlParseError` — 無効な YAML 構文
- `TypeError` — 入力が `str` または `bytes` でない

**例:**

```python
doc = pyrs_yaml.parse("key: value")
doc = pyrs_yaml.parse(b"key: value")
doc = pyrs_yaml.parse(yaml_str, resolve_merges=False)
```

#### `parse_file()`

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

#### `parse_all_docs()`

文字列から複数の YAML ドキュメントをパースします。

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**戻り値:** `YamlDocument` オブジェクトのリスト

**例:**

```python
docs = pyrs_yaml.parse_all_docs("a: 1\n---\nb: 2")
```

## PyYAML 互換関数

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

#### `safe_loads()`

複数の YAML ドキュメントをパースします。

```python
safe_loads(yaml: str) -> list[dict[str, Any] | list[Any]]
```

**以下と同等:** PyYAML の `yaml.safe_loads()`

#### `safe_dump()`

Python オブジェクトを YAML にシリアライズします。

```python
safe_dump(data: dict[str, Any] | list[Any] | ndarray) -> str
```

**以下と同等:** PyYAML の `yaml.safe_dump()`

**サポートされる入力型:** `dict`, `list`, `str`, `int`, `float`, `bool`, `None`, および **`numpy.ndarray`** (すべての次元と数値 dtype: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`)

#### `safe_dumps()`

`safe_dump()` のエイリアス。

```python
safe_dumps(data: dict[str, Any] | list[Any] | ndarray) -> str
```

## 変換関数

### `from_dict()`

Python dict を YAML 文字列に変換します。dict の値として `numpy.ndarray` も受け付けます。

```python
from_dict(data: dict[str, Any]) -> str
```

#### `from_json()`

JSON 文字列を YAML 文字列に変換します。

```python
from_json(json_str: str) -> str
```

#### `dump_file()`

Python オブジェクトを YAML にシリアライズしてファイルに書き込みます。`dict`, `list`, または `numpy.ndarray` を受け付けます。

```python
dump_file(data: Any, path: str) -> None
```

## Pydantic 統合

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

## タグレジストリ

### `register_tag()`

カスタムタグハンドラを登録します。デコレータ形式と命令形式の両方をサポートします。

```python
register_tag(name: str, handler: Callable | None = None, priority: int = 0) -> Callable
```

**例（デコレータ）:**

```python
@pyrs_yaml.register_tag("!custom")
def handler(node):
    return f"custom:{node}"
```

**例（命令形式）:**

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

## コンプライアンス

### `compliance_report()`

YAML テストスイートのコンプライアンスレポートを計算します。

```python
compliance_report() -> dict
```

YAML テストスイートの合格率とテストごとの結果を返します。

## ストリーミングイベント

### `parse_stream()`

YAML をインクリメンタルにパースし、生のイベント dict を生成します。

```python
parse_stream(yaml: str) -> StreamIterator
```

各ステップで 1 つのイベント dict を生成する `StreamIterator` を返します。`YAML().load_stream()`（Python 値に解決される）とは異なり、生のトークンストリームを公開します。

## 非同期関数

`asyncio.run_in_executor` を使用した非同期 I/O ラッパー。イベントループコンテキストではノンブロッキング。

### `safe_dumps_async()`

Python オブジェクトを YAML 文字列にシリアライズ (非同期)。

```python
async def safe_dumps_async(data: Any) -> str
```

#### `safe_dump_async()`

Python オブジェクトを stdout に YAML として出力 (非同期)。

```python
async def safe_dump_async(data: Any) -> None
```

#### `safe_loads_async()`

YAML 文字列をネイティブ Python オブジェクトにパース (非同期)。

```python
async def safe_loads_async(yaml: str, schema: str = "core") -> Any
```

#### `safe_load_async()`

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

## Markdown Front Matter

### `read_markdown()`

Markdown ファイルから YAML Front Matterを抽出します。

```python
read_markdown(path: str, schema: str = "core", max_depth: int = 1000) -> tuple[dict[str, Any] | None, str]
```

**戻り値:** `(frontmatter_dict, content_string)`。Front Matterがない場合、`frontmatter` は `None`。

#### `read_markdown_str()`

Markdown 文字列から YAML Front Matterを抽出します。

```python
read_markdown_str(content: str, schema: str = "core", max_depth: int = 1000) -> tuple[dict[str, Any] | None, str]
```

## i18n 関数

### `set_language()`

エラーメッセージの言語を設定します。

```python
set_language(lang: str) -> None
```

サポート: `"en"`, `"zh-CN"`, `"ja-JP"`, `"ko-KR"`

#### `get_language()`

現在の言語を取得します。

```python
get_language() -> str
```

#### `list_languages()`

すべてのサポートされる言語を一覧表示します。

```python
list_languages() -> list[str]
```

#### `detect_language()`

環境変数からユーザーの優先言語を自動検出します。

```python
detect_language() -> str
```

#### `negotiate_language()`

BCP 47 言語ネゴシエーション。

```python
negotiate_language(user_locales: list[str], default: str = "en") -> str
```

## 例外

- `YamlParseError` — YAML パースエラー (`ValueError` を継承)
- `YamlSerializeError` — YAML シリアライズエラー (`ValueError` を継承)
- `YamlTypeError` — 型変換エラー (`TypeError` を継承)
- `YamlValidateError` — JSON Schema 検証エラー (`ValueError` を継承)
- `YamlEditError` — インプレース編集エラー (`ValueError` を継承)
- `YamlPathError` — YAML パスエラー (`ValueError` を継承)
- `YamlDocumentError` — 陳腐化した `Node` アクセスエラー (`Exception` を継承)

詳細は [例外](exceptions.md) ページを参照してください。

## バージョン

```python
__version__ = "0.6.0"
```
