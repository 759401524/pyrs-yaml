---
title: コミュニティプラグイン
description: Community Plugins API で pyrs-yaml を拡張し、カスタムノード型を登録します。
tags:
  - docs
status: new
---

## コミュニティプラグイン

Community Plugins API を使うと、カスタム YAML ノード型を定義して、pyrs-yaml のシリアル化と
デシリアル化に統合できます。カスタム型は、YAML タグ付きスカラと任意の Python オブジェクトを
相互変換できます。

### 組み込みプラグイン

pyrs-yaml にはインポート時に自動登録される組み込みプラグインが付属しています：

| タグ | Python 型 | 説明 |
|-----|----------|------|
| :material-clock-outline: `!timestamp` | `datetime` | ISO 8601 日時のラウンドトリップ |
| :material-calendar: `!date` | `datetime.date` | ISO 8601 日付（時刻なし） |
| :material-clock: `!time` | `datetime.time` | ISO 8601 時刻（日付なし） |
| :material-binary: `!uuid` | `uuid.UUID` | UUID 文字列 ↔ オブジェクト |
| :material-decimal: `!decimal` | `decimal.Decimal` | 任意精度の小数 |
| :material-binary: `!binary` | `bytes` | Base64 エンコードされたバイナリデータ |
| :material-language-python: `!regex` | `re.Pattern` | コンパイル済み正規表現 |
| :material-format-list-bulleted: `!set` | `str` | YAML セット（キー無しマッピング） |

### カスタム型の作成

`CustomType` を継承し、`from_yaml()` と `to_yaml()` を実装します：

```python title="CustomType サブクラス"
import pyrs_yaml
from datetime import datetime


class TimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value: str):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj) -> str:
        return obj.isoformat()
```

### 登録

**命令形式：**

```python title="命令形式の登録"
pyrs_yaml.register_type("!timestamp", TimestampType())
```

**デコレータ形式：**

```python title="デコレータ形式の登録"
@pyrs_yaml.register_type("!timestamp")
class TimestampType(pyrs_yaml.CustomType):
    ...
```

### 使用

**タグ付きスカラの読み込み：**

```python title="タグ付きスカラの解析"
doc = pyrs_yaml.parse("when: !timestamp 2026-08-11T10:30:00")
val = doc.get("when")
assert isinstance(val, datetime)
```

**Python オブジェクトのダンプ：**

```python title="Python オブジェクトのダンプ"
data = {"ts": datetime(2026, 8, 11, 10, 30)}
out = pyrs_yaml.safe_dump(data)
# out に含まれる: ts: !timestamp 2026-08-11T10:30:00
```

### API リファレンス

| メソッド | 説明 |
|---------|------|
| :material-function: `can_parse(node)` | この型が AST ノードを処理するかどうか |
| :material-swap-horizontal: `from_yaml(value)` | YAML 文字列を Python オブジェクトに変換 |
| :material-swap-horizontal: `to_yaml(obj)` | Python オブジェクトを YAML 文字列に変換 |
| :material-check-decagram: `validate(obj)` | Python オブジェクトを検証（`bool` を返す） |

### 例：UUID 型

```python title="uuid_plugin.py"
import uuid
import pyrs_yaml


class UUIDType(pyrs_yaml.CustomType):
    python_type = uuid.UUID

    def from_yaml(self, value):
        return uuid.UUID(value)

    def to_yaml(self, obj):
        return str(obj)


pyrs_yaml.register_type("!uuid", UUIDType())

doc = pyrs_yaml.parse("id: !uuid 550e8400-e29b-41d4-a716-446655440000")
assert isinstance(doc.get("id"), uuid.UUID)
```

---

### 関連項目

- [プラグイン開発](plugin-development.md) — 独自のカスタムタイプを構築
- [カスタムスキーマ](custom-schema.md) — スカラー型解決を制御
