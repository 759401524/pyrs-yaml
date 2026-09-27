---
title: pyq - Rust ネイティブ CLI
description: pyrs-yaml-core ベースの jq/yq スタイル Rust CLI。YAML・JSON・TOML・INI を実行時 Python なしで処理。
tags:
  - docs
status: new
---

`pyq` は Rust ネイティブのコマンドラインツールです。`pyrs-yaml-core` を
jq/yq スタイルのインターフェースに直結し、実行時に Python を必要としません。
Python 版の [pyrs-yaml CLI](cli.md) と補完関係にあり、両者は同じコアと
終了コード・エラーメッセージの仕様を共有します。

## インストール

```bash
cargo install --path crates/pyrs-yaml-cli   # ソースツリーから
# もしくはリポジトリ内でビルド：
cargo build -p pyrs-yaml-cli --release      # -> target/release/pyq
```

## クエリ（jq スタイル）

```bash
# JSONPath-lite：ドットキー、[n]、[-n]（Python 式負インデックス）、['key']、[*]
$ pyq get '.servers[-1].host' inventory.yaml
web-3

# ファイルが `-` または省略時は stdin、--raw で素のスカラー
$ cat services.yaml | pyq get --raw .db.pool.size
20

# JSON 出力（キー順序保持）
$ pyq get '.servers' --json services.yaml
[ { "host": "web-1", "port": 8080 }, ... ]
```

## 編集（yq スタイル）

```bash
# 値は YAML 式（JSON も可、YAML はその上位互集合）
pyq set '.db.pool.size' 50 services.yaml          # 編集後ドキュメントを表示
pyq set -i '.db.pool.size' 50 services.yaml       # ファイルをその場で書き換え
pyq set --create-missing '.a.b.c' 1 empty.yaml    # 中間マッピングを自動生成
pyq delete '.legacy_field' -i config.yaml
```

編集後の出力は `fmt` と同じラウンドトリップ・シリアライザを経由します：
コメント・アンカー・キー順序は保持され、注入した値は自身の表記スタイルを
保ちます（`[1, two]` はフローのまま、`"true"` はクォート文字列のまま）。

## 変換

```bash
pyq fmt k8s.yaml                 # コメント保持の正規化
pyq fmt --explicit-start cfg.yaml
pyq to-json config.yaml          # YAML -> JSON（キー順序保持）
pyq to-toml compose.yaml
pyq from-toml Cargo.toml         # TOML -> YAML
pyq from-json package.json       # JSON -> YAML
pyq from-ini settings.ini        # INI -> YAML（値はすべて文字列）
```

入力形式は拡張子で判定（`.json`・`.toml`・`.ini`）され、
`--input yaml|json|toml|ini` で上書きできます。YAML は JSON の上位互集合
なので、JSON 内容は YAML 経路でもそのまま解析できます。

## 終了コード

- `0` 成功；
- `1`：パス未検出・解析失敗・TOML で表現不能な構造（null 値、テーブル
  以外ルート）では stderr に `pyq: <メッセージ>` — Python API と同じ
  安定メッセージ。

## 対応範囲

| 機能 | pyq | pyrs-yaml CLI（Python） |
|------|-----|--------------------------|
| クエリ / set / delete / 整形 / 変換 | ✅ | ✅ |
| マルチドキュメント（`-A`） | 計画中 | ✅ |
| 登録スキーマでの `validate` | 計画中 | ✅ |
| sort-keys / rename / move / frontmatter | — | ✅ |

多機能面は Python 版 CLI が担い（スプライスによるレイアウト保持編集が
可能）、`pyq` は単一ドキュメントの高速・無依存スクリプティングを目標とします。

## 関連

- [コマンドラインインターフェース](cli.md) — Python 版 `pyrs-yaml` コマンド
- [TOML・JSON・INI 形式](formats.md) — ライブラリ側の変換 API
- [インプレース編集](editing.md) — `pyq` が使うラウンドトリップモデル
