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
| [`sort-keys`](#cmd-edit) | パス位置のマッピングキーをソート |
| [`move`](#cmd-edit) | サブツリーを別の既存パスへ移動 |
| [`frontmatter`](#cmd-frontmatter) | Markdown フロントマターを YAML で抽出 |
| [`validate`](#cmd-validate) | スキーマに対して YAML を検証 |
| [`to-json`](#cmd-convert) | YAML を JSON に変換 |
| [`from-json`](#cmd-convert) | JSON を YAML に変換 |
| [`compliance`](#cmd-compliance) | YAML Test Suite の適合率をレポート |

ファイル引数が `-` または省略された場合は **stdin** から読み込み、`-o/--output` や `-i/--inplace` がない限り結果は **stdout** に出力されます。ストリーム入力は [`-A/--all-docs`](#multi-doc) で扱います。

YAML を読み込むサブコマンドはすべて `--duplicate-keys` を受け取ります — `fmt`、`get`、`set`、`delete`、`rename`、`sort-keys`、`move`、`validate`、`to-json`、`to-toml`、`to-jsonc`、`to-json5`。これは、ひとつのマッピングに YAML が同じと数えるキーが二つあるときの読み手の答えです: `as_is`（既定）、`error`、`last_wins`、`first_wins` のいずれか。四つ以外の値は stderr で拒否されます — 四つの名前を挙げて、終了コード `1` で終わります。`as_is` または `error` で重複を読み込むときの拒否は `YamlDuplicateKeyError` です。

```bash
$ pyrs-yaml get twins.yaml '$.a' --duplicate-keys last_wins    # 後のペアが勝つ
2

$ pyrs-yaml get twins.yaml '$.a' --duplicate-keys first_wins   # 先のペアが勝つ
1

$ pyrs-yaml get twins.yaml '$.a' --duplicate-keys newest       # 四つに含まれない値
error: unknown duplicate_keys "newest"; expected as_is, error, last_wins or first_wins
```

ここに登場する `twins.yaml` はプレーンな重複 `a: 1` / `a: 2` を持つファイルで、`as_is` か `error` で読む場合は上で挙げた拒否になります。`last_wins` はライブラリの `allow_duplicate_keys=True` の意味です。`as_is` がクラス別の答えをどう返すかは [重複キー](../features.md) を参照してください。

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

pyrs-yaml sort-keys config.yaml                               # ルートのキーをソート
pyrs-yaml sort-keys config.yaml "$.meta"                      # 単一のネストしたマッピングをソート
pyrs-yaml move deploy.yaml "$.staging" "$.environments.dev"   # サブツリーを移動
```

編集しても周辺のメタデータは保持されます——編集したノードの上や行内のコメントはそのまま残ります。

「パスにつき正確に 1 ノード」が指すのは 1 *ペア*です。そしてドキュメントは、同じキーをつづる二つのペアを持てます——読み手が保つタグかアンカーでのみ両者は区別されます。その扱いを尋ねるのが `delete` です: `--on-duplicate` は `one`（既定、パスが解決するペア）、`all`（そのキーが答えになり得るすべてのペア——出力からも名前が消えます）、`error`（二つ以上残っているあいだは拒否）のいずれかを受け取ります。

```bash
$ cat dup.yaml
a: 1
!!str a: 2
b: 3

$ pyrs-yaml delete dup.yaml '$.a' --on-duplicate all
b: 3

$ pyrs-yaml delete dup.yaml '$.a' --on-duplicate error
error: cannot delete '$.a': edit failed: duplicate-key-target

$ pyrs-yaml delete dup.yaml '$.a' --on-duplicate both
error: cannot delete '$.a': edit failed: unknown on_duplicate "both"; expected one, all or error
```

`--on-duplicate error` はキーごとにペアが 1 つだけのドキュメントでは何も起きません——拒否が応えるのは重複が実際に残っているあいだだけです。`set`、`rename`、`sort-keys`、`move` は `one` の振る舞いのままです——そのようなフラグを持ちません。

注意：

- 親が存在する場合、`set` は `--create-missing` なしでもパスの最終キーを追加します。このフラグが必要なのは**中間**キーが欠けている場合のみです。
- `sort-keys` は `path` のマッピング（デフォルトはルート）のキーをソートします。再帰的ではありません。
- `move` の宛先は既に存在している必要があり、その値は移動したサブツリーで置き換えられます。両端でワイルドカードは使えません。

### バリデーション（`validate`） { #cmd-validate }

`validate` は JSON Schema ファイルに基づいてドキュメントを検証します。`--schema` は `--schema-file` のエイリアスで、どちらも指定しない場合は構文チェックのみを行います:

```bash
pyrs-yaml validate app.yaml --schema-file schema.json
pyrs-yaml validate app.yaml --schema schema.json        # 同じパラメーター
```

評価は Python の `jsonschema` に委譲されるため、ファイルは標準の JSON Schema ドキュメントです:

```json
{
  "type": "object",
  "properties": {"port": {"type": "integer"}},
  "required": ["port"]
}
```

成功時は無音で終了コード `0`、失敗時はリファレンスエンジンのメッセージを stderr に出力して終了コード `1` で終わります——CI での利用に最適です。実測では、通る文書はどちらのストリームにも何も書かず `0` で終わり、スキーマが拒否する文書は `error: 'port' is a required property` を stderr に出して `1` で終わり、解析できない文書は同じ終了コードで `error: YAML parse error: mapping values are not allowed in this context at char 4 line 1 column 5` を印字します。

その文の外に答えるスキーマ入力が二つあります。評価は Python の `jsonschema` に委譲され、本プロジェクトが公開するどれにもそれはインストールされないためです——基底の依存は `typing-extensions` だけで、extra は `cli`、`pydantic`、`settings` です：

- JSON ではないスキーマのファイル——読み手が手を伸ばす YAML の綴り——は、評価器を参照する前に `json.loads` に止められます。だからコマンドは、最後の行が `json.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)` である traceback を印字して `1` で終わります（コンソールスクリプト経由で実測：traceback は stderr、stdout は空のまま）。
- `jsonschema` のないインストールで評価器までに届いた JSON のスキーマも、同じ終わり方になります。`ModuleNotFoundError` の traceback です。こちらは評価器がこの開発 venv に入っているので、プロセス内で `sys.modules["jsonschema"] = None` を使って実測しました——テストスイートが使う再現です——コマンドの `except YamlValidateError` 節はどちらのクラスも受け取らないため、行は `error: …` ではなく traceback になります。

両方とも `.ci/known-residuals.json` に登記済みの境界（`api:validate-schema-text-is-json-only` と `api:validate-evaluator-is-not-installed-by-anything-we-ship`）で、CLI が直す欠陥ではありません。[例外 — YamlValidateError](../api/exceptions.md#yamlvalidateerror) を参照してください。

スカラー解決用スキーマと `Document.validate()` API は[カスタムスキーマ](custom-schema.md)を参照してください。

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

### 複数ドキュメントストリーム { #multi-doc }

`-A/--all-docs` を付けると、最初のドキュメントだけでなく `---` 区切りのドキュメント列全体を入力として扱います:

```bash
pyrs-yaml fmt stream.yaml -A                              # 全ドキュメントを再整形
pyrs-yaml get stream.yaml '$..name' --format text -A      # ドキュメント横断でクエリ
pyrs-yaml to-json stream.yaml -A                          # ドキュメントの JSON 配列を出力
pyrs-yaml set stream.yaml "$.retries" 5 -A                # 全ドキュメントを編集
pyrs-yaml move stream.yaml "$.src" "$.nested" -A          # パスが解決できるドキュメントで移動
pyrs-yaml validate stream.yaml --schema-file s.yaml -A    # 失敗時は "document N" と報告
```

対象コマンド: `fmt`、`get`、`set`、`delete`、`rename`、`sort-keys`、`move`、`validate`、`to-json`。出力は標準の `---` 区切りで連結されます。編集コマンドはパスが解決できる各ドキュメントに適用され、1 つも一致しない場合のみ失敗します。

### Markdown フロントマター（`frontmatter`） { #cmd-frontmatter }

```bash
$ pyrs-yaml frontmatter post.md
title: Hello

$ pyrs-yaml frontmatter post.md --body-out body.md   # 本文も分割して出力
```

フロントマターがないページでは終了コード `1` で終わります。ライブラリ API の詳細は [Markdown フロントマター](frontmatter.md) を参照してください。

### YAML Test Suite 適合率（`compliance`） { #cmd-compliance }

```bash
pyrs-yaml compliance [--json] [SUITE_DIR]
```

[yaml-test-suite](https://github.com/yaml/yaml-test-suite) コーパスに対してパーサーを実行し（デフォルトのチェックアウト場所: `./Reference/yaml-test-suite`)、セクションごとの合否統計を出力します。他の YAML 実装と pyrs-yaml を比較評価する際に有用です。

### 終了コード

| コード | 意味 |
|------|---------|
| `0` | 成功 |
| `1` | 実行時エラー —— 入力が読めない・パース失敗・マッチなし・検証失敗 |
| `2` | 使用方法エラー —— 不明なコマンドやオプション |

!!! tip "スクリプト化"
    エラーは stderr、データは stdout に出るため、`pyrs-yaml` はパイプラインと組み合わせやすい設計です：`pyrs-yaml get deploy.yaml '$..host' | sort -u`。
