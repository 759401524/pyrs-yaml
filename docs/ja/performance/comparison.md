---
title: 他ライブラリとの比較
description: pyrs-yaml を PyYAML や ruamel.yaml と比較。パフォーマンス、機能、移行パスをカバーします。
tags:
  - docs
status: new
---

pyrs-yamlを、最も人気のある2つのPython YAMLライブラリと比較します。

## パフォーマンス比較

### パース速度（大規模YAML、約2 KB）

| ライブラリ | 時間 | 高速化率 |
|---------|------|---------|
| **pyrs-yaml** | **1.5 ms** | — |
| PyYAML | 57.7 ms | 38倍遅い |
| ruamel.yaml | 127.9 ms | 85倍遅い |

### シリアライズ速度（大規模YAML、約2 KB）

| ライブラリ | 時間 | 高速化率 |
|---------|------|---------|
| **pyrs-yaml** | **0.17 ms** | — |
| PyYAML | 30.2 ms | 177倍遅い |
| ruamel.yaml | 63.1 ms | 371倍遅い |

### ラウンドトリップ速度（大規模YAML、約2 KB）

| ライブラリ | 時間 | 高速化率 |
|---------|------|---------|
| **pyrs-yaml** | **1.6 ms** | — |
| PyYAML | 87.9 ms[^1] | 55倍遅い |
| ruamel.yaml | 191.0 ms[^1] | 119倍遅い |

[^1]: PyYAML/ruamel.yaml のラウンドトリップ時間は、同じベンチマークのパースとシリアライズの合計による推定値です。

## 機能比較

| 機能 | pyrs-yaml | PyYAML | ruamel.yaml |
|---------|-----------|--------|-------------|
| **YAML 1.2準拠** | :material-check: | :material-check: | :material-check: |
| **コメント（スタンドアロン）** | :material-check: | :material-close: | :material-check: |
| **コメント（インライン）** | :material-check: | :material-close: | :material-check: |
| **アンカー/エイリアス** | :material-check: | :material-close: | :material-check: |
| **タグ（明示的）** | :material-check: | :material-close: | :material-check: |
| **ブロックスカラー** | :material-check: | :material-check: | :material-check: |
| **フローコレクション** | :material-check: | :material-check: | :material-check: |
| **マージキー（<<）** | :material-check: | :material-close: | :material-check: |
| **複合キー** | :material-check: | :material-check: | :material-check: |
| **ラウンドトリップ保持** | :material-check: | :material-close: | :material-check: |
| **Pythonバインディング** | :material-check: | :material-check: | :material-check: |
| **ABI3（py3.8+）** | :material-check: | :material-close: | :material-close: |
| **型スタブ（.pyi）** | :material-check: | :material-check: | :material-close: |
| **多言語エラーメッセージ** | :material-check: | :material-close: | :material-close: |
| **Rustバックエンド** | :material-check: | :material-close: | :material-close: |
| **パフォーマンス** | :material-rocket-launch: 最速 | :material-snail: 遅い | :material-snail: 遅い |

## まとめ

### pyrs-yamlを選ぶべき場合

- **パフォーマンスが重要** — PyYAMLより解析で21〜43倍、シリアライズで55〜177倍高速
- **ラウンドトリップ保持が重要** — コメント、アンカー、タグを保持
- **PyYAML互換性が欲しい** — 差し替え可能なAPI
- **型ヒントが必要** — 完全な`.pyi`スタブ
- **単一wheelで配布したい** — ABI3はPython 3.8〜3.15全体で動作

### PyYAMLを選ぶべき場合

- すでに使用しており、ラウンドトリップ保持が必要ない
- 既存コードとの最大限の互換性が必要
- パフォーマンスを気にしない

### ruamel.yamlを選ぶべき場合

- 最も機能豊富なYAMLパーサーが必要
- 複雑なYAML操作を行っている
- パフォーマンスを気にしない（最も遅い選択肢）

## 移行パス

```python
# ステップ1: インストール
pip install pyrs-yaml

# ステップ2: インポートを置換
# Before:
import yaml

# After:
import pyrs_yaml as yaml

# ステップ3: テスト
# 既存のテストを実行して互換性を確認
```

ほとんどのコードは変更なしで動作します。主な違い：

1. ラウンドトリップ出力はコメントとフォーマットを保持します
2. エラーメッセージは詳細で、ローカライズ可能です
3. パフォーマンスは大幅に向上します
