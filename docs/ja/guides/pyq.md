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
# 先頭のドットは省略可、`.` 単体はドキュメント全体
$ pyq get '.servers[-1].host' inventory.yaml
web-3

# ワイルドカードは全マッチに展開され jq のようにストリーム出力：
# マッチごとに YAML ドキュメント（--json では 1 行 1 値）
$ pyq get '.servers[*].port' --json inventory.yaml
8080
8081

# ファイルが `-` または省略時は stdin、--raw で素のスカラー
$ cat services.yaml | pyq get --raw .db.pool.size
20

# JSON 出力（キー順序保持）
$ pyq get '.servers' --json services.yaml
[ { "host": "web-1", "port": 8080 }, ... ]
```

## 絞り込み動詞（jq スタイルの後処理）

式言語ではなく構造化フラグ——コマンドラインの順序に関係なく、
マッチ列に固定パイプライン `select -> sort -> unique -> slice`、最後に `join` を適用。

```bash
pyq get '.servers[*]' --select 'port >= 1000' services.yaml
pyq get '.servers[*]' --sort-by host --desc services.yaml
pyq get '.tags[*]' --unique --skip 2 --take 5 blob.yaml
pyq get '.hosts[*]' --join ',' --raw inventory.yaml   # 1 行のむき出しテキスト
```

| フラグ | jq 対応 | 備考 |
|--------|---------|------|
| `--select 'PATH OP LITERAL'` | `select(.PATH OP LITERAL)` | OP は `== != > >= < <=`。リテラルは YAML。パス欠損・型不一致は false（jq の全順序なし） |
| `--sort-by PATH` / `--desc` | `sort_by(.PATH)` | 安定ソート。キー欠損は最後 |
| `--unique` | `unique` | ソート後の重複排除、jq と同じ |
| `--first` / `--last` | `.[0]` / `.[-1]` | 排他 |
| `--skip N` / `--take N` | `.[N:][…]` | 列スライス |
| `--join SEP` | `join(SEP)` | 全スカラー列のみ |

## 編集（yq スタイル）

```bash
# 値は YAML 式（JSON も可、YAML はその上位互集合）
pyq set '.db.pool.size' 50 services.yaml          # 編集後ドキュメントを表示
pyq set -i '.db.pool.size' 50 services.yaml       # ファイルをその場で書き換え
pyq set --create-missing '.a.b.c' 1 empty.yaml    # 中間マッピングを自動生成
pyq delete '.legacy_field' -i config.yaml
pyq sort-keys '$' -i config.yaml                  # マッピング 1 階層をキー順に
```

すべての編集は共有 splice エンジンを経由します：レイアウトが条件を満たせば、
触れていない行（コメント・空行・不規則な空白）は 1 バイトも変わりません。

編集後の出力は `fmt` と同じラウンドトリップ・シリアライザを経由します：
コメント・アンカー・キー順序は保持され、注入した値は自身の表記スタイルを
保ちます（`[1, two]` はフローのまま、`"true"` はクォート文字列のまま）。

## 変換

```bash
pyq fmt k8s.yaml                 # コメント保持の正規化
pyq fmt --explicit-start cfg.yaml
pyq fmt --indent 4 --width 0 cfg.yaml   # ブロックインデント 4、スカラー折り返しなし
pyq fmt --sort-keys cfg.yaml     # 全ての mapping をキー順に（ドキュメント全体）
pyq fmt --indent 4 -i cfg.yaml   # ファイルをその場で書き換え
pyq to-json config.yaml          # YAML -> JSON（キー順序保持）
pyq to-json --jsonc config.yaml  # AST に載ったコメントをそのまま再出力
pyq to-json --json5 config.yaml  # JSON5 表記（'x'、.5、+7、Infinity）
pyq to-toml compose.yaml
pyq from-toml Cargo.toml         # TOML -> YAML
pyq from-json package.json       # JSON -> YAML
pyq from-ini settings.ini        # INI -> YAML（値はすべて文字列）
pyq validate k8s.yaml --schema rules.yaml   # 構文チェック + スキーマ言語検証
pyq frontmatter README.md --body-out body.md # Markdown フロントマター分割
```

入力形式は拡張子で判定（`.json`・`.toml`・`.ini`）され、
`--input yaml|json|toml|ini` で上書きできます。YAML は JSON の上位互集合
なので、JSON 内容は YAML 経路でもそのまま解析できます。

## 終了コード

- `0` 成功；
- `1`：パス未検出・解析失敗・TOML で表現不能な構造（null 値、テーブル
  以外ルート）では stderr に `pyq: <メッセージ>` — Python API と同じ
  安定メッセージ。

## シェル補完

```bash
pyq completion bash > /etc/bash_completion.d/pyq   # bash
pyq completion zsh  > "${functions[@]:0:1}/_pyq"   # zsh
pyq completion fish | source                        # fish
pyq completion powershell > pyq.ps1                 # PowerShell
```

## 対応範囲

| 機能 | pyq | pyrs-yaml CLI（Python） |
|------|-----|--------------------------|
| クエリ / set / delete / 整形 / 変換 | ✅ | ✅ |
| 動詞後処理（`select`/`sort`/`unique`/…） | ✅ | — |
| レイアウト保持編集（splice エンジン） | ✅ | ✅ |
| パス指定 sort-keys | ✅ | ✅ |
| マルチドキュメント照会（`-A`：get/fmt/to-json） | ✅ | ✅ |
| rename / move / append / insert / frontmatter / `validate`（スキーマ言語） | ✅ | ✅ |
| マルチドキュメント編集（`-A` 全編集コマンド、`to-json -A`） | ✅ | ✅ |

両者とも同一のコア plan/splice エンジンでレイアウト保持編集を行います。
ストリームでは `pyq -A` が一歩先を行く：ドキュメントごとに独立した splice
状態を持つため、触れていないドキュメントもすべての `---` 区切り行もバイト単位で
そのまま保たれ、レイアウトが異常なドキュメントだけ個別にフォールバックします
（`to-json -A` は JSON 配列を出力）。Python 版 CLI の残る優位点は CustomType
登録による検証で、これは Python 層の概念です。

## 関連

- [コマンドラインインターフェース](cli.md) — Python 版 `pyrs-yaml` コマンド
- [TOML・JSON・INI 形式](formats.md) — ライブラリ側の変換 API
- [インプレース編集](editing.md) — `pyq` が使うラウンドトリップモデル
