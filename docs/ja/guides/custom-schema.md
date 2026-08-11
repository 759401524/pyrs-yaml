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
| `null` | `None` | `~` |
| `bool` | `True` / `False` | `true`, `yes`, `on` |
| `int` | `int` | `42`, `0xFF`, `0o77`, `0b1010` |
| `float` | `float` | `3.14`, `1e10` |
| `str` | `str` | `2026-08-11` |

### スキーマの登録と使用

```python
import pyrs_yaml

# YAML 文字列から登録
pyrs_yaml.register_schema("hex", """
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
""")

# YAML インスタンスで使用
y = pyrs_yaml.YAML(schema="hex")
doc = y.parse("addr: 0xFF")
assert doc.get("addr") == 255

# モジュール関数で使用
d = pyrs_yaml.safe_load("addr: 0x1F", schema="hex")
assert d["addr"] == 31
```

### インラインディクスキーマ

個別に登録せず、ディクショナリを直接渡せます：

```python
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

#### 日付を文字列として保持

```python
schema = {
    "extends": "core",
    "rules": [{"pattern": "^\\d{4}-\\d{2}-\\d{2}$", "type": "str"}],
}
```

#### YAML 1.1 ブール値の追加

```python
schema = {
    "extends": "core",
    "rules": [{"pattern": "^(yes|no|Yes|No|YES|NO)$", "type": "bool"}],
}
```

#### 厳格な JSON モード

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
