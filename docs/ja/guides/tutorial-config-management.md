---
title: pyrs-yaml で設定管理
description: YAML 設定ファイルの解析・編集・検証・再シリアライズを、すべてのメタデータを保持したまま行うためのエンドツーエンドのチュートリアル。
tags:
  - docs
  - tutorial
status: new
---

## pyrs-yaml で設定管理

このチュートリアルでは、マイクロサービスアプリケーションの YAML 設定ファイルを管理するという実際のシナリオを扱います。コメント、アンカー、タグ、フォーマットをすべて保持しながら、YAML ファイルの解析、検査、編集、検証、書き戻しの方法を学びます。

## セットアップ

```bash title="PyPI からインストール"
pip install pyrs-yaml
```

## 1. 設定ファイル

コメント、アンカー、マージキー、ブロックとフローが混在したフォーマットを含む YAML 設定ファイルから始めます：

```yaml title="config.yaml"
# Application configuration (v2.0)
app:
  name: my-service
  version: 2.0

# Default database settings
database: &default-db
  host: localhost
  port: 5432
  name: mydb

# Environment-specific overrides
environments:
  staging:
    <<: *default-db
    host: staging.example.com
    debug: true

  production:
    <<: *default-db
    host: prod.example.com
    port: 5432
    debug: false

# Feature flags
features:
  - name: login
    enabled: true
  - name: export
    enabled: true
  - name: reporting
    enabled: false

# Custom scalar example
threshold: 0x1F  # hex value (should be parsed as int)
```

## 2. ファイルをパースする

```python title="ファイルをパース"
import pyrs_yaml

doc = pyrs_yaml.parse_file("config.yaml")
print(f"Parsed: {doc.get('app.name')} v{doc.get('app.version')}")
# Parsed: my-service v2.0
```

**重要なポイント**：コメント、アンカー、タグ、フォーマットはすべてメモリ上に保持されます。ドキュメントは `YamlDocument` オブジェクトであり、生の Python dict ではありません。

## 3. 値を検査する

パス API（JSONPath 風）または Node API（ツリー構造）を使用します：

```python title="値を検査"
# Path API — simple and direct
db_host = doc.get("database.host")
print(f"Database host: {db_host}")

# Node API — access metadata and formatting
db_node = doc.node().find("$.database")
print(f"Database is flow style: {db_node.flow_style}")  # False (block)
print(f"Database anchor: {db_node.anchor}")  # "default-db"
```

## 4. メタデータを保持した値の編集

値を編集すると、そのコメント、アンカー、タグ、クォートスタイルが保持されます。編集は AST 上で直接行われ、文字列操作はありません：

```python title="値をその場で編集"
# Change the production port
doc.set("$.environments.production.port", 5444)

# Change the app name while keeping its comment
doc.set("$.app.name", "my-service-v2")

# Add a comment to document a change
prod_node = doc.node().find("$.environments.production")
prod_node.set_comment("overridden for v2 rollout")
```

## 5. メタデータを操作する

pyrs-yaml は値の編集だけではありません。YAML メタデータ自体の読み書きもできます：

```python title="メタデータの読み書き"
# Read existing metadata
debug_node = doc.node().find("$.environments.staging.debug")
print(f"Debug comment: {debug_node.comment}")  # None

# Add a tag to document a custom type
import_node = doc.node().find("$.threshold")
import_node.set_tag("!!int")
print(f"Threshold tag: {import_node.tag}")  # "!!int"

# Add an anchor for later reference
prod_db = doc.node().find("$.environments.production")
prod_db.set_anchor("prod-db")
```

## 6. フォーマットを制御する

スカラーのクォート、ブロック/フロー形式、チョンピング指示子を切り替えます：

```python title="フォーマットを制御"
# Switch the threshold to single-quoted for clarity
doc.node().find("$.threshold").set_scalar_style("single_quoted")

# Switch the staging environment to compact flow style
staging = doc.node().find("$.environments.staging")
staging.set_flow_style(True)
```

## 7. ワイルドカードで一括編集

`set_many` を使用して、一致するすべてのパスに変更を適用します。トグル操作に便利です：

```python title="ワイルドカードで一括編集"
# Disable ALL debug flags across every environment
doc.set_many(
    {
        "$.environments[*].debug": False,
    }
)

# Disable all features at once
doc.set_many(
    {
        "$.features[*].enabled": False,
    }
)
```

## 8. キーのソート

読みやすくするために、トップレベルのキーと環境キーをソートします：

```python title="キーのソート"
doc.sort_keys()  # sort the root mapping
doc.sort_keys("$.environments")  # sort the environments
```

## 9. スキーマによる検証

構造ルールを持つスキーマを定義し、設定を検証します：

```python title="スキーマによる検証"
schema = """\
name: app-config
extends: core
validate:
  - path: $.app.name
    type: str
    required: true
  - path: $.environments.*.debug
    type: bool
  - path: $.threshold
    type: int
"""

# Validate — raises YamlValidateError on failure
pyrs_yaml.validate_against_schema(doc.to_yaml(), schema)
print("Configuration is valid!")
```

## 10. サブツリーのディープコピー

サブツリーを（ドキュメントから切り離された）独立した Python 値としてコピーします：

```python title="サブツリーのディープコピー"
# Copy the staging configuration for reuse
staging_config = doc.node().find("$.environments.staging").copy()
print(staging_config)  # {'host': 'staging.example.com', 'debug': False, ...}
```

## 11. サブツリーの移動

同じドキュメント内でサブツリーを移動します：

```python title="サブツリーの移動"
# Move the reporting feature to a new section
doc.node().find("$.features[2]").move("$.deprecated-features")
```

## 12. ファイルへの書き戻し

最後に、編集したドキュメントを YAML にシリアライズします：

```python title="ファイルへの書き戻し"
output = doc.to_yaml()
with open("config-updated.yaml", "w", encoding="utf-8") as f:
    f.write(output)
```

出力は**すべて**を保持します — コメント、アンカー、マージキー、フォーマット、そして行った編集のすべて：

```yaml title="config-updated.yaml"
# Application configuration (v2.0)
app:
  name: my-service-v2
  version: 2.0

# Default database settings
database: &default-db
  host: localhost
  port: 5432
  name: mydb

# Environment-specific overrides
environments:
  # overridden for v2 rollout
  production: &prod-db
    <<: *default-db
    host: prod.example.com
    port: 5444
    debug: false

  staging:
    <<: *default-db
    debug: false
    host: staging.example.com
```

## まとめ

このチュートリアルでは、次のことを行いました：

- :material-file-code: メタデータを完全に保持して YAML ファイルを**パース**
- :material-magnify: パス API と Node API で値を**検査**
- :material-pencil: 値、コメント、アンカー、タグ、フォーマットを**編集**
- :material-format-list-bulleted: `set_many` でワイルドカードによる**一括編集**
- :material-sort: 読みやすくするためにキーを**ソート**
- :material-check-decagram: スキーマに対して**検証**
- :material-content-copy: サブツリーを**コピー**および**移動**
- :material-sync: すべてを保持したまま YAML に**シリアライズ**

### 次のステップ

- :material-rocket-launch: [クイックスタート](../quick-start.md) — 数分で始める
- :material-pencil: [インプレース編集ガイド](../guides/editing.md) — 編集 API の完全リファレンス
- :material-check-decagram: [カスタムスキーマガイド](../guides/custom-schema.md) — 独自スキーマの定義
- :material-book-open-page-variant: [API リファレンス](../api/reference.md) — 完全な API ドキュメント
