---

title: 開発環境のセットアップ
lang: ja

## 開発環境のセットアップ

pyyaml-rs に貢献するための環境をセットアップします。

### 前提条件

- **Python** ≥ 3.8 (CPython)
- **Rust** ≥ 1.70 ([rustup](https://rustup.rs/) 経由)
- **Git**
- **uv**（推奨）または **pip**
- **NumPy** — NumPy シリアライゼーションテストスイートの実行に必要 (`uv run --frozen pytest tests/test_numpy.py`)

### クローンとインストール

```bash
git clone https://github.com/759401524/pyyaml-rs.git
cd pyyaml-rs

# uv を使用（推奨）
uv sync

# または pip を使用
pip install maturin
uv run --frozen maturin develop --release
```

### インストールの確認

```bash
# Rust テストの実行
cargo test

# Python テストの実行
uv run --frozen pytest tests/

# ベンチマークの実行
cargo bench
```

### プロジェクト構造

```text
pyyaml-rs/
├── src/
│   ├── lib.rs              # PyO3 モジュール定義
│   ├── ast.rs              # カスタム AST (CustomNode)
│   ├── parser/
│   │   ├── mod.rs          # saphyr-parser 統合
│   │   └── yaml/           # YAML 特定パース
│   │       ├── comment.rs  # コメント抽出
│   │       ├── merge.rs    # マージキー解決
│   │       ├── scalar.rs   # スカラーパース
│   │       └── types.rs    # YAML 1.2 型解決
│   └── serializer.rs       # YAML シリアライゼーション
├── python/pyyaml_rs/
│   ├── __init__.py         # Python パッケージ初期化
│   ├── pyyaml_rs.pyi       # 型スタブ
│   └── py.typed            # PEP 561 マーカー
├── tests/                  # Python テストスイート
├── benches/                # Rust ベンチマーク
└── docs/                   # ドキュメント (mkdocs)
```

### ビルドコマンド

```bash
# Python 拡張のビルド
uv run --frozen maturin develop --release

# wheel のビルド
maturin build --release --out dist

# デバッグ情報付きでビルド
cargo build
```

### 開発ワークフロー

1. **まずテストを書く** (TDD)
2. `src/` で**変更を実装**
3. **`cargo test` を実行**して Rust テストを確認
4. **`uv run --frozen pytest tests/` を実行**して Python テストを確認
5. **`cargo clippy -- -D warnings` を実行**してコード品質を確認
6. **`cargo fmt` を実行**してコードをフォーマット
