---
title: プラグイン開発
description: Community Plugins API を使用して pyrs-yaml のサードパーティプラグインを作成します。
tags:
  - docs
status: new
---

## プラグイン開発

このガイドでは、Community Plugins API を使用して pyrs-yaml のサードパーティプラグインを作成する方法を説明します。

### プラグインの構造

プラグインは `CustomType` サブクラスを定義し、それを登録する Python モジュールです。

```python title="my_timestamp_plugin.py"
import pyrs_yaml
from datetime import datetime


class MyTimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj):
        return obj.isoformat()


def register():
    pyrs_yaml.register_type("!mytimestamp", MyTimestampType())
```

### 統合 TagHandler API

`TagHandler` はタグレジストリ（`register_tag`）とコミュニティプラグイン（`register_type`）を単一の登録ポイントに統合します。ハンドラはスカラー、コンテナ、ライフサイクルメソッドを任意に組み合わせられます。

```python
class HexInt(pyrs_yaml.TagHandler):
    python_type = int

    def from_yaml(self, value, tag):
        return int(value, 16)

    def to_yaml(self, obj, tag):
        return hex(obj)


pyrs_yaml.register_tag_handler("!hex", HexInt())
```

コンテナタグ（`from_yaml_mapping`/`from_yaml_sequence`/`to_yaml_mapping`/`to_yaml_sequence`）は AST ラウンドトリップパスでのみ有効。`safe_dump` 高速パスは常にプレーンコンテナを出力します。

### API リファレンス

| 関数 | 説明 |
|---------|------|
| :material-code-braces: `register_type(name, handler)` | `CustomType` インスタンスを登録 |
| :material-close: `clear_type_handlers()` | すべての登録タイプを削除 |
| :material-close: `remove_type(name)` | 特定のタイプを削除 |
| :material-check-decagram: `validate_custom_types(obj)` | オブジェクトを全登録タイプに対して検証 |
| :material-link-variant: `register_tag_handler(name, handler)` | 統合 `TagHandler` を登録 |
| :material-close: `clear_tag_handlers()` | すべてのタグハンドラを削除 |
| :material-close: `remove_tag_handler(name)` | 指定タグのハンドラを削除 |
| :material-code-braces: `list_tag_handlers()` | 登録済みタグハンドラを一覧 |
| :material-code-braces: `get_tag_handler(name)` | タグのハンドラインスタンスを取得 |
| :material-export: `serialize(doc, ...)` | ライフサイクルフック付きシリアライズ |

### CustomType から TagHandler への移行

`CustomType`、`register_type`、`register_tag` は**非推奨**となりましたが、引き続き利用可能です（`DeprecationWarning` が出ます）。`register_type("!x", handler)` は `CustomType` を `TagHandler` に自動ラップするため、既存の登録コードは変更不要です。

**API 対照：**

| 旧 API | 新 API |
|---------|--------|
| `register_type("!x", T())` | `register_tag_handler("!x", THandler())` |
| `from_yaml(value)` | `from_yaml(value, tag)` |
| `to_yaml(obj)` | `to_yaml(obj, tag)` |
| `can_parse` / `validate` / `python_type` | 変更なし |
| — | `resolve` / `from_yaml_mapping` / `to_yaml_mapping` / ライフサイクルフック（TagHandler 新規） |

!!! note "コンテナ型"
    タグが Mapping/Sequence（例: `!set`）に付く場合は、スカラーの `from_yaml` ではなく `TagHandler` に `from_yaml_mapping` / `to_yaml_mapping` を実装します。

---

### 関連項目

- [コミュニティプラグイン](community-plugins.md) — 拡張できる組み込みタイプ
- [カスタムスキーマ](custom-schema.md) — 型解決ルールを定義
- [タグレジストリ API](../api/reference.md#tag-registry) — `register_tag()` と関連関数
