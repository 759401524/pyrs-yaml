---
title: YAML クラス
description: YAML クラスの API リファレンス。パース、シリアライズ、ストリーミングの各メソッドをカバーします。
tags:
  - docs
status: new
---

## YAML クラス

`YAML` クラスは、`typ`、`schema`、`max_depth`、`allow_duplicate_keys` の設定を通じてパース動作を制御する、設定済みのパーサーインスタンスです。ラウンドトリップ（`rt`）、セーフ、フルの YAML パースモードをサポートします。

### 概要

```python
class YAML:
    """Configured YAML parser instance (rt / safe / full)."""
```

### コンストラクタ

#### `__init__()`

設定済みの YAML パーサーインスタンスを作成します。

```python
__init__(
    typ: str = "rt",
    schema: str = "core",
    max_depth: int = 1000,
    allow_duplicate_keys: bool = False,
) -> None
```

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `typ` | `str` | `"rt"` | パーサータイプ。`"rt"`（ラウンドトリップ）、`"safe"`、`"full"` のいずれか。 |
| `schema` | `str` | `"core"` | YAML スキーマ。`"core"`、`"yaml1.1"`、`"failsafe"`、`"json"` のいずれか。 |
| `max_depth` | `int` | `1000` | パース時の最大ネスト深さ。 |
| `allow_duplicate_keys` | `bool` | `False` | 重複するマッピングキーを許可するかどうか。 |

**Raises:** `YamlTypeError` — `typ` または `schema` が無効な場合。

**Example:**

```python
from pyrs_yaml import YAML

# Round-trip parser (default)
yaml = YAML()

# Safe parser (no merge resolution)
yaml_safe = YAML(typ="safe")

# Full parser with YAML 1.1 schema
yaml_full = YAML(typ="full", schema="yaml1.1")
```

### メソッド

#### `parse()`

YAML 文字列をパースし、完全なメタデータを保持した `YamlDocument` を返します。

```python
parse(yaml: str | bytes) -> YamlDocument
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `yaml` | `str \| bytes` | パースする YAML コンテンツ。 |

**Returns:** ラウンドトリップ編集、コメント保持、ソース追跡をサポートする `YamlDocument`。

**Notes:**

- マージ解決（`<<`）は `typ` が `"rt"` または `"full"` の場合に有効になります。
- 返されるドキュメントは、コメント、アンカー、フォーマットを保持します。

**Example:**

```python
yaml = YAML(typ="rt")
doc = yaml.parse("name: Alice\nage: 30\n")
print(doc.root_type())  # mapping
print(doc["name"])  # Alice
```

#### `safe_load()`

YAML をプレーンな Python の `dict` または `list` にパースし、アンカーとマージを解決します。

```python
safe_load(yaml: str) -> dict[str, Any] | list[Any]
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `yaml` | `str` | パースする YAML コンテンツ。 |

**Returns:** すべての YAML アンカーが解決されたプレーンな Python の `dict` または `list`。

**Notes:**

- このメソッドはコメント、フォーマット、ソース追跡を保持しません。
- すべてのアンカー参照が解決されます — 結果はプレーンな Python オブジェクトです。
- パースエラー時に `YamlTypeError` をスローします。

**Example:**

```python
yaml = YAML(typ="safe")
data = yaml.safe_load("""
person: &ref
  name: Alice
alias: *ref
""")
# data == {"person": {"name": "Alice"}, "alias": {"name": "Alice"}}
```

#### `safe_loads()`

複数ドキュメントの YAML 文字列をパースし、`dict`/`list` オブジェクトのリストを返します。

```python
safe_loads(yaml: str) -> list[dict[str, Any] | list[Any]]
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `yaml` | `str` | 複数ドキュメントの YAML コンテンツ。 |

**Returns:** ドキュメントごとに 1 つの、プレーンな Python の `dict` または `list` オブジェクトのリスト。

**Notes:**

- ドキュメントは `---` マーカーで区切られます。
- アンカーとマージは各ドキュメント内で解決されます。
- コメントとフォーマットは保持されません。

**Example:**

```python
yaml = YAML(typ="safe")
docs = yaml.safe_loads("""
---
a: 1
---
b: 2
""")
# docs == [{"a": 1}, {"b": 2}]
```

#### `parse_file()`

YAML ファイルをパースし、完全なメタデータを保持した `YamlDocument` を返します。

```python
parse_file(path: str) -> YamlDocument
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `path` | `str` | 読み取りおよびパースするファイルパス。 |

