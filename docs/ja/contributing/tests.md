---

title: テスト実行
lang: ja

## テスト実行

pyrs-yaml は Rust ユニットテストと Python 統合テストの両方を持っています。

### Rust テスト

```bash
# すべての Rust テストを実行
cargo test

# 特定のモジュールのテストを実行
cargo test ast
cargo test parser
cargo test serializer

# 出力を伴って実行
cargo test -- --nocapture

# 統合テストのみ実行
cargo test --test integration
```

#### テストカバレッジ

- **`src/ast.rs`** — ノード構築、メタデータ、等値性
- **`src/parser/`** — さまざまな YAML 構造のパース
- **`src/serializer.rs`** — シリアライゼーション往復保存
- **`src/integration/`** — YAML Test Suite 統合

### Python テスト

```bash
# すべての Python テストを実行
pytest tests/

# 詳細出力で実行
pytest tests/ -v

# 特定のテストファイルを実行
pytest tests/test_parse.py

# パターンに一致するテストを実行
pytest tests/ -k "comment"

# カバレッジ付きで実行
pytest tests/ --cov=pyrs_yaml --cov-report=term-missing

# ベンチマークを実行
pytest tests/ --codspeed
```

#### テストファイル

| ファイル | カバレッジ |
|---------|----------|
| `test_parse.py` | パース、データ型、特殊文字 |
| `test_serialize.py` | シリアライゼーション、往復保存 |
| `test_edge_cases.py` | エッジケース、エラーハンドリング |
| `test_errors.py` | カスタム例外タイプ、ファイル I/O |
| `test_features.py` | Markdown フロントメータ、from_dict/from_json |
| `test_json.py` | JSON ↔ YAML 変換 |
| `test_tabs.py` | タブ処理 |
| `test_yaml_suite.py` | YAML Test Suite 統合 |
| `test_performance.py` | パフォーマンス整合性チェック |
| **`test_numpy.py`** | **NumPy Ndarray シリアライゼーション (0 次元〜N 次元、すべての dtype)** |

### CI テスト

GitHub Actions はすべてのプッシュと PR で実行：

- **Rust**: `cargo test`、`cargo clippy -- -D warnings`、`cargo fmt --check`
- **Python**: 4 つの Python バージョン × 3 つの OS で `pytest tests/`
- **Maturin**: 各 Python バージョン用に wheel をビルド

### 新しいテストの追加

#### Rust テスト

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

### テストカテゴリ

- **ユニットテスト** — 個別の関数、小さな入力
- **統合テスト** — 完全なパース → シリアライズ往復保存
- **エッジケーステスト** — 特殊文字、空の入力、不正な YAML
- **パフォーマンステスト** — 整合性チェック（ベンチマークではない）
- **YAML Test Suite** — YAML 準拠の外部テストスイート
