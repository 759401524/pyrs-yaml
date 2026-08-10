---
title: ストリーム解析
description: pyrs-yaml のストリーム解析機能を説明します。load_stream、parse_stream、StreamIterator、リソース管理をカバーします。
tags:
  - docs
status: new
---

!!! note "ストリーミング解析"
    ストリーム解析はメモリ使用量を O(入力サイズ) ではなく O(アンカー数 + 64KB チャンク) に抑えるため、100MB を超える大きなファイルに適しています。

`YAML.load_stream(file_obj)` および `YAML.load_stream_file(path)` は YAML イベントを遅延反復し、入力サイズに依存しないメモリ使用量を実現します。

```python
from pyrs_yaml import YAML

for event in YAML().load_stream_file("huge.yaml"):
    print(event["type"], event["value"])
```

## parse_stream との違い

| 動作 | load_stream | parse_stream |
| --- | --- | --- |
| メモリ | O(アンカー + チャンク) | O(入力) |
| コメント | 出力しない | 出力する |
| アンカー名 | `anchor_{id}` | 元の名前 |
| エラーメッセージ | ソーススニペットなし | ソーススニペットあり |
| 空入力 | `[stream_start, stream_end]` | `[]` |
| タグハンドラ | 適用しない | 適用する (YAML.parse) |

## リソース管理

早期に停止する場合は `close()` を呼び出してください——これが唯一の解放保証ポイントです（PyPy の遅延 GC は `Drop` のタイミングを保証しません）。`close()` は冪等であり、渡したファイルオブジェクトは**閉じません**。

## ストリーム書き込み

`YAML().dump_stream(file_obj, iterable, ...)` および `YAML().dump_file(path, iterable, ...)`
はドキュメントを一つずつシリアライズし、定数メモリ（O(単一ドキュメント + 64KB チャンク)）を使用します。

```python
from pyrs_yaml import YAML

buf = io.StringIO()
YAML().dump_stream(buf, [{"a": 1}, {"b": 2}])
# buf.getvalue() == "a: 1\n---\nb: 2\n"
```

### セパレータルール

- 最初のドキュメントの前には `---` なし。以降の各ドキュメントの前に `---` を追加。
- `explicit_start=True` で最初のドキュメントの前に `---` を追加。
- `explicit_end=True` で最後のドキュメントの後に `...` を追加。
- 空の iterable は 0 バイトを出力。

#### エラーセマンティクス

途中で失敗（イテレータ例外、シリアライズエラー、書き込み失敗）した場合、既に書き込まれた出力はターゲットに残ります——部分出力のロールバックは行われません。

#### safe_dump との違い

| 側面 | dump_stream / dump_file | safe_dump |
|------|------------------------|-----------|
| 出力 | マルチドキュメントストリーム | 単一ドキュメント |
| メモリ | O(単一ドキュメント + 64KB) | O(入力) |
| 項目タイプ | `YamlDocument`（コメント/アンカー保持）または通常の Python オブジェクト | 単一の Python オブジェクト |

#### キーソート

`sort_keys=True` を渡すと、マッピングキーをソート順で出力します。`safe_dump` の `sort_keys` と同じ動作です。

## StreamIterator

`StreamIterator` クラスは `parse_stream()` および `YAML().load_stream()` / `YAML().load_stream_file()` によって生成されます。イテレータプロトコルを実装し、イベント dict を一度に 1 つずつ生成します。

```python
from pyrs_yaml import parse_stream

iterator = parse_stream("key: value\n---\na: 1")
for event in iterator:
    print(event["type"], event["value"])
```

### イテレータプロトコル

`StreamIterator` は `__iter__`（`self` を返す）と `__next__` を実装します：

```python
def __iter__() -> StreamIterator: ...
def __next__() -> dict | None: ...
```

ストリームを使い切ると `__next__()` は `None` を返します（`StopIteration` は発生しません）。

#### イベント dict のキー

| キー | 型 | 説明 |
| --- | --- | --- |
| `type` | `str` | イベントタイプ（下記参照） |
| `value` | `str` または `None` | スカラー値、エイリアス名、またはコメントテキスト |
| `style` | `str` または `None` | スカラーのクォートスタイル：`"plain"`、`"single_quoted"`、`"double_quoted"`、`"literal"`、`"folded"`；コメントの場合は `"standalone"` または `"inline"` |
| `anchor` | `str` または `None` | アンカー名（`&name`） |
| `tag` | `str` または `None` | タグ文字列（`!!str`、`!custom`） |
| `line` | `int` | 行番号（0 始まり） |
| `column` | `int` | 列番号（0 始まり） |

#### イベントタイプ

| `type` | 生成される場面 |
| --- | --- |
| `stream_start` | YAML ストリームの開始 |
| `stream_end` | ストリームの終了 |
| `document_start` | ドキュメントの開始 |
| `document_end` | ドキュメントの終了 |
| `mapping_start` | マッピングの開始 |
| `mapping_end` | マッピングの終了 |
| `sequence_start` | シーケンスの開始 |
| `sequence_end` | シーケンスの終了 |
| `scalar` | スカラー値 |
| `alias` | エイリアス参照（`*name`） |
| `comment` | YAML コメント |

#### `load_stream` との違い

`parse_stream()` はコメントを生成し、元のアンカー名を保持する `StreamIterator` を返します。`YAML().load_stream()` / `YAML().load_stream_file()` はデフォルトが異なる `YamlStream` を返します（上記の比較表を参照）。
