---
title: pyrs-yaml
description: 高性能な Python YAML ライブラリ。完璧な Round-Trip サポートを備え、Rust と PyO3 で構築されています。
tags:
  - docs
status: new
---

## 高性能な Python YAML ライブラリ、完璧な Round-Trip サポート、Rust と PyO3 で構築されています。

### なぜ pyrs-yaml を選ぶべきか？

ほとんどの Python YAML ライブラリは、パフォーマンスと忠実性のどちらかを犠牲にします。 pyrs-yaml の両方を提供します:

- **PyYAML** (Python) — 遅く、往復解析時に**コメント/アンカー/タグを失う**
- **ruamel.yaml** (Python) — フォーマットを保持するが、pyrs-yaml より **5–10 倍遅い**
- **pyrs-yaml** (Rust) — PyYAML より **25–40 倍高速**、すべてを保持

#### 主要機能

- **YAML 1.2 準拠** — saphyr-parser 駆動（YAML テストスイート 98.1% 合格率）
- **完璧なラウンドトリップ** — コメント、アンカー、タグ、チョンピング、スカラースタイル、フロー/ブロックフォーマットを保持
- **インプレース編集** — JSONPath スタイルのパス（`doc.set("$.a.b", v)`）または `Node` ツリー API で解析済みドキュメントを編集、フォーマットを失わない
- **PyYAML より 25–40 倍高速** — Rust バックエンド、ゼロコピー解析
- **カスタム AST** — 高度な YAML 操作とカスタムフォーマット用の拡張可能な AST
- **PyYAML 互換** — `safe_load` / `safe_dump` API で直接置換可能
- **型ヒント** — PEP 561 準拠、完全な `.pyi` スタブファイル
- **ABI3** — 単一のホイールで Python 3.9–3.13 に対応
- **国際化エラーメッセージ** — `set_language("ja")` でバイリンガルエラーレポート
- **NumPy ndarray サポート** — 任意次元の `numpy.ndarray` をゼロコピー Rust ディスパッチで YAML にシリアライズ

#### クイックスタート

```bash
pip install pyrs-yaml
```

```python
import pyrs_yaml

# Parse YAML
doc = pyrs_yaml.parse("key: value")
print(doc.to_yaml())  # key: value\n

# PyYAML compatible API
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# Round-trip preserves comments
original = "# Comment\nkey: value  # inline\n"
doc = pyrs_yaml.parse(original)
assert doc.to_yaml() == original
```

#### PyYAML との比較

| Operation | pyrs-yaml | PyYAML | Speedup |
|-----------|-----------|--------|---------|
| Parse (small) | 0.00 ms | 0.11 ms | **25×** |
| Parse (medium) | 0.03 ms | 0.75 ms | **28×** |
| Parse (large) | 0.07 ms | 1.83 ms | **26×** |
| Serialize (small) | 0.01 ms | 0.19 ms | **36×** |
| Serialize (medium) | 0.03 ms | 1.21 ms | **40×** |
| Serialize (large) | 0.08 ms | 2.96 ms | **37×** |

---

### [クイックスタート →](quick-start.md)

### [API リファレンスを参照 →](api/reference.md)

### [GitHub で見る →](https://github.com/759401524/pyrs-yaml)
