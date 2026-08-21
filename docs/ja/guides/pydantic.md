---
title: Pydantic 統合
description: YAML を Pydantic v2 モデルにパースし、モデルを YAML にシリアライズし、pyrs-yaml をパーサーとして BaseSettings を読み込みます。
tags:
  - docs
status: new
---

## Pydantic 統合

pyrs-yaml は [Pydantic](https://docs.pydantic.dev/) v2 と
[pydantic-settings](https://docs.pydantic.dev/latest/concepts/pydantic_settings/) と
連携し、 YAML を検証済みモデルに変換し、その逆もできます。両方ともオプションの依存関係です：

- モデルのパース・シリアライズには `pip install pydantic`（または `pip install 'pyrs-yaml[pydantic]'`）
- `BaseSettings` の読み込みには `pip install 'pyrs-yaml[settings]'`（`pydantic-settings` をインストールします）

### YAML をモデルにパース

`parse_as()` は YAML をパースし、Pydantic の `BaseModel` サブクラスに対して検証し、モデルインスタンスを返します。`**yaml_kwargs` は `YAML()` コンストラクタに渡されます（例：`resolve_merges`）。

```python title="Parse YAML into a Pydantic model"
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


user = pyrs_yaml.parse_as(User, "name: Alice\nage: 30")
print(user.name)  # Alice
print(user.age)  # 30
```

`parse_as()` は以下を送出します：

- `ImportError` — pydantic がインストールされていない
- `TypeError` — `model` が `BaseModel` サブクラスではない
- `pydantic.ValidationError` — パースされたデータがモデルの検証に失敗した

### モデルを YAML にシリアライズ

`dump_pydantic()` は Pydantic モデルを YAML 文字列にシリアライズします。まず
`model_dump(mode="json")` を呼んで文字列型のフィールドを維持し（「10001」のような郵便番号が整数に強制変換されない）、その後 `safe_dump` に委ねます。

```python title="Serialize a Pydantic model to YAML"
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


yaml_str = pyrs_yaml.dump_pydantic(User(name="Alice", age=30))
print(yaml_str)
# name: Alice
# age: 30
```

`dump_pydantic()` は以下を送出します：

- `ImportError` — pydantic がインストールされていない
- `TypeError` — `model` が `BaseModel` インスタンスではない

### pydantic-settings で設定を読み込み

`PyrsYamlConfigSettingsSource` は `pydantic_settings.YamlConfigSettingsSource` の
同等の置き換えです。pyrs-yaml の YAML 1.2 パーサーで YAML 設定ファイルを読み込み、
 env 変数・dotenv・秘密鍵と並んで `BaseSettings` モデルに値を投入します。優先順序と挙動は
 同じです。

```python title="Load BaseSettings from a YAML file"
from pydantic_settings import BaseSettings, SettingsConfigDict
import pyrs_yaml


class Settings(BaseSettings):
    app_name: str

    model_config = SettingsConfigDict(yaml_file="config.yaml")

    @classmethod
    def settings_customise_sources(
        cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
    ):
        return (
            init_settings,
            env_settings,
            dotenv_settings,
            file_secret_settings,
            pyrs_yaml.PyrsYamlConfigSettingsSource(settings_cls),
        )
```

このソースは `YamlConfigSettingsSource` と同じオプションをサポートします：

- `yaml_file` — パスまたはパスのリスト（`SettingsConfigDict` で指定、または直接指定）
- `yaml_file_encoding` — ファイルエンコーディング
- `yaml_config_section` — ドット表記によるネストセクションのパス
- `deep_merge` — 複数ファイルを置き換えではなく深くマージ

!!! note "遅延インポート"
    `import pyrs_yaml` だけでは pydantic や pydantic-settings を必要としません。
    対応する依存関係がインストールされていない状態で `pyrs_yaml.parse_as`、
    `pyrs_yaml.dump_pydantic`、`pyrs_yaml.PyrsYamlConfigSettingsSource` にアクセスすると、
    インストールヒント付きの `ImportError` が送出されます。

### コメント付きラウンドトリップ

`parse_as()` は `safe_load` を基盤としているため、コメントやアンカーの保持は
モデルパスには含まれません — ラウンドトリップ編集が必要な場合は `parse()` と
`YamlDocument` を使用し、検証済みモデルが必要な場合にのみ `parse_as()` を使用します。

!!! tip "正しいパースパスを選択"
    設定の検証には `parse_as()` を、コメント・アンカー・書式がラウンドトリップで
    残る必要がある場合は `parse()` を使用します。

### 関連項目

- [YAML のパース](parsing.md) — 文字列・ファイル・複数ドキュメントのパース
- [シリアライズ](serialization.md) — YAML ドキュメントと Python オブジェクトの相互変換
- [設定管理](tutorial-config-management.md) — エンドツーエンドの_walkthrough
- [API リファレンス](../api/reference.md) — `parse_as`、`dump_pydantic`、`PyrsYamlConfigSettingsSource` の完全なシグネチャ
