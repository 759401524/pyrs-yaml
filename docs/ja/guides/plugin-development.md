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

### API リファレンス

| 関数 | 説明 |
|---------|------|
| :material-code-braces: `register_type(name, handler)` | `CustomType` インスタンスを登録 |
| :material-close: `clear_type_handlers()` | すべての登録タイプを削除 |
| :material-close: `remove_type(name)` | 特定のタイプを削除 |
| :material-check-decagram: `validate_custom_types(obj)` | オブジェクトを全登録タイプに対して検証 |

---

### 関連項目

- [コミュニティプラグイン](community-plugins.md) — 拡張できる組み込みタイプ
- [カスタムスキーマ](custom-schema.md) — 型解決ルールを定義
- [タグレジストリ API](../api/reference.md#tag-registry) — `register_tag()` と関連関数