**Returns:** ラウンドトリップ編集をサポートする `YamlDocument`。

**Raises:** `IOError` — ファイルが読み取れない場合。

**Notes:**

- ファイルは Rust の `std::fs::read_to_string` を使用してディスクから読み取られます — GIL ブロッキングは発生しません。
- ソースはラウンドトリップの忠実性のためにドキュメントに保存されます。

**Example:**

```python
yaml = YAML(typ="rt")
doc = yaml.parse_file("config.yaml")
print(doc["database"]["host"])
```

#### `parse_all_docs()`

複数ドキュメントの YAML 文字列をパースし、`YamlDocument` オブジェクトのリストを返します。

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `yaml` | `str` | 複数ドキュメントの YAML コンテンツ。 |

**Returns:** ドキュメントごとに 1 つの `YamlDocument` オブジェクトのリスト。

**Notes:**

- ドキュメントは `---` マーカーで区切られます。
- 各ドキュメントは完全なラウンドトリップサポート（コメント、アンカー、フォーマット）を保持します。
- マージ解決は `typ` が `"rt"` または `"full"` の場合に有効になります。

**Example:**

```python
yaml = YAML(typ="rt")
docs = yaml.parse_all_docs("""
---
a: 1
---
b: 2
""")
for doc in docs:
    print(doc.root_type())
```

#### `dump_stream()` / `dump_file()`

ストリーミングライター: Python オブジェクトをファイルライクオブジェクトまたはディスク上のファイルにシリアライズし、一定メモリを使用します。

=== "メモリ内"

    ```python
    dump_stream(
        file_obj: Any,
        iterable: Any,
        explicit_start: bool = False,
        explicit_end: bool = False,
        sort_keys: bool = False,
    ) -> None
    ```

    **Parameters:**

    | Parameter | Type | Default | Description |
    |-----------|------|---------|-------------|
    | `file_obj` | `Any` | — | `write(str)` メソッドを持つ書き込み可能なファイルライクオブジェクト。 |
    | `iterable` | `Any` | — | シリアライズする Python オブジェクトの反復可能オブジェクト。 |
    | `explicit_start` | `bool` | `False` | 各ドキュメントの先頭に `---` を出力するかどうか。 |
    | `explicit_end` | `bool` | `False` | 各ドキュメントの末尾に `...` を出力するかどうか。 |
    | `sort_keys` | `bool` | `False` | マッピングキーをアルファベット順にソートするかどうか。 |

    **Raises:** `YamlTypeError` — `file_obj` に `write` メソッドがない場合。

    **Notes:**

    - 一定メモリを使用 — 出力全体をメモリに保持する必要はありません。
    - Rust のシリアライズフェーズ中は GIL が解放されます。
    - 反復可能オブジェクトの各アイテムが個別の YAML ドキュメントになります。

    **Example:**

    ```python
    import io
    from pyrs_yaml import YAML

    yaml = YAML()
    buf = io.StringIO()
    yaml.dump_stream(buf, [{"a": 1}, {"b": 2}], explicit_start=True)
    print(buf.getvalue())
    # ---
    # a: 1
    # ---
    # b: 2
    ```

=== "ファイルパス"

    ```python
    dump_file(
        path: str,
        iterable: Any,
        explicit_start: bool = False,
        explicit_end: bool = False,
        sort_keys: bool = False,
    ) -> None
    ```

    **Parameters:**

    | Parameter | Type | Default | Description |
    |-----------|------|---------|-------------|
    | `path` | `str` | — | 書き込み先のファイルパス。 |
    | `iterable` | `Any` | — | シリアライズする Python オブジェクトの反復可能オブジェクト。 |
    | `explicit_start` | `bool` | `False` | 各ドキュメントの先頭に `---` を出力するかどうか。 |
    | `explicit_end` | `bool` | `False` | 各ドキュメントの末尾に `...` を出力するかどうか。 |
    | `sort_keys` | `bool` | `False` | マッピングキーをアルファベット順にソートするかどうか。 |

    **Raises:** `IOError` — ファイルを作成または書き込みできない場合。

    **Notes:**

    - Rust の `std::fs::File` を直接使用 — I/O 中の GIL ブロッキングは発生しません。
    - 反復可能オブジェクトの各アイテムが個別の YAML ドキュメントになります。
    - 一定メモリを使用し、大規模な出力に適しています。

    **Example:**

    ```python
    from pyrs_yaml import YAML

    yaml = YAML()
    yaml.dump_file("output.yaml", [{"x": 2}, {"x": 3}], sort_keys=True)
    ```

