# ストリーム解析

`YAML.load_stream(file_obj)` および `YAML.load_stream_file(path)` は YAML イベントを遅延反復します——メモリ使用量は O(アンカー数 + 64KB チャンク) で、入力サイズに依存しません。100MB+ のファイルに適しています。

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

### エラーセマンティクス

途中で失敗（イテレータ例外、シリアライズエラー、書き込み失敗）した場合、既に書き込まれた出力はターゲットに残ります——部分出力のロールバックは行われません。

### safe_dump との違い

| 側面 | dump_stream / dump_file | safe_dump |
|------|------------------------|-----------|
| 出力 | マルチドキュメントストリーム | 単一ドキュメント |
| メモリ | O(単一ドキュメント + 64KB) | O(入力) |
| 項目タイプ | `YamlDocument`（コメント/アンカー保持）または通常の Python オブジェクト | 単一の Python オブジェクト |

### キーソート

`sort_keys=True` を渡すと、マッピングキーをソート順で出力します。`safe_dump` の `sort_keys` と同じ動作です。
