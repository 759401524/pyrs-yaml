---
title: TOML・JSON・INI 形式
description: pyrs-yaml で TOML・JSON・INI をやり取りし、編集可能な表現は YAML のままにするガイド。
tags:
  - docs
status: new
---

TOML・JSON・INI の読み書き — ハブは YAML。

## ハブ・スポークモデル

pyrs-yaml は複数の設定形式に対応しますが、**編集可能なのは YAML のみ**です:
ラウンドトリップ編集（コメント・アンカー・インプレースsplice）は YAML 専用で、
交換形式は入出力の変換を担います。

```text
load_toml / load_ini / JSON テキスト ──▶ 値と YAML ──▶ parse() / edit() / dump
to_toml  ◀── YAML テキスト ◀──────────────────┘
```

## JSON: もともとネイティブ

YAML 1.2 は JSON の上位互集合なので、すべての JSON ドキュメントは YAML を
受け付ける場所ならどこでもそのまま読み込めます。専用 API は不要です:

```python title="JSON 入力"
import pyrs_yaml

data = pyrs_yaml.safe_load('{"a": [1, 2], "b": true}')
# {'a': [1, 2], 'b': True}
```

専用の `json` スキーマ（[カスタムスキーマ](custom-schema.md) 参照）は
スカラー解決を JSON 互換の規則に限定し、`from_json` と CLI の `to-json` で
明示的変換ができます。

### 行区切りの JSON: `load_json_lines` { #json-lines }

`.jsonl` / NDJSON ファイルは 1 つの JSON ドキュメントではないため、ドキュメントを受け取る
どの API もそれを読めません — 2 レコードのファイルに対する `load_json` は 2 行目を末尾の
余分な文字として拒否します。読める経路が `load_json_lines` で、`JsonLines` イテレータを
返して `__next__` ごとに 1 ドキュメントだけ解決するので、同時にメモリにある行は 1 行だけです:

```python title="JSON Lines"
import pyrs_yaml

records = pyrs_yaml.load_json_lines('{"a": 1}\n{"b": 2}\n')
next(records)  # {'a': 1}
list(pyrs_yaml.load_json_lines('{"a": 1}\n{"b": 2}\n'))
# [{'a': 1}, {'b': 2}]
```

各値は `load_json` がその行に対して返すものそのままです。そしてストリームはエンジンの
契約を保ちます — フォーマットが定義していない行は `YamlParseError` を投げ、入力全体に
対するその行の位置を名乗り、カーソルは動きません。つまり再び尋ねれば、次のレコードでは
なく同じレコードが報告されます。

```python title="拒否された行が伝える内容"
list(pyrs_yaml.load_json_lines('{"a": 1}\n\n'))
# pyrs_yaml.YamlParseError: a blank line carries no document in
# a line-delimited JSON stream at line 2 column 1

next(pyrs_yaml.load_json_lines("// note\n"))
# pyrs_yaml.YamlParseError: expected a JSON value at line 1 column 1

next(pyrs_yaml.load_json_lines('{"a": 1,\n'))
# pyrs_yaml.YamlParseError: expected a quoted object key at line 1 column 9
```

空行は、どこにある場合でも拒否されます — ファイルの末尾に余分に付く 1 行も含みます。
`{"a": 1}\n\n` に `[]` で答えるストリームは、作者がどの行を落としたかったのかを推測する
ことになるからです。1 行に 1 つの完全な値があることが必要であり、それで足ります —
区切りなし、末尾カンマなし、1 行 1 ドキュメント。

## TOML

JSON 変換ファミリーと同型の 3 関数:

```python title="TOML の入出力"
import pyrs_yaml

# TOML テキスト -> Python 値（中間ドキュメントなしの高速パス）
config = pyrs_yaml.load_toml('s = "true"\nn = 42\n')
# {'s': 'true', 'n': 42}   <- TOML 文字列は再解決されない: "true" は文字列のまま

# TOML テキスト -> YAML テキスト（以降は通常の編集が可能）
yaml_text = pyrs_yaml.from_toml('title = "app"\nport = 8080\n')
# 'title: "app"\nport: 8080\n'

# YAML テキスト -> TOML テキスト
toml_text = pyrs_yaml.to_toml("name: app\ncount: 3\nnested:\n  a: 1\n  b: two\n")
# 'name = "app"\ncount = 3\nnested = { a = 1, b = "two" }\n'
```

!!! note "日付と時刻"

    TOML の datetime は内蔵 `!!timestamp` プラグイン経由で本物の
    `datetime.datetime` として届きます:

    ```python
    pyrs_yaml.load_toml("when = 2026-01-02T03:04:05Z\n")
    # {'when': datetime.datetime(2026, 1, 2, 3, 4, 5, tzinfo=datetime.timezone.utc)}
    ```

### `to_toml` が拒否するもの

TOML で表現できない YAML 構造は、黙ってデータを落とす代わりに安定した
メッセージの `ValueError` になります:

- null 値（TOML に null は存在しない）
- 非テーブルのドキュメント（ルートがスカラーや配列）
- アンカー / エイリアス、および `!!timestamp` 以外のタグ
- 非スカラーのマップキー

## INI

意図的に読み取り専用 — INI には正式な文法がないため、pyrs-yaml は標準
ライブラリのパーサで受け取り、書き出しは各ツールの裁量に委ねます:

```python title="INI 入力"
import pyrs_yaml

config = pyrs_yaml.load_ini("[server]\nHost = 127.0.0.1\nPort = 8080\n")
# {'server': {'Host': '127.0.0.1', 'Port': '8080'}}
```

挙動のメモ:

- キーの大文字小文字は保持されます（`host` ではなく `Host`）;
- セクション、`;`/`#` コメント、複数行値は厳格モードの
  `configparser.RawConfigParser` の意味論に従います — 重複キーや
  セクションヘッダー欠落は `ValueError` を送出;
- すべての値は文字列です。型が必要なら自前で変換するか、YAML 経由で:

```python title="INI から YAML へ"
import pyrs_yaml

yaml_text = pyrs_yaml.safe_dump(pyrs_yaml.load_ini("[s]\nport = 8080\n"))
# s:\n  port: '8080'\n
```

## 関連

- [カスタムスキーマ](custom-schema.md) — `json` スカラー解決スキーマ
- [コマンドラインインターフェース](cli.md) — `to-json` / `from-json`
- [ラウンドトリップ保持](round-trip.md) — YAML が編集ハブである理由
