---

title: 往復保存
lang: ja

## 往復保存

これは pyyaml-rs の**最大の特徴** — Python YAML ライブラリの中でユニークな点です。

### 往復保存とは？

往復保存とは：**YAML をパース → 変更 → シリアライズ → 出力が入力と同一（または意味的に同等）** であることを意味します。

```python
original = """
# サーバー設定
server:
  host: 0.0.0.0
  port: 8080  # メインポート

# データベースアンカー
database: &db
  host: localhost
  port: 5432

api:
  <<: *db
  endpoint: /api/v1
"""

doc = pyyaml_rs.parse(original)
output = doc.to_yaml()

# すべてのフォーマットとメタデータが保持される
assert "# サーバー設定" in output
assert "# メインポート" in output
assert "&db" in output
assert "<<: *db" in output
```

### 保持されるもの

| 要素 | 保持されるか | 備考 |
|------|------------|------|
| 独立行コメント | ✅ | キーと値の前 |
| 行末コメント | ✅ | 行の末尾 |
| アンカー (`&name`) | ✅ | 完全なアンカーシンタックス |
| エイリアス (`*name`) | ✅ | エイリアス参照が解決される |
| マージキー (`<<`) | ✅ | デフォルトで解決される |
| タグ (`!!str`, `!!int`) | ✅ | 明示的なタグが保持される |
| スカラースタイル | ✅ | Plain, 引用符付き, リテラル, フォールド |
| チョンピング (`\|-`, `>-`) | ✅ | ブロックスカラーインジケーター |
| フロー/ブロックスタイル | ✅ | `[]`/`{}` vs ブロックが保持される |
| キーの順序 | ✅ | `IndexMap` が順序を保証 |

### PyYAML vs pyyaml-rs 往復保存

```python
original = "# コメント\nkey: value  # 行末\n"

# PyYAML: すべて失われる
yaml.safe_dump(yaml.safe_load(original))
# 出力: 'key: value\n'  ❌

# pyyaml-rs: すべて保持される
doc = pyyaml_rs.parse(original)
doc.to_yaml()
# 出力: '# コメント\nkey: value  # 行末\n'  ✅
```

### パフォーマンス

他のライブラリとの往復パフォーマンス比較：

| ライブラリ | 往復保存 (大) | コメント | アンカー | タグ |
|-----------|-------------|---------|---------|------|
| **pyyaml-rs** | **0.08 ms** | ✅ | ✅ | ✅ |
| PyYAML | 2.98 ms | ❌ | ❌ | ❌ |
| ruamel.yaml | 6.79 ms | ✅ | ✅ | ✅ |

**pyyaml-rs は PyYAML より 37 倍速く、ruamel.yaml より 85 倍速く**、すべてを保持します。
