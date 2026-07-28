---

title: モジュール リファレンス
lang: ja

## モジュール リファレンス

`pyyaml_rs` モジュールの完全な API リファレンス。

### コア関数

#### `parse()`

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
doc = pyyaml_rs.parse("key: value")
doc = pyyaml_rs.parse(b"key: value")
doc = pyyaml_rs.parse(yaml_str, resolve_merges=False)
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
doc = pyyaml_rs.parse_file("config.yaml")
```

#### `parse_all_docs()`

文字列から複数の YAML ドキュメントをパースします。

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**戻り値:** `YamlDocument` オブジェクトのリスト

**例:**

```python
docs = pyyaml_rs.parse_all_docs("a: 1\n---\nb: 2")
```

### PyYAML 互換関数

#### `safe_load()`

YAML をパースしてネイティブ Python 型を返します。

```python
safe_load(yaml: str) -> dict[str, Any] | list[Any]
```

**以下と同等:** PyYAML の `yaml.safe_load()`

**例:**

```python
data = pyyaml_rs.safe_load("key: value")  # {'key': 'value'}
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

### 変換関数

#### `from_dict()`

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

### 非同期関数

`asyncio.run_in_executor` を使用した非同期 I/O ラッパー。イベントループコンテキストではノンブロッキング。

#### `safe_dumps_async()`

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
import asyncio, pyyaml_rs

async def main():
    yaml = await pyyaml_rs.safe_dumps_async({"a": 1})
    data = await pyyaml_rs.safe_loads_async(yaml)
    print(data)  # {'a': 1}

asyncio.run(main())
```

### Markdown フロントメータ

#### `read_markdown()`

Markdown ファイルから YAML フロントメータを抽出します。

```python
read_markdown(path: str) -> tuple[dict[str, Any] | None, str]
```

**戻り値:** `(frontmatter_dict, content_string)`。フロントメータがない場合、`frontmatter` は `None`。

#### `read_markdown_str()`

Markdown 文字列から YAML フロントメータを抽出します。

```python
read_markdown_str(content: str) -> tuple[dict[str, Any] | None, str]
```

### i18n 関数

#### `set_language()`

エラーメッセージの言語を設定します。

```python
set_language(lang: str) -> None
```

サポート: `"en"`, `"zh-CN"`

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

### 例外

- `YamlParseError` — YAML パースエラー (`ValueError` を継承)
- `YamlSerializeError` — YAML シリアライズエラー (`ValueError` を継承)
- `YamlTypeError` — 型変換エラー (`TypeError` を継承)
- `YamlValidateError` — JSON Schema 検証エラー (`ValueError` を継承)

### バージョン

```python
__version__ = "0.6.0"
```
