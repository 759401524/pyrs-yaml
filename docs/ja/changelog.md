---

title: Changelog
lang: ja

## 変更履歴

All notable changes to this project will be documented in this file.

The format is based on [Keep a 変更履歴](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### [Unreleased]

### [0.11.2] - 2026-08-03

#### Added

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`: O(アンカー + チャンク) メモリの遅延イベントイテレータ

#### Performance

- **パース時にスプライス資格を計算しない** — O(ドキュメント) のレイアウトチェックは最初の編集時に `YamlDocument.splice_checked` 経由で遅延実行され、v0.11.0 の回帰を復元: parse_comments -59%、parse_anchors -42%、parse/roundtrip/edit -10~35% すべて v0.10.0 レベルに戻る
- **線形カーソルのレイアウトチェック** — 事前計算済み行オフセット上のノード単位バイナリサーチを置換（単調なソース順トラバーサル）

#### Changed

- `parse_with_options` は `CustomNode` を返す（旧 `(CustomNode, bool)`）; スプライス資格は現在 `YamlDocument` 内部にあり、オンデマンドで計算される

### [0.11.0] - 2026-08-02

#### Added

- **Surgical Serialization** — 全 AST ノードのバイトレベルソーススパン追跡；セグメントベーススプライス — 編集はタッチされた領域のみ再生成、 untouched テキストはバイトコピー
- プロパティテスト（proptest、新規開発依存）
- 10MB 編集-フラッシュベンチマーク

#### Changed

- `flush_source` がセグメントスプライスを使用；フロースタイル領域、非デフォルトレイアウト文書、マージキー、CRLF/BOM 文書、materialize 後（シングルバーストモデル）では全量シリアライズにフォールバック
- スプライス編集は `---`/`...`/ディレクティブマーカー行を未変更バイトとして保持（全量シリアライズは以前それらを削除 — 意図的な動作差）

### [0.10.0] - 2026-08-01

#### Added

- **インプレース編集** — フォーマットメタデータを失わずに解析済みドキュメントを編集：
    - パス API：`doc.set(path, value)`、`doc.insert(path, index, value)`、`doc.append(path, value)`、`doc.delete(path)`、`doc.rename(path, new_key)`、JSONPath スタイルのパス（`$.a.b[0]`）；ルート用糖衣構文 `doc["key"] = value` と `del doc["key"]`
    - ノード API：`doc.node()` / `doc.find(path)` は `Node` オブジェクトを返し、`set_value` / `append` / `insert` / `delete` / `rename` とツリー走査（`parent`、`children`、`walk`、`filter`）をサポート
    - 完全なメタデータ保持 — 置換されたスカラーはコメント/アンカー/タグ/クォートを保持；リネームされたキーは位置とコメントを保持；削除時もマッピングの順序は保持
    - アトミック編集 — 失敗した操作はドキュメント（リビジョンを含む）を変更しません
    - 遅延ソース再同期 — `source()` / `to_yaml()` / `reparse()` は編集成功後にのみ再シリアライズ
    - 陳腐化ノード検出 — ドキュメント編集後の `Node` アクセスは `YamlDocumentError` をスロー（`RuntimeWarning` 付き）
    - 新しい例外：`YamlEditError`、`YamlPathError`（en/zh-CN/ja-JP/ko-KR の i18n 対応）
    - エイリアス対応編集 — エイリアス自身のパスへの設定はその場で置換；エイリアス経由の編集は `YamlEditError` をスロー
- **編集ベンチマーク** — `benches/yaml_bench.rs` に divan ベンチマークを 6 つ追加（小〜大ドキュメントの set/insert/delete）
- **Python 3.13、3.14、3.15 サポート** — PyO3 `abi3-py38` wheel が Python 3.8-3.15 をカバー（GIL ビルド）；`abi3t` + `abi3t-py315` は free-threaded 安定 ABI を提供
- **Free-threaded CPython（GIL なし）サポート** — `#[pymodule(gil_used = false)]` がモジュールを free-threaded Python 向けにスレッドセーフと宣言；`Py_GIL_DISABLED` cfg フラグで numpy をゲート（rust-numpy は free-threaded 未対応 — free-threaded ビルドでは `--no-default-features` で numpy feature を無効化）
- **CI free-threaded ジョブ** — 新しい `test-freethreaded` ワークフロージョブが Python 3.14t でコンパイルとテストを検証
- **`pyo3-build-config` ビルド依存** — `build.rs` 経由で `#[cfg(Py_GIL_DISABLED)]`、`#[cfg(Py_3_15)]` などのコンパイラフラグを有効化
- **`numpy` をオプション化** — `numpy` feature の背後にゲート（デフォルト有効）；`Py_GIL_DISABLED` 下では自動的に除外

#### Changed

- CI Python マトリクスを拡張：ubuntu、windows、macos で 3.8-3.14
- 安定 ABI：`abi3-py39` → `abi3-py38`（より広い Python 3.8+ サポート）、`abi3t` + `abi3t-py315` を追加（free-threaded 安定 ABI）
- `pyproject.toml` の classifiers に 3.13、3.14、3.15 のエントリを追加
- `YamlDocument.source()` は `str` を返し、インプレース編集後に遅延再シリアライズするようになりました

#### Fixed

- ラウンドトリップのドキュメントを明確化：マージキー（`<<`）はデフォルトで解決され、`resolve_merges=False` の場合のみそのまま保持

### [0.3.0] - 2025-07-27

#### Added

- **NumPy Ndarray serialization** — `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()` now support `numpy.ndarray` of all dimensions (0-D through N-D)
    - Supported dtypes: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`
    - Multi-dimensional arrays serialize as nested YAML lists with correct indentation
    - Complex numbers serialize as `(re+imj)` string format
    - `0-D` scalar arrays reshape to 1-D and serialize as a single-item list
    - `PyUntypedArray` + `PyArrayDyn` via `numpy` Rust crate for zero-copy dtype dispatch
    - GIL released during slice iteration for maximum performance
- **`quoted_scalar()`** — new `CustomNode::quoted_scalar()` constructor for values requiring single-quoted YAML style
- **Type resolution for quoted scalars** — `resolve_yaml_type` now applied to `SingleQuoted`/`DoubleQuoted` scalars for correct round-trip of quoted negative numbers
- **Comprehensive NumPy test suite** — 42 tests covering all dtypes, dimensions (0-D through 4-D), negative numbers, infinity, NaN, empty arrays, and edge cases

#### Fixed

- **Negative number round-trip** — YAML 1.2 block sequences cannot contain plain scalars starting with `-`; negative numbers are now quoted during serialization and correctly parsed back as integers/floats
- **N-D array support** — replaced `PyArray1<T>` with `PyArrayDyn<T>` to support arrays of any dimension, not just 1-D
- **Correct nesting depth** — multi-dimensional arrays now produce exactly N levels of nesting (shape[1..] handles inner dimensions, root dimension wrapped by `plain_sequence`)

#### Changed

- Added `numpy` crate (v0.29) as a dependency for ndarray type dispatch

#### Added

- Flow collections (`{}`/`[]`) round-trip support with `flow_style` field on Mapping/Sequence AST nodes
- `parse()` accepts both `str` and `bytes` input
- `parse()` supports `resolve_merges` parameter to opt out of merge key expansion
- `parse_all_docs()` for multi-document parsing via saphyr events
- `to_yaml_with_options()` with `indent_size`, `explicit_start`, `explicit_end`, `sort_keys` parameters
- `get()` supports default value parameter
- `dump_file()` for writing YAML to files
- Criterion benchmarks in `benches/yaml_bench.rs` (parse/serialize/roundtrip)
- GitHub Actions CI with matrix testing (3 OS × 4 Python versions)
- Anchor name parsing expanded to full YAML 1.2 spec (dots, colons, hashes, quoted anchors)
- `__version__` attribute, `py.typed` PEP 561 marker
- Full `.pyi` type stubs with type annotations in `#[pyo3(signature)]`
- `experimental-inspect` feature enabled for `pyo3-introspection` compatibility
- i18n support with `set_language()`, `get_language()`, `list_languages()`, `detect_language()`, `negotiate_language()`
- Markdown frontmatter extraction: `read_markdown()`, `read_markdown_str()`
- JSON ↔ YAML conversion: `from_json()`, `from_dict()`

#### Fixed

- Alias resolution in `to_dict()` and `safe_load()` — aliases now resolve to referenced values instead of `None`
- `safe_loads()` no longer uses naive `split("---")` — uses saphyr's document events
- Mapping/Sequence tags no longer discarded during parsing
- `format_scalar_for_key()` now handles Literal/Folded block scalar styles
- Exception types now properly exported from inline `#[pymodule]` via `#[pymodule_export]`

#### Changed

- Upgraded PyO3 from 0.21 to 0.29
- Replaced 15+ boilerplate `CustomNode` constructions with `plain_scalar()`/`plain_mapping()`/`plain_sequence()`/`plain_null()` constructors
- Serializer extracted `write_anchor_tag()` and `write_inline_comment()` helpers
- Parser extracted `detect_flow_style()` helper
- Removed dead code: `ParseOptions`, `find_inline_comment`, `find_standalone_comment_before`, `format_yaml_type` (test-only)
- Consolidated 6 duplicate test files, moved 9 diagnostic scripts to `scripts/`
- Improved error messages with key/index/type context
- `#[pymodule]` refactored to inline module for `pyo3-introspection` compatibility
- All `#[pyo3(signature)]` attributes now include Python type annotations

### [0.1.0] - 2025-07-25

#### Added

- Initial release with YAML 1.2 compliance via saphyr-parser
- Custom AST with full metadata (comments, anchors, tags, chomping, scalar styles)
- Round-trip preservation of comments, anchors, tags, and formatting
- PyYAML-compatible API (`safe_load`/`safe_dump`)
- `from_dict`/`from_json` conversion functions
- `read_markdown`/`read_markdown_str` for YAML frontmatter extraction
- Block scalars (`|`/`>`) with chomping indicators (`|-`/`|+`/`>-`/`>+`)
- Escape sequences (`\n`, `\t`, `\uXXXX`, `\xXX`)
- YAML 1.2 type resolution (null, bool, int, float, infinity, NaN)
- Merge key resolution (`<<: *alias`)
- Complex keys (sequence/mapping as key)
