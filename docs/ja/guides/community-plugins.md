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
| `!timestamp` | `datetime` | ISO 8601 日時のラウンドトリップ |
| `!date` | `datetime.date` | ISO 8601 日付（時刻なし） |
| `!time` | `datetime.time` | ISO 8601 時刻（日付なし） |
| `!uuid` | `uuid.UUID` | UUID 文字列 ↔ オブジェクト |
| `!decimal` | `decimal.Decimal` | 任意精度の小数 |
| `!binary` | `bytes` | Base64 エンコードされたバイナリデータ |
| `!regex` | `re.Pattern` | コンパイル済み正規表現 |
| `!set` | `str` | YAML セット（キー無しマッピング） |

### カスタム型の作成

`CustomType` を継承し、`from_yaml()` と `to_yaml()` を実装します：

```python
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

```python
pyrs_yaml.register_type("!timestamp", TimestampType())
```

**デコレータ形式：**

```python
@pyrs_yaml.register_type("!timestamp")
class TimestampType(pyrs_yaml.CustomType):
    ...
```

### 使用

**タグ付きスカラの読み込み：**

```python
doc = pyrs_yaml.parse("when: !timestamp 2026-08-11T10:30:00")
val = doc.get("when")
assert isinstance(val, datetime)
```

**Python オブジェクトのダンプ：**

```python
data = {"ts": datetime(2026, 8, 11, 10, 30)}
out = pyrs_yaml.safe_dump(data)
# out に含まれる: ts: !timestamp 2026-08-11T10:30:00
```

### API リファレンス

| メソッド | 説明 |
|---------|------|
| `can_parse(node)` | この型が AST ノードを処理するかどうか |
| `from_yaml(value)` | YAML 文字列を Python オブジェクトに変換 |
| `to_yaml(obj)` | Python オブジェクトを YAML 文字列に変換 |
| `validate(obj)` | Python オブジェクトを検証（`bool` を返す） |

### 例：UUID 型

```python
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
