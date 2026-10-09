---
title: カスタムスキーマ
description: 型解決の制御のためのカスタム YAML スキーマの定義と使用。
tags:
  - docs
status: new
---

## カスタムスキーマ

デフォルトでは、pyrs-yaml は YAML 1.2 Core スキーマで暗黙の型解決を行います。YAML Schema Language を使えば、
プレーンなスカラをどの Python 型に解決するかを制御するカスタムスキーマを定義できます。

### なぜカスタムスキーマが必要か？

Core スキーマは `0xFF` を `int(255)`、`2026-08-11` を `int(2026)`、
`hello` を `"hello"` に解決します。場合によっては異なる挙動が欲しいことがあります：

- 日付を文字列のままに保つ（`2026` ではなく `"2026-08-11"`）
- 16進数/2進数のリテラルを整数として解釈
- YAML 1.1 風のブール語彙（`yes`/`no`）を追加
- JSON 専用のサブセット（`inf`、`nan`、`0x` なし）

### スキーマ定義形式

スキーマは `rules` リストを持つ YAML ファイルとして定義します。各ルールは `pattern`（正規表現）と
`type`（`null`、`bool`、`int`、`float`、`str` のいずれか）を持ちます。

```yaml
# hex_schema.yaml
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
  - pattern: ^0b[01]+$
    type: int
```

**`extends`** — オプションの基本スキーマ。ルールを優先して一致させ、一致しない場合は
`extends` で指定したスキーマにフォールバックします。デフォルト：`core`。

**`rules`** — 順序付きリスト。最初に一致したパターンが型を決定します。サポートされる型：

| `type` | Python 結果 | 例 |
|--------|------------|----|
| :material-null: `null` | `None` | `~` |
| :material-toggle-switch: `bool` | `True` / `False` | `true`, `yes`, `on` |
| :material-numeric: `int` | `int` | `42`, `0xFF`, `0o77`, `0b1010` |
| :material-decimal: `float` | `float` | `3.14`, `1e10` |
| :material-format-text: `str` | `str` | `2026-08-11` |

### スキーマの登録と使用

```python title="スキーマを登録して使用"
import pyrs_yaml

# YAML 文字列から登録
pyrs_yaml.register_schema(
    "hex",
    """
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
""",
)

# YAML インスタンスで使用
y = pyrs_yaml.YAML(schema="hex")
doc = y.parse("addr: 0xFF")
assert doc.get("addr") == 255

# モジュール関数で使用
d = pyrs_yaml.safe_load("addr: 0x1F", schema="hex")
assert d["addr"] == 31
```

#### ファイルからスキーマを読み込む

`load_schema()` はファイルパスからスキーマ定義を読み込んで登録します：

```python title="ファイルからスキーマを読み込む"
# hex.yaml に上記のスキーマ YAML が含まれている
pyrs_yaml.load_schema("hex", "path/to/hex.yaml")
```

#### 登録済みスキーマの一覧

`list_schemas()` は登録済みのすべてのスキーマ名（組み込み + カスタム）を返します：

```python title="登録済みスキーマの一覧"
print(pyrs_yaml.list_schemas())
# ['failsafe', 'json', 'core', 'yaml1.1', 'hex', ...]
```

#### 構造検証

スキーマ定義に `validate` セクションを追加すると、スカラー型解決に加えて構造チェックができます。`validate_against_schema()` で文書の使用前にチェックします：

```yaml title="検証付きスキーマ"
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
  - path: $.tags[*]
    type: str
  - path: $.numbers
    sequence_of: int
  - path: $.config
    mapping_of: str
  - path: $.extra
    type: map
```

```python
import pyrs_yaml

schema = """\
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
"""

pyrs_yaml.validate_against_schema("port: 80\n", schema)  # OK
# すべての失敗を列挙して YamlValidateError を送出:
pyrs_yaml.validate_against_schema("port: abc\n", schema)
```

- `path` — JSONPath 風の場所（`$.key`、`$.a.b`、`$.tags[*]`）。省略時はすべてのスカラーを指します。
  `[*]` は一つの要素だけを指すため、`$.rows[*]` は `$.rows[0].a` に届きません
- `type` — スカラーがこの YAML 型（`null`/`bool`/`int`/`float`/`str`）に解決されること。または
  ノードがこの容器の形（`map`/`seq`。`mapping`/`object`、`sequence`/`array`/`list` も受け付けます）を
  持つこと。容器の型には `path` が必要です
- `sequence_of` / `mapping_of` — ノードがその容器であり、すべての要素 / 値が指定型であること。
  指定型には `map` や `seq` も使えます
- `required` — パスが存在し非 null であること（`type` と組み合わせ可能）

規則が主張する量はスコープが決めます。`path` を書いた規則はそのノードに関するものなので、形が
違えば失敗になります。`$.port` がマップなら `type: int` に反し、`$.config` がスカラーなら
`mapping_of: str` に反します。`path` を省略した規則はノードを指定できないため、説明できるノードを
形で選ぶ側に回ります。パスなしの `type: str` は「すべてのスカラーは文字列」という意味で、文書の
残りは問いません。パスなしの `type: map` は何も選ばないので、書かれた時点で拒否されます。要素は
どちらの場合も検査されます。`sequence_of: int` は要素が何かをすでに述べており、入れ子の配列は
int ではないからです。一つの規則が持てる検査は一つだけで、`required` は検査と併用できます。別名の
ノード（`b: *x`）についての規則は判定しません。別名は形ではなく名前を保持するからです。

### インラインディクスキーマ

個別に登録せず、ディクショナリを直接渡せます：

```python title="インラインディクスキーマ"
d = pyrs_yaml.safe_load(
    "addr: 0xFF",
    schema={
        "extends": "core",
        "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}],
    },
)
assert d["addr"] == 255
```

### 一般的なパターン

=== "日付を文字列として保持"

    ```python
    schema = {
        "extends": "core",
        "rules": [{"pattern": "^\\d{4}-\\d{2}-\\d{2}$", "type": "str"}],
    }
    ```

=== "YAML 1.1 ブール値の追加"

    ```python
    schema = {
        "extends": "core",
        "rules": [{"pattern": "^(yes|no|Yes|No|YES|NO)$", "type": "bool"}],
    }
    ```

=== "厳格な JSON モード"

    ```python
    schema = {
        "extends": "failsafe",
        "rules": [
            {"pattern": "^null$|^~$", "type": "null"},
            {"pattern": "^(true|false)$", "type": "bool"},
            {"pattern": "^-?\\d+$", "type": "int"},
            {"pattern": "^-?\\d+\\.\\d+$", "type": "float"},
        ],
    }
    ```

### パフォーマンス

カスタムスキーマは正規表現ベースのルールエンジンを使用します。各スカラは順にルールを照合します。
最適なパフォーマンスのため：

- ルール数は 20 以下に保つ
- よく使うパターンを先頭に置く
- `extends: core` を使って完全な Core 解決を再実装しない

組み込みの Core スキーマは影響を受けません。ゼロコストの `match` ディスパッチのままで、
カスタムスキーマの登録の影響を受けません。

---

### 関連項目

- [プラグイン開発](plugin-development.md) — カスタムタグハンドラの構築
- [i18n エラーメッセージ](i18n.md) — スキーマのエラーメッセージをローカライズ
- [スキーマ API リファレンス](../api/reference.md#yaml-schema-language) — `register_schema()` とインラインスキーマのドキュメント
