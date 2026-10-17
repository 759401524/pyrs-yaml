---
title: テスト実行
description: pyrs-yaml の Rust ユニットテストと Python 統合テストの実行方法を説明します。
tags:
  - docs
status: new
---

pyrs-yaml は Rust ユニットテストと Python 統合テストの両方を持っています。

## Rust テスト

```bash title="Rust テストを実行"
# すべての Rust テストを nextest で実行（推奨）
cargo nextest run --all

# すべての Rust テストを cargo test で実行
cargo test --all

# Pure Rust コアテストを実行（Python ランタイム不要）
cargo test --all --no-default-features

# 出力を伴って実行
cargo test --all -- --nocapture
```

### テストカバレッジ

- **`crates/pyrs-yaml-core/src/ast.rs`** — ノード構築、メタデータ、等値性
- **`crates/pyrs-yaml-core/src/parser/`** — さまざまな YAML 構造のパース
- **`crates/pyrs-yaml-core/src/serializer.rs`** — シリアライゼーション往復保存
- **`crates/pyrs-yaml-core/src/editing/`** — 編集プリミティブ（navigate, region, dirty, metadata）
- **`crates/pyrs-yaml-core/src/integration/`** — YAML Test Suite 統合
- **`crates/pyrs-yaml/src/fidelity.rs`** — プロパティベースファズテスト

## Python テスト

```bash title="Python テストを実行"
# すべての Python テストを実行
uv run pytest tests/ -v

# 特定のテストファイルを実行
uv run pytest tests/test_edit.py -v

# 特定のテストクラスを実行
uv run pytest tests/test_node_api.py::TestDocWalk -v

# カバレッジ付きで実行
uv run pytest tests/ -v --cov=pyrs_yaml

# コンプライアンススイートを実行
uv run pytest tests/test_yaml_suite.py -v

# ベンチマークを実行
uv run pytest tests/ --codspeed
```

## Maturin ビルド

```bash title="ビルドしてインストール"
# ビルドしてインストール（モノレポの manifest-path を使用）
uv run maturin develop --release

# .pyi ファイルのスタブを生成
uv run maturin build --release --generate-stubs
```

### テストファイル

| ファイル | カバレッジ |
|---------|----------|
| `test_parse.py` | パース、データ型、特殊文字 |
| `test_serialize.py` | シリアライゼーション、往復保存 |
| `test_edge_cases.py` | エッジケース、エラーハンドリング |
| `test_errors.py` | カスタム例外タイプ、ファイル I/O |
| `test_features.py` | Markdown Front Matter、from_dict/from_json |
| `test_json.py` | JSON ↔ YAML 変換 |
| `test_tabs.py` | タブ処理 |
| `test_yaml_suite.py` | YAML Test Suite 統合 |
| `test_performance.py` | パフォーマンス整合性チェック |
| **`test_numpy.py`** | **NumPy Ndarray シリアライゼーション (0 次元〜N 次元、すべての dtype)** |

## CI テスト

GitHub Actions はすべてのプッシュと PR で実行：

- **Rust**: `cargo nextest run --all`、`cargo clippy --all -- -D warnings`、`cargo fmt --check`
- **Python**: 4 つの Python バージョン × 3 つの OS で `uv run pytest tests/`
- **Maturin**: 各 Python バージョン用に wheel をビルド（`crates/pyrs-yaml/Cargo.toml` 経由）

## 新しいテストの追加

### Rust テスト

```rust title="Rust テストテンプレート"
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_feature() {
        // テストをここに書く
    }
}
```

#### Python テスト

```python title="Python テストテンプレート"
import pyrs_yaml
import pytest


class TestNewFeature:
    def test_basic(self):
        result = pyrs_yaml.parse("key: value")
        assert result.get("key") == "value"

    def test_edge_case(self):
        # エッジケーステスト
        pass
```

## テストカテゴリ

- **ユニットテスト** — 個別の関数、小さな入力
- **統合テスト** — 完全なパース → シリアライズ往復保存
- **エッジケーステスト** — 特殊文字、空の入力、不正な YAML
- **パフォーマンステスト** — 整合性チェック（ベンチマークではない）
- **YAML Test Suite** — YAML 準拠の外部テストスイート

## YAML Test Suite 既知の逸脱

スイートは `tests/test_yaml_suite.py` の三つの「名前指定」ゲートで読みます。パーセンテージでは
ありません。このプロジェクトが意図して一度だけ置く拒否を除き、すべての有効ケースは解析できなければ
ならず、無効ケースはひとつも通せず、JSON 比較は成因つきで記録した五つのケースの台帳以外はすべて
一致しなければなりません。`test_compliance_report` はレポート自身の数値を 99.5% で床張りします。
一部のケースは意図的に追跡していません。それらを拒否することは仕様的に正しく、リファレンス
パーサー（特に PyYAML/libyaml）と一致するためです：

ケースはゲートと同じ名前で指します。スイートの id は*ファイル*であり、351 のファイルのうち 18 は
複数のケースを含むため、名前にファイル内での位置も添えます。

| ID | 入力 | 逸脱として許容する理由 |
|:---|:------|:----------------------------|
| `ZYU8#3` | `%YAML 1.1 1.2` | バージョンディレクティブに続く内容は、YAML 1.2 文法（`ns-yaml-version ::= ns-dec-digit+ '.' ns-dec-digit+`）に従うと**無効**です。PyYAML もこれを拒否します。スイート自身の注釈でも、これらのディレクティブバリアントは「まったく有用に有効ではない」と述べており、サポートは推奨されていません。 |

その他のスイートケースはすべて合格しています（現在 405/406 = 99.75%）。

JSON 台帳は未修正の欠陥二つと表記の違い一つです。項目が消えるのはエンジンが規則を覚えたときだけで、
集合が増えて消えることはありません。欠陥：内容行がすべて空の keep チョミング・ブロックは改行をひとつ
多く残す（`JEF9#1`、`K858#1`）。表記の違い：スイートは `!!binary` ノードをスカラー自身のソース文として
書き、比較は読み込んだバイトを正規 base64 で書きます（`565N#1`）——両表記は同じ 185 バイトを構成し、
それを `test_binary_spellings_construct_the_same_bytes` が固定します。

二件の項目がすでにこの消え方をしました。別名は同名アンカーの*最後*の定義に結ばれていて直前の定義には
結ばないため、`3GZX` は `Bar` と読んでいました——スイート自身のイベントツリーも ruamel も `Foo` です。
PyYAML は重複したアンカー名を即座に拒否する（`found duplicate anchor`）ので、コーパスとそれを受け取る
ライブラリが権限であり、その違いは `test_a_reused_anchor_name_is_read_and_not_refused` が固定します。
もう一件は、むき出しの `!` がタグなしのスカラーと同じように schema で解決されていて `! 12` が整数に
なっていた件です。YAML 1.2 §6.8 はそのノードのタグを `!` のまま残し、§10.5.2 はアプリ側の解決規則を
`?` タグに限定するので、スカラーは自身のテキストになります——spec Example 6.28 もコーパスも `"12"` と
書きます。両方の Python ライブラリは暗黙的に解決しています（ruamel には未対応チケット #573 がある）
ので、その違いは両方の読みを添えて記録し、`tests/test_cross_format_tags.py` の一枚の表でローダーと
`to_json`、`to_jsonc`、`to_json5`、`to_toml` を同じ答えに固定します。
