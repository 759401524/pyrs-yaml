---
title: テスト実行
description: pyrs-yaml の Rust ユニットテストと Python 統合テストの実行方法を説明します。
tags:
  - docs
status: new
---

pyrs-yaml は Rust ユニットテストと Python 統合テストの両方を持っています。

## Rust テスト

```bash
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

```bash
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

```bash
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

```rust
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

```python
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

スイートの合格率は **95%** でゲートされています（`test_compliance_report` 参照）。一部のケースは意図的に追跡していません。それらを拒否することは仕様的に正しく、リファレンスパーサー（特に PyYAML/libyaml）と一致するためです：

| ID | 入力 | 逸脱として許容する理由 |
|:---|:------|:----------------------------|
| `ZYU8` | `%YAML 1.1 1.2` | バージョンディレクティブに続く内容は、YAML 1.2 文法（`ns-yaml-version ::= ns-dec-digit+ '.' ns-dec-digit+`）に従うと**無効**です。PyYAML もこれを拒否します。スイート自身の注釈でも、これらのディレクティブバリアントは「まったく有用に有効ではない」と述べており、サポートは推奨されていません。 |

その他のスイートケースはすべて合格しています（現在 405/406 = 99.75%）。