#### `load_stream()` / `load_stream_file()`

遅延イベントイテレーター: ファイルライクオブジェクトまたはファイルパスからインクリメンタルに読み取ります。

=== "メモリ内"

    ```python
    load_stream(file_obj: Any) -> YamlStream
    ```

    **Parameters:**

    | Parameter | Type | Description |
    |-----------|------|-------------|
    | `file_obj` | `Any` | `str` または `bytes` を返す `read()` メソッドを持つ読み取り可能なファイルライクオブジェクト。 |

    **Returns:** パースされたイベント dict を遅延生成する `YamlStream` イテレーター。

    **Raises:** `YamlTypeError` — `file_obj` に `read` メソッドがない場合。

    **Notes:**

    - ストリームはインクリメンタルにパースされます — ファイル全体をメモリに読み込む必要はありません。
    - 生成される各イベントは、`"type"`、`"key"`、`"value"`、`"start_mark"`、`"end_mark"` などのキーを持つ `dict` です。
    - `__next__` が `None` を返すとストリームは終了します。

    **Example:**

    ```python
    import io
    from pyrs_yaml import YAML

    yaml = YAML()
    buf = io.StringIO("key: value\n")
    stream = yaml.load_stream(buf)
    for event in stream:
        if event is None:
            break
        print(event["type"])
    ```

=== "ファイルパス"

    ```python
    load_stream_file(path: str) -> YamlStream
    ```

    **Parameters:**

    | Parameter | Type | Description |
    |-----------|------|-------------|
    | `path` | `str` | インクリメンタルに読み取るファイルパス。 |

    **Returns:** パースされたイベント dict を遅延生成する `YamlStream` イテレーター。

    **Raises:** `IOError` — ファイルを開けない場合。

    **Notes:**

    - Rust の `std::fs::File` とバッファリング I/O を使用 — 読み取り中の GIL ブロッキングは発生しません。
    - ファイルをインクリメンタルにパースするため、大規模な YAML ファイルに最適です。

    **Example:**

    ```python
    from pyrs_yaml import YAML

    yaml = YAML()
    stream = yaml.load_stream_file("large.yaml")
    for event in stream:
        if event is None:
            break
        print(event)
    ```

### 使用例

#### 設定インスタンスを使ったラウンドトリップ編集

```python
from pyrs_yaml import YAML

yaml = YAML(typ="rt", schema="core")
doc = yaml.parse("""
# User configuration
user:
  name: Alice
  age: 30
  tags: [admin, user]
""")

# Edit the document
doc["user"]["age"] = 31
doc["user"]["tags"].append("staff")

# Serialize back — comments and formatting are preserved
print(doc.to_yaml())
```

#### JSON スキーマを使ったセーフパース

```python
from pyrs_yaml import YAML

yaml = YAML(typ="safe", schema="json")
data = yaml.safe_load("{name: Bob, age: 25}")
print(data["name"])  # Bob
```

#### 複数ドキュメントストリームの処理

```python
from pyrs_yaml import YAML

yaml = YAML(typ="rt")
docs = yaml.parse_all_docs("""
---
doc: first
---
doc: second
""")
for doc in docs:
    print(doc["doc"])

# Or dump multiple documents
yaml.dump_file("multi.yaml", [{"id": 1}, {"id": 2}], explicit_start=True)
```

### 関連項目

- [`YamlDocument`](yaml-document.md) — ラウンドトリップ編集可能なドキュメントオブジェクト
- [`YamlStream`](reference.md#yamlstream) — 遅延イベントストリームイテレーター
- [`parse()`](reference.md#parse) — モジュールレベルの便利関数
- [`safe_load()`](reference.md#safe_load) — モジュールレベルの便利関数
