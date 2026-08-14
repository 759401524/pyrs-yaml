---
title: 変更履歴
description: pyrs-yaml プロジェクトのすべての注目すべき変更を文書化します。
tags:
  - docs
status: new
---

## 変更履歴

このプロジェクトのすべての注目すべき変更は、本ファイルに文書されます。

本書式は [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に基づき、
このプロジェクトは [Semantic Versioning](https://semver.org/ja/spec/v2.0.0.html) に準拠しています。

### [Unreleased]

### [v0.14.0] — 2026-08-14

#### 追加

- **YAML Schema Language** — 正規表現パターンを YAML 型にマッピングする
  カスタムスキーマを定義可能。`register_schema()` で登録。
- **インライン dict スキーマ** — `schema` パラメータに `dict` を直接渡せる。
- **Community Plugins** — `CustomType` ベースクラスによる
  カスタムノード型の登録。`register_type()` で登録。
- **組み込みプラグイン** — `!timestamp`（datetime）と `!set` がデフォルトで登録済み。

#### 変更

- **スキーマ解決がプラグイン可能に** — `SchemaResolver` トレイト +
  `Schema` 列挙型 + グローバル `SchemaRegistry`。組み込みスキーマは
  ゼロコストディスパッチを維持。
- **`node_to_pyobject` と `direct_dump` が `CustomType` をチェック** —
   タグ付きスカラは `from_yaml()` で変換、Python オブジェクトは
  `to_yaml()` でシリアライズ。

#### 修正

- **クォート付きスカラーは常に文字列として読み込まれる** — 暗黙の型解決はプレーンスカラーのみに適用（YAML 1.2）。`safe_load('"true"')` は文字列 `"true"`（`True` ではない）を返す。シリアライザはドキュメント（`to_yaml`）経路でも負数を正しく往復させます。
- **一重/二重引用符のみのキーが往復保存される** — 単一の `'` または `"` であるマッピングキーは引用スカラーとして出力され、解析不能な YAML になりません。
- **空コレクションは `{}`/`[]` を出力** — 空のマッピング/シーケンスのダンプが、再解析で `None` になる空ドキュメントを生成しなくなります。

#### 変更

- **`get()` はリテラルキーのみ** — `YamlDocument.get()` は `.` や `[` を含むキーを JSONPath と推定しなくなり、常にトップレベルのマッピングキーとして扱います（`__getitem__`/`__setitem__` と一貫）。パスアクセスは `find()`/`node()` を利用してください。

### [v0.13.0] — 2026-08-10

#### 変更

- **Rust MSRV を 1.96 に引き上げ、edition を 2024 に変更** — 両 crate は
  `rust-version = "1.96"` および `edition = "2024"` を宣言します。CI は
  `build`/`test-freethreaded` ジョブを Rust 1.96 に固定し、決定論的な wheel
  ビルドを実現します。また、`msrv-check` ジョブを追加し、MSRV で
  `cargo check`/`cargo test` を実行して静かな MSRV ドリフトを防ぎます
  （`rust-lint` ジョブは `stable` のまま）。バージョンの床は
  PyO3 0.29 自身の基線（rustc 1.83）よりも上に設定され、std API の先行対応
  （例: `assert_matches!`、1.96 で安定化）を目的としています。
  `TAG_REGISTRY`（タグハンドラ管理）が `std::sync::LazyLock` にリファクタされ、
  `Mutex<Option<...>>` の間接レイヤーが除去されました。

#### パフォーマンス

- **`safe_dump` / `from_dict` / `dump_file` / `dump_iterable`: direct writer**
  — 中間 `CustomNode` AST を介さず Python→YAML シリアライズ。
  単一パス `direct_dump` が従来の 2 パス `pyobject_to_node` + `to_yaml` を置換。
  `safe_dump` で 7 倍高速化（28ns→4ns）、`from_dict` で 6 倍高速化（35ns→6ns）。(#60)
- **`safe_load` / `safe_loads` / `to_dict`: fast-path skip anchor tracking**
  — 入力に `&` 文字がない場合、`collect_anchors` + アンカー解決を省略し、
  より単純な `node_to_pyobject_simple` パスを使用。(#59)
- **`resolve_core_type`: first-byte dispatch whitelist** — 数値/ブールでない
  先頭バイトは即座に `Str` を返すようになり、一般的なケースにおけるスキーマ
  解決のオーバーヘッドを回避。(#59)
- **granit-parser への移行** — saphyr-parser を granit-parser 1.0.1 に置換し、
  ネイティブな `Event::Comment` 出力により全文 `scan_yaml()` プリスキャンを
  廃止。parse_small -18%、parse_large -21%、roundtrip_large -18%。

#### 修正

- **`float_to_yaml_string` の round-trip 修正** — Rust の Display が小数部を
  落とした場合に `.0` を付加（`42` → `42.0`）し、float が int にならずに
  round-trip するように。
- **`count_nodes` 事前割り当ての巻き戻し** — 全 AST 走査のコストが回避できた
  realloc を上回ったため（serialize_10mb は約 14% 低下）、バッファ拡張は
  Vec の再割り当てに委ねる。

#### 追加

- **`max_depth` をストリーム & frontmatter API に追加** — `parse_stream(yaml, on_event, max_depth)`、
  `read_markdown(path, schema, max_depth)`、`read_markdown_str(content, schema, max_depth)`
  が `max_depth` を受け付ける（デフォルト 1000）。ストリーム解析はコアの
  `parse_stream_with_options` によりネスト深さ制限を強制するようになった
  （従来のストリームイベントには深さ制限がなかった）。
- **Pydantic 統合** — `dump_pydantic()` は Pydantic モデルを YAML 文字列に
  シリアライズ（`model_dump(mode='json')` + `safe_dump`）；`parse_as()`
  は YAML 文字列を Pydantic モデルインスタンスにパース。両方とも遅延インポート、
  pydantic へのハード依存なし。(#61)

#### 内部

- **`py/mod.rs` の分割** — 巨大な 1786 行のモジュールを
  `document.rs`（YamlDocument）、`yaml_instance.rs`（YAML クラス）、
  `functions.rs`（モジュールレベル関数）、`stream_iterator.rs`、
  `walk_helpers.rs` に分割。`mod.rs` は 128 行に削減。(#61)
- **`needs_quotes()` ガード + `double_quoted_scalar()` コンストラクタ** —
  `'true'` / `'42'` / `'null'` のような文字列は、コアスキーマで再パース時に
  誤読されないようダブルクォートのスカラーとして出力
  （`pyobject_to_node` + `json_value_to_node`）。
- **CodSpeed ベンチマークを `codspeed-divan-compat` に統一** —
  `exclude-allocations` でアロケータノイズを除去。クロスライブラリの
  ベンチマークを `tests/test_benchmark_crosslib.py` に統合し、共通の
  `tests/data/yaml_samples.py` フィクスチャとストリーミングのカバレッジを追加。

### [v0.12.1] — 2026-08-06

#### 追加

- **`set(create_missing=True)`** — 編集パス上の欠落中間マッピングキーがネストしたマッピングとして作成されます
  （例: `a: 1` に対して `a.b.c` を設定すると `b` と `c` が作成されます）；
  未解決のインデックスセグメントは依然としてエラーとなり、
  パス上のスカラー中間ノードも依然として例外を発生させます。
- **`doc.walk()` / `doc.scalars()`** — Rust 実装の深さ優先 AST 走査で、
  ノードごとの `to_dict()` 解決を回避した `Node` オブジェクトを返します。
  `walk()` は全ノードを返します；`scalars()` はスカラー/null ノードのみを返します。
- **Rust コアモジュールテスト** — `editing::navigate`（key_eq, navigate, navigate_mut,
  normalize_index, mapping_key_index）、`editing::region`（行ヘルパー、node_is_flow、
  extend_delete_over_comments, nav_err）、`editing::dirty`（DirtyKind/DirtyUnit コンストラクタ）、
  `editing::metadata`（with_metadata_from, needs_quoting）をカバーする
  39 個の新規テスト。
- **Python doc.walk() エッジケーステスト** — 空ドキュメント、null 値、
  深ネスト、フローコレクション、混合型のカバーするための 9 個の新規テスト。

#### 変更

- **モノレポワークスペース** — ソースコードを `crates/pyrs-yaml-core/`（純粋 Rust、PyO3 なし）
  と `crates/pyrs-yaml/`（PyO3 バインディング）に分割。ルート
  `Cargo.toml` はワークスペースになりました。旧 `src/` ディレクトリと
  `build.rs` は削除されました。
- **pyproject.toml** — `tool.maturin.manifest-path` を
  `crates/pyrs-yaml/Cargo.toml` に追加。
- **パースホットパス** — 単一パスコメント/アンカー抽出、遅延
  重複キー検出、`shift_insert` マージプリペンド、および単一ドキュメント
  パース用の `DocumentEnd` ディープクローンのスキップにより、大規模ドキュメントの
  パースコストを約 19% 削減（CodSpeed: parse[large] +13.9%、parse[medium] +16.6%、
  roundtrip[large] +12.2%）。
- **`Arc<str>` スカラーストレージ** — `CustomNode::Scalar` とコメント/イベント
  テキストは `Arc<str>` を介して割り当てを共有；AST ノードが 8 バイト縮小し、
  クローンがディープコピーの代わりに参照カウントインクリメントに。

#### 修正

- **`set(create_missing=True)` ネストチェーン構築** — 作成されたマッピング
  チェーンは最初のセグメントをネストキーレベルとして重複しなくなりました。
- **`set(create_missing=True)` 資格チェック** — 新たに作成されたキーは
  値の書き込みに対して資格を持つようになりました（資格チェックは合成ペア
  挿入後に実行されなくなりました）。
- **単純マッピングキー前のスタンドアロンコメント** — ラウンドトリップが
  単純キーノードに付随するスタンドアロンコメントを以前は削除していましたが、
  現在は保持されます（回帰テスト 2 件）。

### [0.11.7] — 2026-08-04

#### 変更

- **stub-build-check から release-guard に置換** — v0.10.0 の
  `--generate-stubs` 失敗モードを再現するために意図的に失敗する常時失敗の
  コンテナビルド（`validate.yml`）を、リポジトリが正しい場合に**合格する**
  3 つの静的アサーションに置換：`grep` で `publish.yml` が
  `--generate-stubs` に対してガードされ、`git ls-files` がコミットされた
  `.pyi` が追跡されていることを確認し、`test -f` が `py.typed` の存在を
  確認します。ジョブは正しい状態で緑の CI を返し、回帰時のみ赤になります。

#### 追加

- **Numpy free-threaded 追跡** — ROADMAP.md が `rust-numpy` の free-threaded
  サポート状況（PyO3/rust-numpy#476）を追跡するようになり、Rust バインディングが
  成熟した際に cp314t wheel での ndarray シリアライズを再有効化する依存関係として
  管理されます。

### [0.11.6] — 2026-08-04

#### 変更

- **Free-threaded（cp314t）wheel が numpy なしに** —
  `--no-default-features` でビルドされるため、rust-numpy は完全に除外されます
  （バイナリが小さく、ランタイムプローブなし）。`numpy.ndarray` に対する
  `safe_dump` は free-threaded ビルドで `YamlTypeError` を発生させます；
  GIL ビルド（Python 3.8-3.15）は完全な ndarray シリアライズを保持します。

#### 追加

- **Free-threaded CI 検証** — `test-freethreaded` ジョブが
  `--no-default-features` でビルドとテストを行うようになり、出荷される
  free-threaded wheel 構成と一致します。
- **インストールドキュメント** — `docs/{en,zh,ja,ko}` が free-threaded
  wheel が numpy なしであることを明記（cp314t での ndarray シリアライズは利用不可）。

### [0.11.5] — 2026-08-04

#### 変更

- **パーサー堅牢性項目 3/4/5 がフェーズ 0 厳格監査でクローズ** — 70 プローブ
  コーパス（インデント、ブロックマッピングキー、フローコンテキスト）を PyYAML
  オーラクルと比較した結果、修正可能な「受け入れられたが不正なケース」は
  **なかった**（64/70 が一致；6 つの相違は PyYAML が例外である意図的な
  YAML 1.2 / yaml-test-suite 要件であり、1 つは意図的な重複キー厳格性）。
  準拠率は **99.75%（405/406）** で維持。詳細は `ROADMAP.md` §v0.11.5 および
  `tests/test_strictness_audit.py` に記載。

#### 追加

- `tests/test_strictness_audit.py` — 70 プローブの厳格性回帰コーパスは現在の
  拒否/受容動作（両方向）を固定し、将来のパーサー変更が厳格性を静かに後退させたり
  過剰拒否したりできないようにします。

### [0.11.4] — 2026-08-04

#### 修正

- 重複する null/空マッピングキーがエラーを発生させなくなりました（`: a\n: b`、`~: a\n~: b`）—
  yaml-test-suite 2JQS に一致；実際の重複キーは依然として `YamlDuplicateKeyError` を発生
- 準拠ハーネス：誤って拒否された不正 YAML がパスとしてカウントされるようになりました
  （準拠動作にもかかわらずレートを下げていました）
- 準拠ハーネス：`convert_special_chars` のタブデコードが正規表現に—
  `—`/`‖` + `»` の連続は 1 つのタブになり、タブエンコード済みスイートケースを修正

#### 変更

- YAML Test Suite 合格率ゲートを >75% から **≥95%** に引き上げ；現在のレート
  **99.75%**（405/406）
- 既知の逸脱を文書化：`ZYU8`（`%YAML 1.1 1.2`）は設計上拒否されます
  （YAML 1.2 文法に違反、PyYAML/libyaml に一致）

### [0.11.3] — 2026-08-03

#### 追加

- ストリーミング書き込み：`YAML.dump_stream(file_obj, iterable)` /
  `YAML.dump_file(path, iterable)` — ドキュメントレベルの一定メモリ、自動
  `---` セパレータ、`explicit_start`/`explicit_end` フラグ付き
- `YamlDocument` の `with` コンテキストマネージャー：スナップショット/ロールバック
  トランザクションスコーピング
- `compliance_report()`：公開 YAML Test Suite 合格率レポート（バージョン一貫）

#### 変更

- 編集バースト行オフセットキャッシュ：スパイスレイヤー内の内部 O(N+edit) 引き継ぎ
  （公開 API 変更なし）
- `compute_compliance` をテストから `pyrs_yaml.compliance` へ移動；バージョンが
  ハードコードされなくなりました

#### 修正

- Changelog ミラードリフトガード：prek フック + CI ジョブが root/ミラー
  `[Unreleased]` の同期を確認
- パブリッシュ stub 事前検証：CI がリリース前に v0.10.0 クラスの
  `--generate-stubs` コンテナ失敗を再現

### [0.11.2] — 2026-08-03

#### 追加

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`：
  O(アンカー + チャンク) メモリの遅延イベントイテレータ

#### パフォーマンス

- **パースはスプライス資格を計算しない** — O(ドキュメント) のレイアウトチェックは
  最初の編集時に `YamlDocument.splice_checked` 経由で遅延実行され、v0.11.0 の
  回帰を復元：parse_comments -59%、parse_anchors -42%、parse/roundtrip/edit
  -10~35% すべて v0.10.0 レベルに戻る
- **線形カーソルレイアウトチェック** — 事前計算済み行オフセット上のノード単位
  バイナリサーチを置換（単調なソース順トラバーサル）

#### 変更

- `parse_with_options` は `CustomNode` を返す（旧 `(CustomNode, bool)`）；
  スプライス資格は現在 `YamlDocument` 内部にあり、オンデマンドで計算されます。

### [0.11.0] — 2026-08-02

#### 追加

- **外科的シリアライズ** — 全 AST ノードのバイトレベルソーススパン追跡；
  セグメントベーススプライス — 編集はタッチされた領域のみ再生成、
  未変更テキストはバイトコピー
- プロパティテスト（proptest、新規開発依存）
- 10MB 編集フラッシュベンチマーク（divan）

#### 変更

- `flush_source` がセグメントスプライスを使用；フロースタイル領域、
  非デフォルトレイアウト文書、マージキー、CRLF/BOM 文書、materialize 後
  （シングルバーストモデル）では全量シリアライズにフォールバック
- スプライス編集は `---`/`...`/ディレクティブマーカー行を未変更バイトとして保持
  （全量シリアライズは以前それらを削除 — 意図的な動作差）

### [0.10.0] — 2026-08-01

#### 追加

- **インプレース編集** — フォーマットメタデータを失わずに解析済みドキュメントを編集：
    - パス API：`doc.set(path, value)`、`doc.insert(path, index, value)`、
    `doc.append(path, value)`、`doc.delete(path)`、`doc.rename(path, new_key)`、
    JSONPath スタイルのパス（`$.a.b[0]`）；ルート用糖衣構文
    `doc["key"] = value` と `del doc["key"]`
    - ノード API：`doc.node()` / `doc.find(path)` は `Node` オブジェクトを返し、
    `set_value` / `append` / `insert` / `delete` / `rename` とツリー走査
    （`parent`、`children`、`walk`、`filter`）をサポート
    - 完全なメタデータ保持 — 置換されたスカラーはコメント/アンカー/タグ/クォートを保持；
    リネームされたキーは位置とコメントを保持；削除時もマッピングの順序は保持
    - アトミック編集 — 失敗した操作はドキュメント（リビジョンを含む）を変更しません
    - 遅延ソース再同期 — `source()` / `to_yaml()` / `reparse()` は編集成功後にのみ再シリアライズ
    - 陳腐化ノード検出 — ドキュメント編集後の `Node` アクセスは
    `YamlDocumentError` をスロー（`RuntimeWarning` 付き）
    - 新しい例外：`YamlEditError`、`YamlPathError`（en/zh-CN/ja-JP/ko-KR の i18n 対応）
    - エイリアス対応編集 — エイリアス自身のパスへの設定はその場で置換；
    エイリアス経由の編集は `YamlEditError` をスロー
- **編集ベンチマーク** — `benches/yaml_bench.rs` に divan ベンチマークを 6 つ追加
  （小〜大ドキュメントの set/insert/delete）

#### 変更

- `YamlDocument.source()` は `str` を返し、インプレース編集後に遅延再シリアライズします。

### [0.9.0] — 2026-08-01

#### 追加

- **Python 3.13、3.14、3.15 サポート** — PyO3 `abi3-py38` wheel が
  Python 3.8-3.15 をカバー（GIL ビルド）；`abi3t` + `abi3t-py315` は
  free-threaded 安定 ABI を提供
- **Free-threaded CPython（GIL なし）サポート** —
  `#[pymodule(gil_used = false)]` がモジュールを free-threaded Python 向けに
  スレッドセーフと宣言；`Py_GIL_DISABLED` cfg フラグで numpy をゲート
  （rust-numpy は free-threaded 未対応 — free-threaded ビルドでは
  `--no-default-features` で numpy feature を無効化）
- **CI free-threaded ジョブ** — 新しい `test-freethreaded` ワークフロージョブが
  Python 3.14t でコンパイルとテストを検証
- **`pyo3-build-config` ビルド依存** — `build.rs` 経由で
  `#[cfg(Py_GIL_DISABLED)]`、`#[cfg(Py_3_15)]` などのコンパイラフラグを有効化
- **`numpy` をオプション化** — `numpy` feature の背後にゲート（デフォルト有効）；
  `Py_GIL_DISABLED` 下では自動的に除外
- **`allow_duplicate_keys`** — `YAML(allow_duplicate_keys=True)`、
  `parse(..., allow_duplicate_keys=True)`、`parse_file`、`safe_load`、
  `safe_loads`、`parse_all_docs` がすべてフラグを受け入れます；
  重複マッピングキーはデフォルトで `YamlDuplicateKeyError` を発生、
  許可時は `last value wins`
- **`SerializeOptions` の拡張** — `doc.to_yaml_with_options()` が
  `width`（行ラップ、0 = 無効）、`indent_mapping`、`indent_sequence`、
  `indent_offset` を既存の `indent_size`/`explicit_start`/`explicit_end`/
  `sort_keys`/`max_depth` とともに追加（`src/py/mod.rs:432`）
- **タグハンドラレジストリ** — `register_tag("!custom")` デコレータと
  インペラティブフォーム + `clear_tag_handlers()`；登録タグを持つスカラーノードは
  ハンドラを介して変換されます（`src/py/tag_registry.rs`）
- **優先度付きタグハンドラチェーン** — 複数のハンドラがタグごとに昇順
  `priority` で実行；`YamlTagSkip` はハンドラが次のハンドラに通すことを許可、
  fallback は元の値を保持
- **Pydantic 統合** — `parse_as(Model, yaml, **yaml_kwargs)` が YAML をパースし
  Pydantic v2 モデルに対して検証；pydantic がない場合は `ImportError` を
  ガイド付きで発生（`python/pyrs_yaml/pydantic.py`）
- **`.pyi` 型スタブ** — maturin によって自動生成されコミットされ、
  `register_tag`、`parse_as`、`to_yaml_with_options` および新しい例外が
  型チェッカーから見えるようになります。

#### 変更

- CI Python マトリクスを拡張：ubuntu、windows、macos で 3.8-3.14
- 安定 ABI：`abi3-py39` → `abi3-py38`（より広い Python 3.8+ サポート）、
  `abi3t` + `abi3t-py315` を追加（free-threaded 安定 ABI）
- `pyproject.toml` の classifiers に 3.13、3.14、3.15 のエントリを追加
- **CI 最適化：重複する Rust コンパイルを除去** — 単一の `rust-lint` ジョブが
  `cargo clippy` + `cargo test` を 1 回実行；ビルドジョブは OS ごとに 1 つの
  abi3 wheel を生成し、テストジョブが `maturin develop` を実行する代わりに
  インストールするため、21 のマトリクスジョブから Rust コンパイルを除去
  （約 86% のコンパイル削減）；`Swatinem/rust-cache` を全ジョブに追加
- **pydantic テスト依存関係** — `pydantic>=2.10.6` を
  `[dependency-groups] test` と `.ci/requirements-test.txt` に追加
  （ci.yml 内の `uv sync` による SSOT）

#### 修正

- **Windows DLL 読み込み** — `src/py/tag_registry.rs` から
  `#[cfg(test)]` ブロックを削除し、Windows での `import pyrs_yaml` を
  修正（`250b8d0`）
- **Python 3.8 互換性** — `pydantic.py` に `from __future__ import annotations`
  を追加（`63d2495`）
- **CI pydantic スキップ** — `pytest.importorskip("pydantic")` を追加し、
  pydantic が未インストールでもテストがパスするよう修正（`7be011d`）
- **CI の Windows でのグロブ展開** — `pip install dist/*.whl` の
  `shell: bash` を追加（PowerShell は `*` を展開しない）（`2f7778d`）
- **文字列以外を返すタグハンドラが `YamlTagError` を発生** — 非 `str` 値を
  返すハンドラ（以前は黙って無視され元のスカラーを保持）が、
  `Tag handler '!x' must return a string` でエラーを発生（`src/py/mod.rs:resolve_tags`）
- **`to_yaml_with_options` インデント配線** — `indent_mapping`/`indent_sequence`/
  `indent_offset` がシリアライザによって尊重されるようになりました
  （以前は死んだフィールド）；省略時はそれぞれ `indent_size`/0 にデフォルト
  （`src/serializer.rs`）
- **`width` が小さな値でハングしない** — `width < continuation indent` の場合、
  永久ループの代わりに未ラップで残りを出力するフォールバックに
  （`src/serializer.rs:write_plain_scalar`）
- **`remove_tag(name)`** — タグハンドラの登録解除用新関数；
  `register_tag`/`clear_tag_handlers` を補完（`src/py/tag_registry.rs`）
- **`duplicate-key` エラーが多言語化** — `YamlDuplicateKeyError` メッセージが
  全 4 ロケールで `format_i18n_error` を経由するようになりました
  （`src/i18n/locales/*.yml`）

### [0.8.0] — 2026-07-30

#### 追加

- **`YAML()` インスタンス API** — 再利用可能な設定付き
  `YAML(typ="rt"|"safe"|"full", schema="core"|"yaml1.1", max_depth=1000)`；
  `.parse()`、`.safe_load()`、`.safe_loads()`、`.parse_file()`、
  `.parse_all_docs()` メソッド
- **Python `Node` API** — AST 操作のための
  `Node` クラス：`find()`、`filter()`、`walk()`、`to_yaml()`、`parent`、
  `children`、`root_type`、`value`；JSONPath 風クエリ言語
  （`$.key.sub`、`$.arr[0]`、`$..deep`）
- **`doc.version` メタデータ** — `YamlDocument.version()` が YAML 仕様バージョンを返す
  （デフォルト "1.2"）
- **`MergedView`** — `doc.merged()` がマージキー解決済みのおよび読み取り専用
  辞書風ビューを返す
- **ライフサイクル警告** — `Node.release()` でノードを明示的に無効化；
  陳腐化したアクセスは `RuntimeWarning` + `YamlDocumentError` を発生

#### 変更

- `parse()` / `safe_load()` は構文糖衣として
  `YAML().parse()` / `.safe_load()` に委譲するようになりました
- `YamlDocument` はドキュメントメタデータの `version` フィールドを保持するようになりました

### [0.7.1] — 2026-07-30

#### 追加

- **ryaml ベンチマーク比較** — `tests/test_benchmark.py` が
  PyYAML と ruamel.yaml と並んで `ryaml`（Rust YAML ライブラリ）とも比較するよう
  になり；`benchmark_compare.py` が機能比較レポートとして書き直されました
  （`tests/test_benchmark.py:25-28`、`.github/workflows/ci.yml:219`）
- **CI 準拠閾値の引き上げ** — YAML Test Suite 準拠ゲートが
  `test_compliance_report()` で 70% から 75% に増加；
  有効パースレートゲート 95%（`tests/test_yaml_suite.py:251`）
- **CI 依存関係の統合** — パブリッシュワークフローとローカル開発全体の
  統一テスト依存関係管理のため、`.ci/requirements-test.txt` と
  `.ci/requirements-test-lite.txt` を追加
- **ベンチマークの近代化** — 高速な C 拡張ベースの統計ベンチマークのため
  `pytest-benchmark` から `pytest-codspeed` へ移行；全 CI ジョブが
  `-r .ci/requirements-test.txt` を使用するようになりました
- **Rust ベンチマークを Divan に移行** — `codspeed-criterion-compat` を
  `codspeed-divan-compat` v5.0.1 に置換；16 個のベンチマークを
  Criterion グループから `#[divan::bench]` 属性に書き直し
  （`Cargo.toml`、`benches/yaml_bench.rs`）

#### 変更

- CI ベンチマークジョブがクロスライブラリ比較用に `ryaml` をインストール
- `benchmark_compare.py` はタイミングを `pytest-benchmark` に委譲し、
  機能比較/レポートツールとして機能するようになりました

### [0.7.0] — 2026-07-29

#### 追加

- **シリアライザ `max_depth` ガード** — `serialize_node_internal` が
  再帰深度を追跡し、制限（デフォルト 1000）を超えると `YamlMaxDepthError` を
  発生（パーサーの保護と一致、`src/serializer.rs:135-145`）
- **シリアライザホットパス最適化** — ブロックスタイルシリアライズを対象とした
  5 つの最適化で約 4.9% のラウンドトリップ高速化：
    - `write_anchor_tag` および `write_inline_comment` の None チェックをインライン化
    （全ノードの約 99% でメソッドコールを除去）
    - `write_indent` のホット/コールドパス分離（キャッシュレベル ≤64 の直接インデックス）
    - `write_plain_scalar` の短小 ASCII 英数字文字列（≤8 文字）用高速パス
    - `write_scalar_for_key` の Plain スカラー用直接ディスパッチ（ディスパッチチェーンを回避）
- **pytest-benchmark 移行** — Python ベンチマークが
  統計的厳密さ、構造化 JSON 出力、CI 統合のため
  生 `time.perf_counter()` から `pytest-benchmark` へ移行
  （`tests/test_benchmark.py` + 更新済み `tests/test_performance.py`）

#### 変更

- Python ベンチマークで生 `timeit` の代わりに `pytest-benchmark` を使用
- CI ベンチマークジョブがスタンドアロンスクリプトの代わりに
  `pytest --benchmark-json` を実行

#### 削除

- `write_inline_comment` メソッド — 全呼び出し箇所でインライン化
- シリアライザからの `Comment` インポート — 不要になった

### [0.6.0] — 2026-07-27

#### 追加

- **非同期シリアライズ** — `asyncio.run_in_executor` 経由の
  `safe_dumps_async`、`safe_dump_async`、`safe_loads_async`、
  `safe_load_async`（`python/pyrs_yaml/async_dump.py`）
- **JSON Schema 検証** — `YamlValidateError` 例外 +
  `YamlDocument.validate(schema)` メソッド（`str` または `dict` を接受）；
  Python `jsonschema` モジュールに委譲
- **`YamlDocument.to_json()`** — ドキュメントを JSON 文字列にシリアライズ
  （Python `json.dumps` を使用）
- **増分再パース** — `YamlDocument` がソーステキストを保持するようになりました
  （`doc.source()`）；`doc.reparse(resolve_merges=True, schema="core")` で
  インプレースに再パース可能
- **29 個の新規テスト** — `test_async.py`（8）、`test_validate.py`（14）、
  `test_reparse.py`（7）にまたがって

#### 変更

- `YamlValidateError` が新しいカスタム例外として登録（`ValueError` を継承）
- `rust_i18n::i18n!` マクロパスが `"src/i18n/locales"` に更新
- `validate_translations()` テストパスが新しいロケールディレクトリに一致するよう更新

#### 削除

- 冗長な `src/i18n/en.ftl`、`src/i18n/zh-CN.ftl` を削除
  （rust-i18n から参照されなかった）
- `locales/*.yml` を `src/i18n/locales/` に移動（i18n モジュールと共置）

#### 依存関係変更

- ランタイム依存：`jsonschema>=4.25.1`
- 開発依存：`pytest-asyncio>=0.23`（ランタイムから移動、ピン留めされなくなりました）

### [0.5.0] — 2026-07-27

#### 修正

- **`Serializer::write_node`** — `block_mapping`/`block_sequence` 内の
  `values.iter().next().unwrap()` での `.unwrap()` を安全なインデックスアクセスに
  置換し、エッジケース AST での潜在的パニックを除去
- **`YAML_SCHEMA` 定数** — 誤字 `yamorg2002` を
  `yamlorg2002` に修正（YAML 1.2 仕様 URL と一致）
- **開発ドキュメント** — `AGENTS.md` を更新し、Python コマンドに必須の
  `uv run` プレフィックスと Rust コマンドに直接 `cargo` を明記

### [0.4.0] — 2026-07-27

#### 追加

- **132 個の新規ギャップフィルテスト** — 未テストの API に対する包括的カバー
- **i18n 関数テスト** — `set_language`、`get_language`、`list_languages`、
  `detect_language`、`negotiate_language`
- **`parse_all_docs` 専用テストスイート** — 単一ドキュメント、複数ドキュメント、
  空、コメント
- **`parse_file` 成功ケーステスト** — 基本パース、コメント保持、ファイル未見つけエラー
- **`to_yaml_with_options` テスト** — `explicit_start`、`explicit_end`、
  `indent_size`、`sort_keys` 順序保持
- **`to_dict()` メソッドテスト** — スカラールート、ネスト、リスト、bool、null、
  アンカー解決、空マッピング/シーケンス
- **YamlDocument ダンダーメソッドテスト** — `__repr__`、`__str__`、
  `__contains__`、`__len__`、`__iter__`、`__getitem__`、`root_type()`
- **バイト入力テスト** — `parse(b"key: value")`、UTF-8 バイト、
  不正 UTF-8 エラー
- **Unicode と特殊文字テスト** — CJK、絵文字、ラウンドトリップ、CRLF 改行、
  重複キー
- **`safe_load`/`safe_loads` フィーチャーカバー** — アンカー、マージキー、
  ブロックスカラー、フローコレクション、特殊フロート、型解決
- **`from_dict` エッジケース** — キー内の特殊文字、ネストリスト、
  None 値、空辞書/リスト
- **`from_json` ラウンドトリップ** — ネスト構造、配列、不正 JSON エラー
- **`dump_file` テスト** — 成功パス、不正パスエラー
- **YAML Test Suite 個別ケーステスト** — 8 進数、16 進数、科学表記法、
  NaN、無限大、マージキー、明示的/暗黙的キー、bool/null 変種、
  ブロックスカラーストリップ（`|-`）、フローコレクション
- **`resolve_merges` パラメータテスト** — 無効時は `<<` を保持、
  デフォルトで解決
- **フローコレクションラウンドトリップ** — ルートレベルとネストされた
  フローマッピング/シーケンス
- **非スカラーノード上のアンカー** — マッピングアンカー（`&defaults`）と
  シーケンスアンカー（`&items`）
- **シーケンスインデックステスト** — 正のインデックス、範囲外エラー
- **マージキー統合** — 解決済みと未解決のマージキーのラウンドトリップ
- **タグ保持** — `!!seq` と `!!map` タグのテストカバー
- **コメント保持** — 複雑構造のインラインおよびスタンドアロンコメントテスト

#### 変更

- バージョン同期を修正：`python/pyrs_yaml/__init__.py` の `__version__` が
  0.2.0 から 0.4.0 に更新され、Cargo.toml/pyproject.toml と一致
- `dist/` から古い 0.2.0 wheel artifact を削除

### [0.3.0] — 2026-07-27

#### 追加

- **NumPy ndarray シリアライズ** — `safe_dump()` / `safe_dumps()` /
  `from_dict()` / `dump_file()` が全次元（0-D から N-D）の
  `numpy.ndarray` をサポートするようになりました
    - 対応 dtype：`int8/16/32/64`、`uint8/16/32/64`、`float32/64`、
    `complex64/128`、`bool`
    - 多次元配列は正しいインデントでネストした YAML リストとしてシリアライズ
    - 複素数は `(re+imj)` 文字列形式でシリアライズ
    - `0-D` スカラー配列は 1-D に reshape され、単一要素リストとしてシリアライズ
    - ゼロコピー dtype ディスパッチ用の `PyUntypedArray` + `PyArrayDyn`
    （`numpy` Rust crate 経由）
    - 最大パフォーマンスのためのスライス反復時の GIL 解放
- **`quoted_scalar()`** — 単一引用 YAML 形式を必要とする値用の
  新 `CustomNode::quoted_scalar()` コンストラクタ
- **引用付きスカラーの型解決** — 引用付き負数の正しいラウンドトリップのため
  `resolve_yaml_type` が `SingleQuoted`/`DoubleQuoted` スカラーに適用されるようになりました
- **包括的 NumPy テストスイート** — 全 dtype、次元（0-D から 4-D）、
  負の数、無限大、NaN、空配列、エッジケースをカバーする 42 個のテスト

#### 修正

- **負の数ラウンドトリップ** — YAML 1.2 のブロックシーケンスに
  `-` で始まるプレーンスカラーは含められないため、負の数はシリアライズ時に
  引用され、整数/浮動小数点数として正しくパースされるようになりました
- **N-D 配列サポート** — 1-D のみに限定されず任意次元の配列をサポートするよう
  `PyArray1<T>` を `PyArrayDyn<T>` に置換
- **正しいネスト深さ** — 多次元配列がちょうど N レベルのネストを生成
  （shape[1..] が内部次元を処理、ルート次元が `plain_sequence` でラップ）

#### 変更

- ndarray 型ディスパッチ用の依存関係として `numpy` crate（v0.29）を追加

#### 追加

- フローコレクション（`{}`/`[]`）のラウンドトリップサポートを
  Mapping/Sequence AST ノードの `flow_style` フィールドで追加
- `parse()` が `str` と `bytes` の両方の入力を接受
- `parse()` がマージキー展開のオプトアウト用 `resolve_merges` パラメータをサポート
- saphyr イベントによる複数ドキュメントパース用 `parse_all_docs()`
- `indent_size`、`explicit_start`、`explicit_end`、`sort_keys` パラメータ付き
  `to_yaml_with_options()`
- デフォルト値パラメータをサポートする `get()`
- YAML をファイルに書き込む `dump_file()`
- `benches/yaml_bench.rs` の Criterion ベンチマーク（パース/シリアライズ/ラウンドトリップ）
- マトリクステスト付き GitHub Actions CI（3 OS × 4 Python バージョン）
- アンカー名パーサーが全 YAML 1.2 仕様（ドット、コロン、ハッシュ、引用アンカー）に拡張
- `__version__` 属性、`py.typed` PEP 561 マーカー

#### 修正

- `to_dict()` と `safe_load()` 内のエイリアス解決 — エイリアスが
  `None` ではなく参照値に解決されるようになりました
- `safe_loads()` が単純な `split("---")` を使用しなくなり、
  saphyr のドキュメントイベントを使用
- パース中のマッピング/シーケンスタグが廃棄されなくなりました
- `format_scalar_for_key()` が Literal/Folded ブロックスカラー形式を処理するようになりました

#### 変更

- PyO3 を 0.21 から 0.29 にアップグレード
- 15 個以上のボイラープレート `CustomNode` 構築を
  `plain_scalar()`/`plain_mapping()`/`plain_sequence()`/`plain_null()` コンストラクタに置換
- シリアライザが `write_anchor_tag()` と `write_inline_comment()` ヘルパーを抽出
- パーサーが `detect_flow_style()` ヘルパーを抽出
- 死んだコードを削除：`ParseOptions`、`find_inline_comment`、
  `find_standalone_comment_before`、`format_yaml_type`（テスト専用）
- 6 個の重複テストファイルを統合し、9 個の診断スクリプトを `scripts/` に移動
- キー/インデックス/型コンテキスト付きのエラーメッセージを改善

### [0.1.0] — 2026-07-25

#### 追加

- saphyr-parser による YAML 1.2 準拠の初期リリース
- 完全なメタデータ（コメント、アンカー、タグ、チョンピング、スカラー形式）付き
  カスタム AST
- コメント、アンカー、タグ、フォーマットのラウンドトリップ保持
- PyYAML 互換 API（`safe_load`/`safe_dump`）
- `from_dict`/`from_json` 変換関数
- YAML フロント matter 抽出用 `read_markdown`/`read_markdown_str`
- チョンピング インジケータ付きブロックスカラー（`|`/`>`、`|-`/`|+`/`>-`/`>+`）
- エスケープシーケンス（`\n`、`\t`、`\uXXXX`、`\xXX`）
- YAML 1.2 型解決（null、bool、int、float、無限大、NaN）
- マージキー解決（`<<: *alias`）
- 複合キー（シーケンス/マッピングをキーとして）
