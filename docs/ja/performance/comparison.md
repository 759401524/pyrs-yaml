---
title: 他ライブラリとの比較
description: pyrs-yaml を PyYAML や ruamel.yaml と比較。パフォーマンス、機能、移行パスをカバーします。
tags:
  - docs
status: new
---

pyrs-yamlを、最も人気のある2つのPython YAMLライブラリと、Rust バックエンドの読み取り器である `yaml-rs` と `ryaml` を用いて比較します。

## パフォーマンス比較

### パース速度（大規模YAML、約2 KB）

<!-- bench: layout=libraries format=yaml operation=parse size=large -->
| ライブラリ | 時間 | 高速化率 |
|---------|------|---------|
| **pyrs-yaml** | 72 µs | — |
| PyYAML | 2.3 ms | 32× slower |
| ruamel.yaml | 5.4 ms | 75× slower |
| yaml-rs | 27 µs | 2.7× faster |
| ryaml | 86 µs | 1.2× slower |

### シリアライズ速度（大規模YAML、約2 KB）

<!-- bench: layout=libraries format=yaml operation=serialize size=large -->
| ライブラリ | 時間 | 高速化率 |
|---------|------|---------|
| **pyrs-yaml** | 5.4 µs | — |
| PyYAML | 1.4 ms | 261× slower |
| ruamel.yaml | 3.3 ms | 603× slower |
| yaml-rs | 17 µs | 3.1× slower |
| ryaml | 42 µs | 7.8× slower |

### ラウンドトリップ速度（大規模YAML、約2 KB）

<!-- bench: layout=libraries format=yaml operation=roundtrip size=large -->
| ライブラリ | 時間 | 高速化率 |
|---------|------|---------|
| **pyrs-yaml** | 82 µs | — |
| PyYAML | 3.9 ms | 48× slower |
| ruamel.yaml | 9.1 ms | 111× slower |
| yaml-rs | 48 µs | 1.7× faster |
| ryaml | 135 µs | 1.6× slower |

## プレーンなオブジェクトを返す Rust パーサとのトレードオフ

pyrs-yaml は**ラウンドトリップ用 AST を保つ**（コメント・アンカー・タグ・ソース span が load→edit→dump を生き延びる）YAML ライブラリ中最速です。プレーンな Python オブジェクトへ逆シリアル化するだけでラウンドトリップ編集を**提供しない** Rust パーサ（上の表の `yaml-rs`）と比べると、結果は分かれます。

- **シリアライズ：** pyrs-yaml が同じフィクスチャで約 2.3〜3.7 倍速（上の表を参照）。
- **`YamlDocument` へのパース：** `yaml-rs` が作らない、より豊かなラウンドトリップ構造を構築するため、pyrs-yaml は `yaml-rs` より約 2.5 倍遅い。これは意図した設計上のトレードです——余剰の作業がコメント/アンカー/順序の保持を生むものなので、排除すべき退行ではありません。

YAML を*読む*のみ・パースの最後の一秒が重要・元の書式を保ったまま書き戻す必要が一切ない場合は `yaml-rs` のようなピュア・ネイティブ パーサを、ラウンドトリップの忠実性が重要なら pyrs-yaml を選んでください——その場合でも PyYAML より解析で 32〜37 倍、シリアライズで 204〜347 倍高速です。

## 機能比較

| 機能 | pyrs-yaml | PyYAML | ruamel.yaml |
|---------|-----------|--------|-------------|
| **YAML 1.2準拠** | :material-check: | :material-check: | :material-check: |
| **コメント（スタンドアロン）** | :material-check: | :material-close: | :material-check: |
| **コメント（インライン）** | :material-check: | :material-close: | :material-check: |
| **アンカー/エイリアス** | :material-check: | :material-check: | :material-check: |
| **タグ（明示的）** | :material-check: | :material-check: | :material-check: |
| **ブロックスカラー** | :material-check: | :material-check: | :material-check: |
| **フローコレクション** | :material-check: | :material-check: | :material-check: |
| **マージキー（<<）** | :material-check: | :material-check: | :material-check: |
| **複合キー** | :material-check: | :material-check: | :material-check: |
| **ラウンドトリップ保持** | :material-check: | :material-close: | :material-check: |
| **Pythonバインディング** | :material-check: | :material-check: | :material-check: |
| **ABI3（py3.8+）** | :material-check: | :material-close: | :material-close: |
| **型スタブ（.pyi）** | :material-check: | :material-close: | :material-close: |
| **多言語エラーメッセージ** | :material-check: | :material-close: | :material-close: |
| **Rustバックエンド** | :material-check: | :material-close: | :material-close: |
| **パフォーマンス** | :material-rocket-launch: 最速 | :material-snail: 遅い | :material-snail: 遅い |

## まとめ

### pyrs-yamlを選ぶべき場合

- **パフォーマンスが重要** — PyYAMLより解析で32〜37倍、シリアライズで204〜347倍高速
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

```python title="移行ステップ"
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
