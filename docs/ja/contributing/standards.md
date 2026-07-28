---

title: コーディング基準
lang: ja

## コーディング基準

pyyaml-rs に貢献する際は、以下の基準に従ってください。

### Rust

#### スタイル

- コミット前に `cargo fmt` を使用
- [Rust API ガイドライン](https://rust-lang.github.io/api-guidelines/) に従う
- `#[allow(unused_imports)]` は必要な場合のみ使用（テスト、フィーチャーフラグ）

#### エラーハンドリング

- ビジネスロジックで **`.unwrap()` や `.expect()` を絶対に使用しない**
- すべての Rust エラーを Python 例外に変換
- 失敗する可能性のある関数には `PyResult<T>` を使用
- 特定のエラーを特定の Python 例外タイプにマッピング

```rust
// OK
let content = std::fs::read_to_string(path)
    .map_err(|e| YamlParseError::new_err(format_i18n_error("file-read-error", ...)))?;

// NG
let content = std::fs::read_to_string(path).unwrap();
```

#### ドキュメント

- すべての公開関数には `///` ドキュメントコメントが必要
- `# Arguments`、`# Returns`、`# Errors`、`# Examples` セクションを含める
- ドキュメントコメントは英語で書く（Rust の慣例）
- 関数内部のドキュメントコメントは中国語でも可

```rust
/// YAML 文字列を CustomNode AST にパースする。
///
/// # Arguments
/// * `yaml` — YAML コンテンツ文字列
///
/// # Returns
/// パースされた AST ルートノード、失敗時は `Err(String)`
///
/// # Errors
/// `"YAML parse error: line N, col M: <msg>"` 形式の `Err(String)` を返す
///
/// # Examples
/// ```
/// let ast = pyyaml_rs::parser::parse("key: value").unwrap();
/// ```
pub fn parse(yaml: &str) -> Result<CustomNode, String> {
```

#### PyO3 シグネチャ注釈

すべての `#[pyfunction]` と `#[pymethods]` は `#[pyo3(signature = "...")]` で型を注釈する必要があります：

```rust
#[pyo3(signature = (yaml: "str", resolve_merges: "bool" = true, schema: "str" = "core") -> "YamlDocument")]
fn parse(...) -> YamlDocument { ... }
```

#### GIL 管理

- 負荷の高い計算中は `py.detach()` または `py.allow_threads()` を使用して GIL を解放
- ファイル I/O やパース中に GIL を保持しない

```rust
// OK
let ast = py.detach(|| {
    parser::parse_with_options(&yaml_str, resolve_merges)
        .map_err(|e| YamlParseError::new_err(...))?
})?;

// NG — パース中に GIL を保持
let ast = parser::parse_with_options(&yaml_str, resolve_merges)?;
```

#### Clippy

`cargo clippy -- -D warnings` を実行 — すべての警告をエラーとして扱う。

### Python

#### スタイル

- [PEP 8](https://peps.python.org/pep-0008/) に従う
- すべての場所で型ヒントを使用
- ドキュメント文字列は Google スタイル
- コードチェック設定は `ruff.toml` にあり（`ruff check` を実行）

```python
def parse(yaml: str, resolve_merges: bool = True) -> YamlDocument:
    """YAML 文字列を YamlDocument にパースする。

    Args:
        yaml: YAML コンテンツを含む文字列
        resolve_merges: マージキーを解決するかどうか (デフォルト: True)

    Returns:
        パースされた YAML を含む YamlDocument

    Raises:
        YamlParseError: YAML が無効な場合
    """
```

#### テスト

- コードの前にテストを書く（TDD）
- 必要に応じて `uv run --frozen pytest` とフィクスチャを使用
- エッジケースをテスト：空の入力、特殊文字、大きなドキュメント
- 往復保存アサーションを含める
- Pytest 設定は `pytest.ini` にあり（asyncio_mode = auto、カスタムマーカー）

### Git

- コミットメッセージは命令形で："Add feature X"（"Added feature X" ではない）
- 1 コミットに 1 つの論理的な変更
- コミット前に `cargo test` と `uv run --frozen pytest tests/` を実行

### ドキュメント

- 動作を変更した場合はドキュメントを更新
- コピー＆ペーストして実行できるコードサンプルを使用
- サンプルは簡潔だが完全に保つ
