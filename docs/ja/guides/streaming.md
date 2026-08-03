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
