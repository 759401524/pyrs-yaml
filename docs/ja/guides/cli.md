---
title: コマンドラインインターフェース
description: pyrs-yaml CLI を使って、ターミナルから YAML ファイルのフォーマット、クエリ、編集、バリデーション、変換をラウンドトリップ品質で行います。
tags:
  - docs
status: new
---

## コマンドラインインターフェース

pyrs-yaml はオプションのコマンドラインツール `pyrs-yaml` を同梱しています。ライブラリの中核機能——ラウンドトリップ整形、JSONPath クエリ、インプレース編集、スキーマバリデーション、フォーマット変換——をターミナルから直接利用できます。

!!! note "要件"
    CLI にはオプションの `cli` extra と **Python >= 3.10** が必要です。ライブラリ本体は引き続き旧インタープリターをサポートします。

### インストール

=== "pip"

    ```bash
    pip install "pyrs-yaml[cli]"
    ```

=== "uv"

    ```bash
    uv add --optional cli pyrs-yaml
    ```

インストールの確認：

```bash
pyrs-yaml --version
```

### コマンド一覧

| コマンド | 用途 |
|---------|---------|
| [`fmt`](#cmd-fmt) | コメント・アンカー・順序を保持して再整形 |
| [`get`](#cmd-get) | JSONPath 式で値を取得 |
| [`set`](#cmd-edit) | パス位置の値を設定 |
| [`delete`](#cmd-edit) | パス位置のノードを削除 |
| [`rename`](#cmd-edit) | マッピングキーを改名 |
| [`validate`](#cmd-validate) | スキーマに対して YAML を検証 |
| [`to-json`](#cmd-convert) | YAML を JSON に変換 |
| [`from-json`](#cmd-convert) | JSON を YAML に変換 |

ファイル引数が `-` または省略された場合は **stdin** から読み込み、`-o/--output` や `-i/--inplace` がない限り結果は **stdout** に出力されます。

### フォーマット（`fmt`） { #cmd-fmt }

`fmt` はラウンドトリップ AST を経由してドキュメントを再シリアライズします——コメント・アンカー・キー順・スタイルはすべて保持されます:

```bash
$ echo "a:    1 # keep me" | pyrs-yaml fmt -
a: 1  # keep me
```

主なオプション：

```bash
pyrs-yaml fmt config.yaml --indent 4        # 4 スペースのインデント
pyrs-yaml fmt config.yaml --inplace         # ファイルを直接書き換え（-i）
pyrs-yaml fmt config.yaml -o formatted.yaml # 別ファイルへ出力
```

### クエリ（`get`） { #cmd-get }

`get` は [JSONPath](editing.md) 風の式を評価し、一致した各ノードを出力します:

```bash
$ pyrs-yaml get deploy.yaml '$.servers[0].host'
db.example.com

$ pyrs-yaml get deploy.yaml '$..name' --format text   # 深さ優先探索
web
db

$ pyrs-yaml get deploy.yaml '$.servers[*]'            # サブツリーは YAML で出力（デフォルト）
```

`--format/-f` で出力形式を指定できます：`yaml`（デフォルト）、`json`、`text`（スカラー値そのもの）。

### 編集（`set`、`delete`、`rename`） { #cmd-edit }

編集コマンドのパスは単一ノードを正確に指す必要があります（ワイルドカードは不可）:

```bash
# VALUE は YAML として解析されます——数値・真偽値・ネスト構造もそのまま書けます
pyrs-yaml set config.yaml "$.retries" 5
pyrs-yaml set config.yaml "$.tags" '[a, b]'
pyrs-yaml set config.yaml "$.token" '12345' --string          # 文字列として扱う
pyrs-yaml set config.yaml "$.a.b.c" new --create-missing      # 親を自動作成

pyrs-yaml delete config.yaml "$.legacy_key"
pyrs-yaml rename config.yaml "$.old_name" new_name

pyrs-yaml set config.yaml "$.port" 8080 --inplace             # ファイルを直接編集
```

編集しても周辺のメタデータは保持されます——編集したノードの上や行内のコメントはそのまま残ります。

### バリデーション（`validate`） { #cmd-validate }

`validate` はスキーマ定義（ファイルパス）または登録済みスキーマ名に基づいてドキュメントを検証します:

```bash
pyrs-yaml validate app.yaml --schema schema.yaml
pyrs-yaml validate app.yaml --schema my_schema        # register_schema() で登録したもの
```

成功時は無音で終了コード `0`。失敗時はすべての違反内容を stderr に出力し終了コード `1` で終わるため、CI での利用に最適です:

```yaml
# schema.yaml
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
```

スキーマ言語の詳細は[カスタムスキーマ](custom-schema.md)を参照してください。

### 変換（`to-json`、`from-json`） { #cmd-convert }

どちらの方向もパイプラインと自然に組み合わせられます:

```bash
$ pyrs-yaml to-json config.yaml
{
  "b": {
    "c": 2
  }
}

$ echo '{"name": "x"}' | pyrs-yaml from-json -
name: x
```

### 終了コード

| コード | 意味 |
|------|---------|
| `0` | 成功 |
| `1` | 実行時エラー —— 入力が読めない・パース失敗・マッチなし・検証失敗 |
| `2` | 使用方法エラー —— 不明なコマンドやオプション |

!!! tip "スクリプト化"
    エラーは stderr、データは stdout に出るため、`pyrs-yaml` はパイプラインと組み合わせやすい設計です：`pyrs-yaml get deploy.yaml '$..host' | sort -u`。
