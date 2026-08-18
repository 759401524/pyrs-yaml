---
title: 往復保存
description: pyrs-yaml の往復保存機能について説明します。パース、変更、シリアライズ後もフォーマットとメタデータを保持します。
tags:
  - docs
status: new
---

これは pyrs-yaml の**最大の特徴** — Python YAML ライブラリの中でユニークな点です。

## 往復保存とは？

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

doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# すべてのフォーマットとメタデータが保持される
assert "# サーバー設定" in output
assert "# メインポート" in output
assert "&db" in output
# 注意：マージキー（<<）はデフォルトで解決（実体化）され、そのまま出力されません。
# <<: *db をそのまま保持するには resolve_merges=False を使用してください
```

## 保持されるもの

| 要素 | 保持されるか | 備考 |
|------|------------|------|
| 独立行コメント | :material-check: | キーと値の前 |
| 行末コメント | :material-check: | 行の末尾 |
| アンカー (`&name`) | :material-check: | 完全なアンカーシンタックス |
| エイリアス (`*name`) | :material-check: | エイリアス参照が解決される |
| マージキー (`<<`) | :material-alert: | デフォルトで解決される；`resolve_merges=False` で保持 |
| タグ (`!!str`, `!!int`) | :material-check: | 明示的なタグが保持される |
| スカラースタイル | :material-check: | Plain, 引用符付き, リテラル, フォールド |
| チョンピング (`\|-`, `>-`) | :material-check: | ブロックスカラーインジケーター |
| フロー/ブロックスタイル | :material-check: | `[]`/`{}` vs ブロックが保持される |
| コンパクトなシーケンス項目 | :material-check: | `- host: a` はダッシュ行に留まる（メタデータのないマッピング項目のみ） |
| キーの順序 | :material-check: | `IndexMap` が順序を保証 |

## PyYAML vs pyrs-yaml 往復保存

```python
original = "# コメント\nkey: value  # 行末\n"

# PyYAML: すべて失われる
yaml.safe_dump(yaml.safe_load(original))
# 出力: 'key: value\n'  :material-close:

# pyrs-yaml: すべて保持される
doc = pyrs_yaml.parse(original)
doc.to_yaml()
# 出力: '# コメント\nkey: value  # 行末\n'  :material-check:
```

## パフォーマンス

他のライブラリとの往復パフォーマンス比較：

| ライブラリ | 往復保存 (大) | コメント | アンカー | タグ |
|-----------|-------------|---------|---------|------|
| **pyrs-yaml** | **0.08 ms** | :material-check: | :material-check: | :material-check: |
| PyYAML | 2.98 ms | :material-close: | :material-close: | :material-close: |
| ruamel.yaml | 6.79 ms | :material-check: | :material-check: | :material-check: |

**pyrs-yaml は PyYAML より 37 倍速く、ruamel.yaml より 85 倍速く**、すべてを保持します。
