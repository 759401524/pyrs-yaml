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

#### 追加

- **YAML のコメント・アンカー・タグが往復し、全シードを門として検査。** —
  `crates/pyrs-yaml-core/tests/note_survival.rs` は確定済みのシード語料を毎回の `cargo nextest run`
  で再生し、読み取り側が記録したコメントがすべて出力テキストに現れることを要求します。往復層の判定基
  準はテキスト冪等性なので「安定だがコメント 1 本足りない」文書は素通りします。沈黙したコメント消失
  5 件が緑のまま続いた原因がこれです。— (details: quality-ledger — Note survival, (w), (ac))

- **入れ子・エイリアス付きマージキーが実際に適用される。** — `<<: {<<: {x: 1}}` は 2 層の `<<` を
  データとして保持し（`{'<<': {'<<': {'x': 1}}}`）、PyYAML と ruamel が `{'x': 1}` と読むのと差が
  ありました。`<<: {<<: {x: 1, y: 1}, y: 2}` では `x` が消え、ブロック形の `- <<:` も同じでした。
  収集側はノード全体の一致で「既に持っている」と判断しなくなりました。 — (details: quality-ledger
  (ae), (af), (ag), (o), (q), (s))

- **命令数ゲートが書式間の橋を双方向で数える。** — `from_jsonc`、`from_json5`、`to_jsonc_text`、
  `to_json5_text`、`to_python_small/medium/anchors`、JSON/TOML の読み取り経路が
  `.ci/ir-baseline.json` の基準値を持ちます。同じ hub AST で 3 つの writers は
  21,232,786 / 22,176,851 / 22,974,851 命令：コメント出力で 4.4%、JSON5 の綴りで 8.2% の追加コスト、
  という数字を初めて示せます。 — (details: quality-ledger (am), (ap), (aq), (as), (ax))

- **品質防御が自身を測定し、穴は findings として出る。** — `scripts/quality_matrix.py` は防御を宣言
  するファイル（`.github/workflows/*.yml`、`prek.toml`、`fuzz/Cargo.toml`、`scripts/check_*.py`、Ir
  ベンチ、 `.ci/ir-baseline.json`）から `QUALITY_MATRIX.md` を再導出し、転記はしません。報告された穴
  は `.ci/quality-holes.json` に両方向の出口を明示して登録され、注入ツリーで発火するテストを備えま
  す。 — (details: quality-ledger (ah), (ai), (aj), (ak), (an), (ao))

- **プロダクトを動かす変更はリリースノートも動かす。** — `scripts/check_changelog_coupling.py` は PR
  のファイル一覧に 2 規則を適用します：`crates/`、`python/pyrs_yaml/`、`fuzz/`、`scripts/`、
  `tests/`、同梱マニフェストに触れる diff は changelog に触れる必要があり、5 つのミラーのいずれかを
  変えたら 5 つすべてを変えます。ある機能がノート無しで出荷された後に追加されました。— (details:
  quality-ledger (al), (ar))

- **タイミング下限は 2 相を隣接サンプリングし、証拠を印字する。** — `tests/timing.py` が唯一の
  サンプラーです：候補と参照を同一ブロック内で隣接計測し、判定はブロック最小値・最低 2 ペアの勝利、
  失敗メッセージは全ペアを引用するので、赤はどちらが動いたかを語ります。`macos-latest` の
  `139.4us vs 359.6us (2.58x)` という報告で 5 ブロック中 3 ブロックが自身の下限の約 3 倍に並んだ件が
  これに当たります。ライブラリ間の下限は 5 倍と top-3 を維持（実測 PyYAML 比 38x-280x、ruamel 比
  71x-77x）。 — (details: quality-ledger (aw), (ay))

- **散文は 100 表示桁、整形器に切られた見出しは拒否。** — `ROADMAP.md` に 21,850 字、
  `docs/ja/changelog.md` に 884 字の行がありました。`scripts/check_doc_wrapping.py` は表示桁を測り
  （全角 2 桁）`--fix` を持ちます。`check_doc_headings.py` は、折り返し後半行が issue 参照で始まった
  とき `rumdl fmt` が昇格させる見出しを拒否します。— (details: quality-ledger (au), (av))

- **各エンジンに fuzz ターゲット、週次実行と PR 層のブロッキング。** — 4 つの libFuzzer ターゲット
  （`parse_yaml`、`yaml_roundtrip`、3 ディアレクト × 3 writers を交差させて各出力を再パースする
  `parse_json`、`parse_toml`）、シードは `fuzz/seeds/`。追跡するのはターゲットとシードだけで、機械語
  料はリポジトリに入りません。— (details: quality-ledger (aj))

- **`pyrs-ast`・`pyrs-schema`・`pyrs-json`・`pyrs-toml` が `no_std` 対応。** — 4 クレートは `alloc`
  だけでビルドされます：`indexmap` と `thiserror` の既定 `std` feature を切り、オプトインの `std`
  feature が `std::error::Error` と `RandomState` を復元します。既定は `std` のままなので、既存利用
  者のノードマップ型は変わりません。— (details: boundaries — Known Engine Boundaries)

- **`pyq` を各リリースでプリビルドバイナリとして同梱。** — `publish.yml` の `pyq` ジョブが 6 プラッ
  トフォームのネイティブ CLI をビルドし、アーカイブを GitHub Release に添付します。独立バイナリに
  Rust 環境は不要になりました。Linux 側は manylinux2014 コンテナ内で `cross` によりネイティブコンパ
  イルし、 glibc 2.17 を下限に固定します。— (details: perf — Leaderboard & Performance Status)

- **確定済みの型スタブをビルド済み拡張と比較する。** — `scripts/check_stub_drift.py` は宣言済みの経
  路で `python/pyrs_yaml/pyrs_yaml.pyi` を再生成し、差異があれば失敗します。このジョブは
  `windows-latest` で動きます — maturin の自省経路は Linux コンテナで拡張を読み込めないため。以前は
  存在と追跡だけを検査していたので、公開型の契約が静かに遅れられました。— (details: quality-ledger
  (ap))

- **スキーマ言語が容器の形を名乗れます。** — `type: map` と `type: seq` は中身を問わずノードの形を主
  張し、`map` / `seq` は要素の型としても使えるので、`mapping_of: seq` で「このマッピングのすべての値
  はシーケンス」と書けます。これは `[*]` がシーケンス添字にしか届かずマッピングキーに届かないため、
  これまで表記が存在しませんでした。`mapping`、`object`、`sequence`、`array`、`list` も受け付けま
  す。スキーマは道具ではなく人が打つものだからです。`path` を欠いた容器型は書かれた時点で拒否されま
  す。対象ノードが無い以上、何も選ばず何も検査しません。— (details: quality-ledger (bm))

#### 変更

- **`path` を書いた検証規則はそのノードを主張します。** — 変更前に測定済み：`$.config` に
  `mapping_of: str` を置いて `config` がスカラーやシーケンスでも通り、`$.port` の `type: int` は
  `port` がマッピングでも通り、`sequence_of: int` は `[[1, 2]]` を通します。どの検査も `if let` の中
  にあり、自分が記述できる種類のノードしか見ず、他の種類は素通りでした —— 見ていない文書に「妥当」と
  答える検証器です。要素も同じように検査されます。入れ子のシーケンスは `int` ではありません。`path`
  を省いた規則は従来の別の意味を保ちます。ノードを指定できないので、記述できるノードを形で選びます
  —— パスなしの `type: str` は「すべてのスカラーは文字列」という意味で、文書の残りは問いません。別名
  のノードに関する規則は判定しません。別名は形ではなく名前を保持し、検証器はアンカー表を持たないため
  です。— (details: quality-ledger (bm))

- **`[*]` は一つの要素を指し、部分木全体を指しません。** — `$.rows[*]` は `$.rows[0].a` にも一致して
  いました。照合が接頭辞と接尾辞を比べ、その間に何かが在ることを求めるだけだったためです。規則が形を
  無視していた間は目立たず、形を主張し始めた瞬間に、一致した要素の下にあるすべての値が誤った違反に
  なりました。`$.rows[*].a` は引き続き名前付きの要素に届きます。— (details: quality-ledger (bm))

- **一つの検証規則は一つの検査だけ持ちます。** — `type` と `mapping_of` を並べて書いた規則は、後に書
  かれた検査だけを実行し、前者には触れません。パーサの各枝が同じフィールドへ代入していたためです。一
  つの規則に二つの検査がある場合はスキーマ解析が失敗します。`required` は直交するので、どの検査とも
  併用できます。— (details: quality-ledger (bm))

- **厳格 JSON の書き出しは非有限浮動小数を別の表記にせず拒否します。** — RFC 8259 に無限大や NaN の
  リテラルが無いので、`to_json` / `to_jsonc` は中枢のテキストを引用して `{"b": ".inf"}` と答えていま
  した。`load_json` はそれを文字列として返すため、出力の途中で数が文字列になり、そのテキストを値とし
  て持つ文書と区別できません。#312 が入口で直したのと同じ種類の失損です。代案は秤にかけず測定しまし
  た：`null`（`JSON.stringify` の答え）は別の数ですし、裸の `Infinity` は本ライブラリの厳格リーダー
  が意図通り拒否するテキストです（ここは CPython の `json.loads` より厳い）。いまの書き出しは安定理
  由キー `json-cannot-represent-non-finite` を載せた `YamlSerializeError` を投げます。語を持つ
  `to_json5` は往復を保ちます。拒否が内部障害を装わなくなりました：新変種
  `SerializeError::UnsupportedValue` は「その形式はこの値を載せられない」専用にあたり、キー・エイリ
  アス・null・タイムスタンプが従来 `internal-error` と報告されていたのはそこでした。

- **`pyq validate` が `--input` を受け取り、実際に読んだ形式に従う。** — YAML パーサをハードコードし
  ていたため `pyproject.toml` や `package.json` は即座に拒否されていました。現在は
  `--input auto|yaml|json|jsonc|json5|toml` を受け取り、共通ローダ経由で読みます。

- **PR 層の fuzzing をブロッキングに。** — ステップ単位の `continue-on-error` は、`main` がこの層が
  正しく指摘したドリフトを抱えている間だけ置く留め金でした。現在のツリーで再検証：4 ターゲットとも確
  定済みシードをクリーンに再生（5/59/6/5 シード、nightly-2026-08-15、cargo-fuzz 0.13.2）。 —
  (details: quality-ledger (aa))

- **Docs 環境を最新版へ、スタブ生成器は意図的に据え置き。** — `zensical` 0.0.56 → 0.0.69、
  `mkdocstrings-python` 1.11.1 → 2.0.9（`mkdocstrings` 1.0.6、`griffelib` 2.2.0、いずれも Python
  3.11 必須）。一方 `maturin` は 1.14.1 のままです — 確定済みの `.pyi` と漂移検査の照合規則がその出
  力を前提にするため。設定された 14 の handler オプションは導入済み `PythonOptions` と照合し、未知の
  オプションはビルドが黙って無視するので検査はテスト側に置かれました。— (details: quality-ledger
  (ba))

- **API リファレンスはライブラリ自身の docstring と型スタブから生成される。** —
  `docs/<locale>/api/{reference,yaml-document,yaml-instance,node,merged-view}.md` に `mkdocstrings`
  ディレクティブを置き、サイトの署名は書き写しでなくコードから出るようにしました。4 ロケールともビル
  ドして検証済み：`YamlDocument` に 23 の署名、`Node` に 41、モジュールページに 48、ディレクティブの
  ないページは 0 のまま。`exceptions.md` は手書きのままです — 生成されたスタブがクラスを 4 つしか宣
  言せず、例外型が一つも含まれないため。— (details: quality-ledger (be))

#### 修正

- **位置付きの検証エラーも、一致した `path` を示します。** — `SchemaValidationError` は
  `line:column: message` か `path: message` のどちらかだけを印字していたので、ノードにソース範囲が
  付いた瞬間、スキーマ作者が書いた方の手がかりが消えました。`1:9: expected map but got sequence` は
  どのキーかを示しません。今は両方を印字します。— (details: quality-ledger (bm))

- **マッピングキーが、同じテキストを値に置いたときの意味を持つ。** — `1: a` は `{"1": "a"}`、 `a: 1`
  は `{"a": 1}`、`~: 1` は `{"~": 1}`、`a: ~` は `{"a": None}` と読めていました。同じ文書が `:` のど
  ちら側にスカラーがあるかで 2 つの意味になり、整数・真偽・null をキーにした設定は検索に到達しませ
  ん。キーは値と同じ schema 経路で解決され、PyYAML・ruamel と一致します。`load_toml` はどちらの YAML
  schema でも型が変わるキーを引用符で括ります。— (details: quality-ledger (as), (at))

- **アンカー名の文法を読み取り側と整え、ドリフトの一系列を閉じた。** — 出力側が読み取り側に拒否され
  るアンカー名（`found unknown anchor`）を出しうるため、「解析不能な出力は作らない」という契約を破っ
  ていました。`:` 終わりは毎巡 1 字落ち、引用付き名は改行を飲み込み、素のスキャナはコメントテキスト
  や重なる `&` から存在しないアンカーを作り出します。現在は granit と同じく、アンカー文字の最長連続
  列として読みます。— (details: boundaries — Fuzz findings, (x), (y), (z))

- **コメントがマーカー・タグ・コンテナを越えて所有者を保つ。** — 初の出力と再読み取りのあいだでコメ
  ントが所有者を変えたり消えたりする系列です：タグだけの値（`k: !`）が直後の行を先頭コメントとして抱
  える、コンテナ自身の行内コメントが保持できない行に落ちる、引用内の `#` を素のバイト走査が追い出
  す、簡潔な `- key:` 分岐がテキストだけ写してコメント槽を見ない、先頭コメントが単一 *スロット* なの
  で最後に重ねたものだけが残る（`# alpha` + `# beta` + `key: 1` → `# beta`）。先頭コメントは現在リス
  トで、全 writers が従います。— (details: quality-ledger (h), (i), (m), (n), (p), (t), (u), (v),
  (ad))

- **マージの同一性と深さを修正、ネイティブスタック溢れも解消。** — `<<: &b` は値が null のリテラルな
  マージキーなのに下の本物の `<<:` と畳み込まれ、畳む過程でdropped 側のエントリのコメントは運びます
  がアンカーは運びませんでした。コメント付きの非タグ `y` とマージされた `y` は別の `IndexMap` キーな
  ので `to_yaml` が同じ層に `y:` を 2 度印刷し、入れ子・自己参照の連鎖は循環ガードを pop 済みの状態
  で末尾再帰に流れ、深さに上限なく溢れました（58 バイトの libFuzzer 発見）。コメント付き `<<` キーノ
  ードはこのパスに見えていませんでした。— (details: quality-ledger (o), (q), (r), (s), (y))

- **ブロックスカラーは 1 回の出力で収束し、指示子も正しく判定。** — `>+8\r\r#` は値が改行 1 つ・
  `Keep`・明示インデント 8 の折りたたみスカラーとして読めます。出力側は指示子を残して `>+8\n\n` を出
  し、それは `Clip`・指示子なしで読み戻るので、*2 巡目* は `>\n\n` — Clip は末尾改行を剥がすため値は
  `""` です。値が空へ崩落するのに各巡は「安定」に見えました。折りたたみの連続改行、`4RWC` 型の明示イ
  ンデント、先頭が空のリテラルも同じ作業で再解析に対して閉じられました。— (details: quality-ledger
  (aa), (ab), (r))

- **Unicode の空白・BOM・タグ接尾辞が文書を壊さない。** — ディアレクト fuzzing が到達した空白集合、
  BOM の位置、タグ接尾辞の境界を読み取り側自身の文法に合わせ、それぞれ確定済みシードと決定的な Rust
  回帰テストで固定しました。— (details: boundaries — Fuzz findings, Blank set)

- **JSON/JSONC のコメント走査が文字の途中で panic しない。** — 行コメントと未閉鎖ブロックコメントの
  走査が `pos` を 1 バイトずつ進めていたため、末尾のマルチバイト文字（U+FEFF、fuzzing 約 25 秒で到
  達）が内側に停止点を作リ、次回の `&text[pos..]` が「not a char boundary」でクラッシュしました。現
  在は完全なコードポイント単位で進み、不正入力は clean に拒否します。— (details: boundaries — Fuzz
  findings)

- **重複キーは値で拒否し、畳むのは null キーだけ。** — 比較は値基準なので `{a: 1, a: 2}` は引き続き
  `YamlDuplicateKeyError`、空・null キー（`: a` + `: b`、`~: a` + `~: b`）は yaml-test-suite `2JQS`
  の要求どおり畳みます。コメント付きキーが名前で到達できない問題も直りました：`CustomNode::hash` が
  正規化後のコメント表示を畳む一方 `eq` は素のスロットを比較するため、`doc["key"]`、`in`、マージ展開
  が明らかなキーに「そのようなキーはない」と答えていました。— (details: quality-ledger (ae))

- **空コンテナはインラインで出力され、2 巡目を要求しない。** — `safe_dump({"a": {}})` は
  `"a:\n  {}\n"` を作る一方、AST 側 writer は `"a: {}\n"` を作ります。シーケンス下の `- \n  {}` は
  flow ノードとして読み戻され、もう一度動きます。どちらのテキストも同じデータに戻るため往復テストに
  は見えませんでした — fuzz 層が主張する不変条件は*テキスト*が不動点だという事です。 — (details:
  quality-ledger (ab))

- **`pyrs-toml` がベアメタルで再びビルドできる。** — 積みコメント作業が `#![no_std]` クレートに
  `std::mem::take` を入れており、ホストビルドは許容しても `no-std-check` は許しません。6 か所を
  `core::mem::take` に変更。別件で、複数行インラインテーブルの末尾でないメンバーのコメントが区切りコ
  ンマの後に出力され — TOML はコメント内にコンマを保てない — 読み取り側が次のキーの先頭コメントへ移
  すため、テキストが収束しませんでした。— (details: boundaries — Known Engine Boundaries)

- **Linux の free-threaded wheel が出荷し、`pyq` も成果物をビルド。** — wheel 行列は Windows と
  macOS のみ free-threaded を作っていたため、GIL なし実行系の Linux ユーザーに導入物がありませんでし
  た：GIL ありの `cp38-abi3` は `Py_GIL_DISABLED` と ABI 非互換で、`abi3t` は 3.15 から。クロスアー
  キの `pyq` レグは qemu binfmt を登録し manylinux イメージ内で自身を検証します — ホストには翻訳機が
  あり外部の loader が無いので、 aarch64/armv7 を直接実行すると `main` の前で死にます。— (details:
  quality-ledger (aq))

- **ゲート装置自体の 6 欠陥を修正（MSRV レグを含む）。** — 3 つのチェッカーが Python 3.8 で import
  不可（import 時に評価される注釈、`str.removeprefix`）、シナリオ探索が 1 つの harness ファイルしか
  名指さず 2 番目の経路が門を外れ、Ir の crate 集合は `[dependencies]` で止まり半分だけ読んだ図を説
  明し、基準更新が rustc 1.99.0 で 1.97.1 の記録に合わせ `serialize_medium` を +6.7% 動かし、集合チ
  ェックは 11 ジョブ中 3 つだけを待ち、プロパティ層は `PROPTEST_CASES` を誰も設定せず proptest 既定
  の 256 用例で回っていました。— (details: quality-ledger (ah), (an), (ao), (ap), (aq), (ar))

- **誰も見ない位置へエントリを登録できない。** — あるコミットは 5 ミラーすべてにノートを加えながら、
  `CHANGELOG.md` では前置きの上に、en と zh では `tags:` リストの中に、ja と ko では front matter と
  最初の見出しの間に置きました。全ファイルで本文の外なのにミラー検査は緑でした —「版見出しの集合が同
  じ」は読者がどこを見るかを何も言わないためです。`placement_errors()` が位置を検査します。 —
  (details: quality-ledger (ar))

- **更新ログページが自身の説明を取り戻し、折りたたみ版も表示される。** — 幅の修正器がページ YAML
  front matter の開始 `---` だけを飛ばしたため、4 言語ページの `title:`/`description:`/`tags:` が 1
  行へ再整形され、デプロイ済みの生成器はページ説明をサイト説明に置き換えて公開し、新しい版はビルドを
  失敗させます。折りたたみ版は素の `<details>` で開かれ、Python-Markdown はその内容を解析しません —
  デプロイ版での実測で見出し 11・そのまま出る `####` の漏れ 3、属性ありでは見出し 69・漏れ 0。
  `scripts/check_doc_metadata.py` が両方を検査します。— (details: quality-ledger (az))

- **wheel に同梱される型スタブが Python として解釈できる。** — maturin 1.14.1 は実行時の `__doc__`
  をトリルクォートへそのまま書き込むため、本プロジェクトの Rust doc comment 2 か所にバックスラッシュ
  があって `python/pyrs_yaml/pyrs_yaml.pyi` は unicode エスケープの切り出しで `ast.parse` に失敗して
  いました。`py.typed` と共にこのファイルがユーザーの mypy/pyright が読む型の契約であり、あらゆるド
  キュメント生成器もここから API に到達できません。漂移経路は docstring 内のバックスラッシュをエスケ
  ープし（箇所数を門に）、派生テキストの構文解析を要求します。— (details: quality-ledger (bb))

- **リリースノートが版全体を説明し、最初の 1 行だけではない。** — 折りたたみ履歴の各 `<summary>` が
  その版の最初のエントリだったため、読者は展開する価値があるか判断できません（v0.16.0 は 93 分の 1：
  Added 64・Fixed 14・Performance 8）。今は各サマリが版全体を要約し、折りたたまれていなかった 0.11.4
  ・ 0.11.3・0.1.0 も他と同じ形に揃え、5 ミラー同期で適用しました。— (details: quality-ledger (bc))

- **文書のみの pull request はマージできず、サイトをビルドする pull request もなかった。** — ブラン
  チ保護が要求するチェックは `Test matrix (all legs)` の 1 本だけで、それを出す workflow は `*.md`
  と `docs/**` をトリガから除外していた。PR #319 は出揃った検査がすべて緑なのに
  `Required status check … is expected` で拒否され、管理者強制が有効なので `--admin` も通らない。現
  在は `ci.yml` が毎回走り、`changes` が変更を分類して重い脚はその回答で条件化、`docs-gates` が毎回
  文書ゲートと 4 ロケールの `--strict` ビルドを実行する —— 登録済みの描画の盲点はそのように閉じた。
  — (details: quality-ledger (bd))

- **すべての wheel に同梱される型契約が、例外クラスを一つも宣言していなかった。** — パッケージは 10
  個のエラー型を export するのに `python/pyrs_yaml/pyrs_yaml.pyi` には一つも無い — maturin 1.14.1 の
  自省経路は PyO3 の `import_exception!` クラスを辿らないためです。それで
  `except pyrs_yaml.YamlParseError:` は mypy と pyright に見えませんでした。漂移経路はビルド済みのク
  ラスから宣言（名前・基底・docstring）を追記し、 `EXPECTED_EXCEPTION_CLASSES` で数を門にし、ファイ
  ルが構文解析できるよう基底を subclass より前に並べ、 import できないときは契約を薄くして通すのでは
  なく exit 2 で止まります。これにより `exceptions.md` も 4 ロケールすべてで自動生成になりました。—
  (details: quality-ledger (bf))

- **スタブは構文解析できるのに型チェックを通らなかった。** — mypy は
  `python/pyrs_yaml/pyrs_yaml.pyi` の**内側**で 5 件のエラーを出します（`u32` 未定義、`Callable` 未
  定義、`Py<PyAny>` の無効な注釈 3 件）。ユーザーのエディタではこれらの診断が本ライブラリ帰属にな
  り、AST レベルの門はすべて緑でした。maturin 1.14.1 が Rust 側の綴りを注釈へ写し `Callable` を
  import しないため、経路側で綴りを書き換え（箇所数 4 を門に）、注釈が参照する名前を `typing` の
  import に統合し、`scripts/check_stub_types.py` が CI で mypy と `ty` の両方に確認させます。`ty` は
  我らの保真テンプレートが書いた `-> dict` も指摘しました — `dict[Any, Any]` に修正済み。残る 32 件
  の厳格性指摘は記録のみで強制しません。この門はその後 CI の全テスト脚を赤にしました。足りな
  かったのは呼ぶはずのツール本体で、実行ファイルが無い環境では戻り値ではなく例外が出るためで
  す。今は確認不能という回答が exit 2 とスキップになり、合格にはなりません。
  この門自身の否定対照も同日に失効しました。`origin/main` を断言していたため、この修正が
  マージされた日の main のスタブはすでに正常だったのです。対照は欠陥を運んだコミットを名指しし
  （`v0.17.0` はフォールバック）、ファイルに想定どおりの綴りがあるかを確認したうえで、両方の
  チェッカーが存在する CI で実行されます。
  — (details: quality-ledger (bg))

- **組版修正ツールが自分のチェッカーに報告される隙間を書かなくなりました。** — ドキュメント追記で
  `check_doc_wrapping.py --fix` が `1 gap(s) [wrap residue]` を返し、収束しなくなりました。修正側は
  二つの物理行の間の結合を文字クラスで決めていましたが、そこに U+3001 は含まれません。それで `、` で
  終わる行と後続のかなの間に空白が入ります。同じファイルの残査クラスは当該コードポイントを含むため、
  その隙を欠陥として報告します。一つの規則を二通りに書いた結果でした。今は `join_lines` が二つの残査
  パターン自体に質問し、`、` の境界は融合します。— (details: quality-ledger (bi))

- **JSON5 の非有限浮動小数はハブ往復の両方向で型を保ちます。** — `from_json5` はハブ文書を返します
  が、その文末の両端が同じ 3 つの語について食い違っていました。YAML Core は `.inf` / `-.inf` /
  `.nan` を浮動小数、裸の `Infinity` を文字列と解決するため、原文の綴りを保存すると数値が文字列化
  し、投影を読み直す利用者はみな `'Infinity'` を受け取ります（`load_json5("{a: Infinity}")` はメモリ
  上で解決するので正しかった）。先に測ると報告は半分しか述べていませんでした。引用する側は逆の間違い
  方で、 `quoted_or_plain` がデコード済みの JSON 文字列の要否を `needs_quotes`（core schema の問い）
  に尋ねたため `"Infinity"` は平文スカラーに保存され、`load_json5('["Infinity"]')` は `[inf]` を返す
  一方、`load_jsonc`・ `load_json`・`json.loads` はいずれも `['Infinity']` を返しました。同じ文書で
  型が二つ、どのローダが読むかで決まっていたのです。出力側は解決後の *値* から方言の裸語を導き、ハブ
  は YAML 自身の綴りを保存し、 JSON5 レゾルバはパーサが渡す綴りを受け入れます。厳格 JSON と JSONC は
  綴れない非有限浮動小数を引き続き引用します。この判断は `ROADMAP.md` に開いたまま残し、バグ修正の中
  で内々に決めはしません。 — (details: quality-ledger (bh))

- **非 ASCII キーを含むルールパスがプロセスを停止させており、`$` も対象になれます。** —
  `rule_path_to_segments` はカーソルを 1 バイトずつ進めるのに文字は単位で押し込んでいたため、
  `$.café` は `é` の途中で止まり、次のスライスが "start byte index 4 is not a char boundary" で
  panic しました。正当なスキーマで到達できる `PanicException` です（`$.emoji😀key` も同じ）。現在は
  デコードした文字の幅だけ進みます。直して初めて後半も見えました：区切りを要求したあとで空かを
  検査していたため、裸の `$`（文書そのもの）はルール対象になれず、`$x` は解釈不能のままです。
  `path: $` に `mapping_of` を使うとルート文書の値が検査されます。panic を見つけた調査は
  `ROADMAP.md` に論点も残しました：`mapping_of` / `sequence_of` は要素を見て容器の種別を見ません。
  — (details: quality-ledger (bj))

- **ファジング層に schema 言語の標的が無く、クラッシュはまさにそこに住んでいました（PR #329）。** —
  六つの標的はすべてエンジンに文書だけを渡すもので、スキーマを渡すものはありませんでした。だからこそ
  `path: $.café` がプロセスを落とすコードがレビューの死角に置けたのです（ledger (bj)）：手書きの
  schema テストはすべて ASCII のキーなので、経路を辿るコードはどの層からも到達できませんでした。七つ
  目の標的は入力を最初の NUL で分割し、二つを `parse_schema_yaml` と `validate_node` に渡すので、1
  件のコーパスが二つの文法とその相互作用を同時に持ちます。種は実際にクラッシュした入力そのものなの
  で、週次のサンプリングだけでなく pull request 層（種子の決定的な `-runs=0` 再生）にも入ります。本
  当に欠陥を拾うかは推定ではなく実演済み：修正を戻すと再生で `schema_language.rs` のクラッシュ入力が
  1 件出て、修正があると同じ種はきれいに再生します（150 秒の探索も同様）。

#### パフォーマンス

- **タグ出力はテーブル駆動、エスケープはメモリを確保しない。** — タグ符号化を読み取り側の文字クラス
  に寄せたことで 1 バイトごとの所属判定が逐次化のホットパスに入り、128 項のコンパイル時テーブルにな
  りました（英数字判定も同じ参照に畳み込み）、`%XX` エスケープは 16 進表から書き、エスケープされる文
  字ごとに `String` を作っていた `format!` をやめました。— (details: perf — Leaderboard &
  Performance Status)

- **null キーが多い文書の解析が線形時間。** — null キーの畳み込みは null キーごとにマップを再走査し
  ていました。すべて null の文書では見えにくく（畳まれた項目は slot 0）、本質的な形では二次です：2k
  の別キーの後に 2k の null キーだと 4 倍入力で 12.4 倍増（8k + 8k で 99 ms）。マップは null キーの
  slot を記憶し、走査は正しさのフォールバックとして残します。— (details: quality-ledger (ae))

- **10% 未満の議論は確定済みの命令基準で決着。** — 2 つの runner ジョブは
  `serialize_block_scalars` を 1.44% 差で測る一方、同一バイナリの繰り返しは ±0.0005% です。runner の
  ノイズ以下の wall clock では何も決まらないため、基準は生成環境を記録し、`scripts/ir_gate.py` が
  異なる出所を注記します。 — (details: quality-ledger (am), (as))

### [v0.17.0] — 2026-10-01

<details markdown="1">
<summary>pyq 互換フラグ · JSONC/JSON5 入力 · diff/merge · Release 自動化</summary>

#### 追加

- **pyq CLI パリティフラグ** — `pyq fmt` に `--indent N`（ブロックインデント、既定 2）、`--width N`
  （スカラーのソフト折返し列幅、0で無効）、`--sort-keys`（シリアライザレベルの文書全体キーソー
  ト）、`-i/--inplace`（ファイルをその場で書き換え）を追加し、`pyrs-yaml-core::SerializeOptions` の
  全オプションを公開、Python CLI の `fmt --indent` と揃えました。`pyq to-json` に排他的な `--jsonc`
  / `--json5` 方言出力を追加し、`pyrs-json` のコメント保持・JSON5 表記シリアライザ
  （`to_jsonc_text*`、`to_json5_text*`）を接続。
- **pyq JSONC/JSON5 入力方言** — `Format` 列挙に `--input jsonc|json5` を追加（`.jsonc` / `.json5`
  拡張子の自動判定にも対応）。`pyrs-json` ネイティブ方言パーサを経由し、コメントや JSON5 表記は AST
  に載る。`to-json --jsonc` と組み合わせれば 1 コマンドでコメント保持の JSONC→JSONC 往復が可能。
  `--all-docs` は単一文書方言では安定したメッセージで拒否。
- **`pyq diff` / `pyq merge`** — ネイティブ CLI に意味的ドキュメント比較と右優先のディープマージ
  （yq `*+` 同型）を追加。`diff` は両 AST を走査し解決後の値・構造・タグを比較（コメント/引用/レイ
  アウトは現れない）し、`-`/`+`/`~` パス行を出力、一致 0・差異 1 で終了。`merge` は mapping を再帰
  的に統合し sequence は末尾追加（`--replace-arrays` で丸ごと置換）、ラウンドトリップ YAML で出力。
  両コマンドとも `--input`/拡張子判定で対応方言を読み込めます。

#### 変更

- **GitHub Release を `publish.yml` が自動作成** — これまで毎回 publish の
  後に手動で `gh release create` を実行しており、忘れやすい工程が 1 つ増え、
  公開済みバージョンと tag がずれる余地も生在じていました。`release` job は
  同じ `refs/tags/` 条件のまま `uv publish` 成功後に `gh release create` を
  実行し、リリースノートを自動生成してビルド済み wheel を添付します。ノート
  と成果物はどちらも PyPI 公開と同じ tag 由来です。`workflow_dispatch` 実行の
  挙動は変わらず（PyPI 公開も Release 作成も行わない）、既存動作と一致します。
- **`README.md` / `README.zh-CN.md` にネイティブ `pyq` CLI を追記** — 両
  README の Python CLI セクションの隣に `pyq` セクションを追加。チェックアウト
  からのインストール方法、実行可能なサンプル 3 つ、完全なコマンド一覧を記載し、
  詳細は pyq ガイドへ誘導します。

#### 修正

- **`pyrs-json` モジュールドキュメント** — 旧記述は「コメントは読み取り時
  に破棄され再出力されない」としていましたが、#122 以降は AST のコメント
  スロットに保持され、JSONC/JSON5 シリアライザが復元します。

</details>

### [v0.16.0] — 2026-10-01

<details markdown="1">
<summary>ライブラリ間パリティ · `load_json` · TOML 1.1 と toml-test · JSON5/JSONC · pyq · 高速化
</summary>

#### 追加

- **JSONC ブロックコメント・ホットスポットベンチ** — 目標 §テストカバレッジ 5 が「block-comment」を
  必須のホットサンプルに指定。以前はインライン `//` のみ計測。新フィクスチャで
  `test_load_jsonc_block_comments` を駆動：50 pair + header/footer、各項に独立 `/* item N */` と末尾
  `value /* trailing */` を持たせ、ブロック走査の回帰を CodSpeed で可視化。
- **YAML の PyYAML + ruamel.yaml 跨库パリティ** — 目標 §テストカバレッジ 3 が両ライブラリを oracle
  として名指し。以前は `test_benchmark_crosslib.py` のベンチ + 特性 support printout のみ。
  `tests/test_yaml_crosslib.py` で 20 正規ドキュメント × 5 パリティ面 + 2 ドキュメント化された
  divergence（duplicate-key 厳格性、YAML 1.1 従来 bool の schema-scope）= 122 テスト。オプション依存
  skipif で降格。
- **`load_toml` の tomlkit 跨库パリティ** — 目標 §テストカバレッジ 3 が tomlkit を oracle として名指
  し。以前はベンチのみ。`tests/test_toml_crosslib.py` に 24 ケット追加：11 の正規構造で pyrs /
  tomlkit / tomllib 三者一致、`>i64` 拒否を仕様準拠（TOML v1.0 §Integers：64bit signed）として固定、
  `-2^63` 境界（PR #174 修正）を確認。オプション依存、skipif で降格。
- **orjson を STRICT-JSON oracle に** — 目標 §テストカバレッジ 3「orjson と逐位比較」は今までベンチ
  のみ。`tests/test_json_crosslib.py` で正規ドキュメント 16 件の一致、非正規 12 件（コメント、末尾カ
  ンマ、シングルクォート、`NaN`/`Infinity`/`-Infinity`、16 進、先頭 0、`+.5`、`5.`）の両者拒否を断
  言。stdlib `json.loads` は `allow_nan=True` で裸リテラルを受理するため orjson が RFC 8259 oracle
  としてより厳格。オプション依存、`skipif` で降格。
- **CLI ↔ Binding 対等ゲート（`tests/test_cli_binding_parity.py`）** — Pillar 1 の「CLI と Python
  Binding 両端で同等機能」を宣言から実行可能な契約に昇格。CLI の登録コマンドを 18 個の固定リストと照
  合（cyclopts の `--help`/`-h`/`--version` 擬似コマンドは除外）し、各 `to-X` / `from-X` 動詞に
  `YamlDocument.to_X` / `from_X` / `load_X` の対応があることを確認。`load_*` 一族
  （json/jsonc/json5/toml）の四兄弟対称性を断言、編集・validate・compliance 動詞は LIVE Python API
  にマッピング。どちらかの表面の漂移は CI 失敗として顕在化。
- **`load_json` プロパティテスト + CodSpeed ベンチ** — Hypothesis
  （`test_load_json_matches_stdlib_json` と `test_load_json_matches_load_jsonc_on_strict_domain`）が
  生成されたすべての正規ドキュメントで STRICT loader と `json.loads` の一致、および両 loader の
  strict 領域での逐字一致を固定。高速経路の拡大や AST 経路の漂移はプロパティ失敗として顕在化する。3
  件の CodSpeed wall-time ベンチ（`test_load_json_large` / `_floats` / `_escapes`）は `load_jsonc`
  のサンプルをミラーし、STRICT binding 層自体を回帰追跡する。
- **`load_json`（厳格）— `load_*` 一族の対称性を完成** — binding は既に `load_jsonc` / `load_json5`
  / `load_toml` を持っていたが、厳格 RFC 8259 の対応関数が欠けていた。`pyrs_yaml.load_json(s)` は正
  規入力では `json.loads` と逐字一致し、JSONC/JSON5 拡張（`//`、`/* */`、末尾カンマ、シングルクォー
  ト、裸の `Infinity`/`NaN`、`0x…`）を型付き `YamlParseError` で拒否。高速経路は `load_jsonc` と
  `json_fast::try_load` を共有（非正規バイトはすべて bail、構文の拡大リスクはゼロ）；拒否対象は
  STRICT な `from_json` AST 経路に流れる。これにより下記の CLI ↔ Binding 対等宣言最後のギャップが埋
  まり、ピラー 1 が完成。`pyrs_yaml.__init__` から再エクスポートし `__all__` に追加；`.pyi` は
  `maturin generate-stubs` で再生成。
- **方言 writer の固定点プロパティ** — `fmt_pbt.rs` はヘッダで writer 固定点（writer 出力を再パース→
  再シリアライズすると逐字一致）を約束していたが未実装だった。4 つの proptest が
  JSON/JSONC/JSON5/TOML でこれを果たす（唯一の入力フィルタは別々のキーが同一の JSON 名になる手組み
  AST を除外——RFC 8259 の object 領域外）。このゲートで注釀忠実性の実バグ 3 件を即座に発見（下記の修
  正参照）。
- **ホットスポットベンチコーパス** — 7 件の CodSpeed wall-time ベンチが歴史的に脆弱なシリアライズ経
  路を狙う：YAML ブロックスカラー文書（6 種のヘッダ表記 `|`、`|-`、`|+`、`>`、`>-`、`>+`）とコメント
  密度文書、TOML マルチライン文字列/進数整数/アンダーセリエータ/指数/日付時、JSON5 の特殊数値形式
  （16 進、`+.1`、`5.`、`Infinity`、`NaN`、シングルクオート、末尾カンマ）。固定種は
  `tests/data/yaml_samples.py`、ベンチは `tests/test_benchmark_api.py`。このコーパス構築こそが下記の
  ネスト式ブロックスカラーのインデントバグを発見した。
- **テキストレベル再パースゲート（`prop_output_always_parses`）** — Rust proptest スイートは生成 AST
  のシリアライズ出力が常にパーサで再読込できることを主張。AST 同士の往復プロパティは再パース不能な形
  態（`try_roundtrip` が `None`）を黙って飛ばしていた。新ゲートは初回の実行で 6 個の実バグを検出し、
  それぞれ targeted Rust ユニットテストと Python 回帰クラス（`TestNestedBlockScalarIndent`）で固定。
- **toml-test 適合性ハーネス** — `tests/test_toml_test_suite.py` は公式
  [toml-test](https://github.com/toml-lang/toml-test) を `test_yaml_suite.py` が YAML スイートを実行
  するのと同じ方式で実行する。未追跡のローカル資産、欠損時 `skipif`、実測フロアのゲート、およびデコ
  ード比較用の型タグアダプタ。
- **TOML 時刻型の正しいデコード** — 日付のみと時刻のみは異なる `!date`/`!time` タグを持つようになり
  ました（日付時は `!timestamp` を維持）。素の時刻（`07:32:00`）、秒省略の時刻（`13:37`）、小文字区
  切りの日付時（`1987-07-05t17:45:00z`）がいずれも正当な TOML で `ValueError` を投げることを
  toml-test が発見。`!time` は省略秒を補い、`!timestamp` は小文字 `t`/`z` を正規化。
- **TOML 制御文字の厳格性** — 基本・字句・複数行文字列内でraw C0 制御コード（NUL、FF、DLE、US 等）と
  DEL（U+007F）を拒否するように（タブと複数行の改行のみ許可）。toml-test の `invalid/control` が 13
  件の誤受理文書を発見。コメント本体・bare CR チェックは後続項目。
- **TOML 数値リテラルの厳格性** — 先頭ゼロの 10 進数（`01`、`-01`）、基数接頭辞整数への符号
  （`+0x1F`、`-0b101` — `signed-int` は 10 進数のみ）、末尾/連続アンダースコア（`1_`、`1__0`）を拒
  否。toml-test の `invalid/integer`+`invalid/float` が 23 件の誤受理を発見（合計 71->48）。従来の
  「基数整数は符号可」は仕様違反。
- **TOML インラインテーブル鍵衝突の厳格性** — インラインテーブルは定義済みパスと同一・拡張・被覆の関
  係にある点線鍵（`{ a = 1, a.b = 2 }`、`{ a.b = 1, a.b.c = 2 }`）を拒否。兄弟パス
  （`{ a.b = 1, a.c = 2 }`）は許可。toml-test `invalid/inline-table` の duplicate-key/overwrite が検
  出（誤受理総数 48->39）。
- **TOML 非 ASCII 文字列のクラッシュ修正** — 基本および複数行基本文字列パーサがバイト単位で進み、マ
  ルチバイト入力（U+00A0 等）で文字の途中でスライスして panic していた。両ループとも文字単位で消費す
  るよう修正。toml-test で発見、#153 の単一行/JSON 版を補完。
- **フォーマットファジング + 堅牢性修正** — 新 `proptest` 属性テストが TOML/JSON/JSONC/JSON5 のパー
  サとライタをファジング（no-panic + 再解析可能性）。発見・修正：TOML と JSON 文字列パーサの中途文字
  スライス panic、および JSONC/JSON5 のインライン `//` コメントが後続の `,`/`}` を呑み込む不具合。
- **YAML merge/別名プロパティファジング** — 良形式のアンカー/別名/merge-key
  ドキュメント（単一別名、別名シーケンス、インラインマップ入りシーケンス、インラインマップ
  マージ、#166 が拒否するスカラー/null マージソース）を新たに生成し、自己参照アンカーと
  重複別名参照も含めます。従来全てのプロパティテストは `arb_custom_node()` を使って
  いましたが、これは `meta.anchor` のみを出し `Alias` ノードを作らないため、別名解決と
  マージ展開の経路（まさに #163/#166 の構造クラス）はプロセス内ファジングされていません
  でした。`prop_merge_alias_never_panics` はパース + マージ解決が panic やネイティブ
  スタック溢れなく完了し、解析できる木は再シリアライズ・再パースで安定であることを検証します。
- **CLI フォーマット対等性** — CLI に `to-toml`/`from-toml`、`to-jsonc`/`from-jsonc`、
  `to-json5`/`from-json5` を追加（既存の `to-json`/`from-json` を倣う）。バインディングが扱う全フォ
  ーマットがコマンドラインから利用可能に。
- **JSON 文字列エスケープ高速パス（性能）** — `load_jsonc` は 8 種の単純な 2 バイトエスケープをイン
  ライン復号し、ドキュメント全体を AST パスへ退避しなくなりました。エスケープ入り JSON は高速パスに
  乗ります（AST ルート比約 15 倍速）。値は `json.loads` と一致；`\u`・不正エスケープは引き続き AST
  パス経由。
- **JSON 浮動小数点高速パス（性能）** — `load_jsonc` は正規の浮動小数（小数・指数）をドキュメント全
  体を AST パスへ退避せず直接 Python オブジェクトへ解析。値は `json.loads` と完全一致（正しく丸めら
  れた parse）。新 bench がこの分岐を回帰監視する。
- **JSON 文字列シリアライズの高速化（性能）** — エスケープ不要な文字列は 1 回の `push_str` で一括コ
  ピーし、文字単位の UTF-8 再エンコードをやめる。文字列の多い `to_json` は約 35% 高速（41→27 ns/
  件）、出力はバイト単位同一。
- **`YamlDocument.to_toml()`** — ドキュメントは `to_json`/`to_jsonc`/`to_json5` に倣い AST から直接
  TOML を出力。`to_toml(doc.to_yaml())` のシリアライズ→再解析の往復が不要になり、出力はバイト単位同
  一。ライタ自体は `tomli_w` より約 4.3 倍速。
- **`to_json` ネイティブシリアライザ（性能）** — `YamlDocument.to_json` は `to_dict()` +
  `json.dumps` の二重変換をやめネイティブエンジンを使用。ASCII はバイト単位同一、約 10 倍速（1200 件
  ~1450µs→~120µs、`json.dumps` を上回る）。非 ASCII は `\uXXXX` でなく生の UTF-8
  （`to_jsonc`/`to_json5` と一致）、正当な JSON を維持。
- **JSON オブジェクトキー直接出力（性能）** — ライタはマッピングキーを出力バッファへ直接書き込み（キ
  ーごとの `String` 確保を廃止）。コンパクト `to_json` はさらに約 2 倍速（~120µs→~60µs）、バイト単位
  同一、シリアライズは現場で #2（orjson のみ上）。
- **JSON ロード高速パス** — `load_jsonc` は正規の strict JSON を `CustomNode` AST を経由せず直接
  Python オブジェクトへ変換する（実測で約 5-6 倍速、stdlib `json.loads` を上回る）。非正規入力（浮動
  小数・エスケープ・コメント・範囲外整数・末尾カンマ）は一般パスへ退避し、値とエラーは不変。
- **TOML マルチライン文字列の再現性** — TOML マルチライン文字列を `ScalarStyle::Literal` の YAML ブ
  ロックとして投射（テキストハブを往復でき）`to_toml` が `"""` ブロックとして再出力する。値はバイト
  単位で往復し出力は冪等、単一行文字列は単一行のまま。既存の `Literal` を再利用し AST 構造変更な
  し。
- **TOML ドキュメントレベルのコメント再現性** — `to_toml` がルートマッピングの先頭コメントを出力す
  るようになり、文書冒頭の独立した `# コメント` が TOML → ハブ → TOML の往復で保持されるようになりま
  した（JSON ライターの `emit_root_leading` に対応）。ネイティブ TOML 解析やコメントなし文書は影響な
  し。
- **JSON5 Unicode 識別子キー** — 引用なしオブジェクトキーが ASCII 限定をやめ、
  Unicode の `ID_Start` / `ID_Continue` 集合全体を受け入れる。`from_json5` /
  `load_json5` が `{ é: 1, 名: 2, हिन्दी: 3 }` を解析できる。rustc 自身の
  字句解析が使う `unicode-ident` テーブルでスクリプトごとに正確に適合（中間の
  合成文字含む）。JSON5 モード限定のため、厳密な `from_json` / `from_jsonc` は
  従来通り引用を要求。`\uXXXX` の孤立 UTF-16 サロゲートは引き続き拒否（Rust
  `String` では無損失で表現不可）。依存を 1 つ追加（`unicode-ident`）。
- **JSON5 Unicode 構造空白** — `from_json5` / `load_json5` が RFC 8259 の 4 つ（タブ / スペース / LF
  / CR）に JSON5 が加えた空白をトークン間の区切りとして受理する：垂直タブ、フォームフィード、NBSP
  （U+00A0）、全ての Unicode `Zs` 区切り、LS/PS 行終端（U+2028 / U+2029）、ZWNBSP（U+FEFF）。`std`
  の `char::is_whitespace`（JSON5 が空白としない NEL U+0085 を除く）に U+FEFF を足して実装、新規依存
  なし。JSON5 モード限定のため、厳密な `from_json` / `from_jsonc` は従来通り全てを拒否し、挙動は一バ
  イト変わらない。
- **JSON5 行継続と `\'` エスケープ** — 二重引用符 JSON5 文字列が 2 つの
  エスケープを受理：行終端直前のバックスラッシュ（行継続で両方を除去）、
  およびエスケープされた一重引用符（`\'` → `'`）。JSON5 モード限定のため、
  厳密な `from_json` / `from_jsonc` は従来通り両方を拒否。#125 の一重引用符
  処理を反映し、JSON5 文字列の再現性を完成。
- **JSON5 文字列エスケープ `\v` と `\0`** — `from_json5` が垂直タブ（`\v`）と NUL（`\0`）を二重/一
  重引用符文字列で受理。厳密 JSON / JSONC は従来通り拒否。#120（数値）・#124（数値意味）と合わせ
  JSON5 文法を完成。
- **load_json5 の JSON5 数値セマンティクス** — `load_json5` が JSON5 独特の数値形式（`0x1F`→31、
  `+7`→7、`5.`→5.0、`Infinity`/ `NaN`）を新しい `Schema::Json5` で実数として解決。厳密 JSON / JSONC
  ローダは不変、`to_json5_text` は元の表記のまま出力。
- **JSON5 / JSONC を公開 API から利用可能に** — `pyrs_yaml.from_json5` / `load_json5`、および
  `YamlDocument.to_jsonc()` / `to_json5()`（ネイティブエンジン経由でコメントと JSON5 スタイルを保
  持）。同時に到達性の欠陥を修正：`from_jsonc` / `load_jsonc` が `pyrs_yaml` パッケージに再エクスポ
  ートされておらず `AttributeError` になっていたが、`__all__` に追加。`to_jsonc`/`to_json5` は
  `emit_root_leading` でドキュメントレベルの standalone コメントを保持。`test_benchmark_api.py` に
  JSON 系のベンチマークを追加。
- **JSON5 ライター（`to_json5_text` / `to_json5_text_pretty`）** — 契約 B ステップ 2。AST を JSON5
  に再帰列化し、パーサーが保持する一重引用符文字列と `0x…`/`.5`/`+7`/`Infinity`/`NaN` の数値形式、
  および `//` コメントを復元。キーは常に引用符付き（損失なし）。内部では `Mode`（Json/Jsonc/Json5）
  を共用、厳密 / JSONC 出力は不変。
- **パーサーに JSON5 数値形式を追加** — `from_json5`（新規 `allow_json5_numbers`）が十六進
  （`0xDECAF`）、前/後小数点（`.5`、`5.`）、前置 `+`（`+7`）、先頭ゼロ（`07`）、素の `Infinity` /
  `NaN` / `-Infinity` を受理。各形式は原文を保持し、将来の JSON5 ライターが再現できます。STRICT /
  JSONC はこの軸を OFF のまま従来通り拒否。`from_jsonc` の旧い「コメントは破棄される」doc を修正。
- **TOML インラインテーブル内部コメント保真** — PR #119 はインラインテーブル内の `# ...` を捕捉し
  （メンバー上の独立行 → leading、値の同じ行の後 → trailing）、IR を通して往復させます。装飾のない
  インラインテーブルはコンパクトな一行形式を維持し、配列にネストされた装飾付きテーブルは複数行に昇
  格します。同時に #114 の潜在バグ（`skip_all_blank` が standalone コメント自身の改行を空行と誤認）
  を修正。
- **YAML レシーバーが standalone コメントを `decor.leading_comment` に書く** — PR #117b で最後のエン
  ジン（granit-parser レシーバー）が #114 / #115 で導入した新しいスロットに移行しました。scalar /
  mapping / sequence の standalone note が `NodeMeta::decor.leading_comment` に載り、古い
  `comment(standalone = true)` ではなくなります。#117 の正規化で hand-built fixture は引き続き等価、
  かつ `CustomNode ::remove_comment` が**両スロット**をアトミックにクリアするので Python の
  `Node.remove_comment()` は YAML 起源ドキュメントでも従来通り動作します。
- **スロット横断の standalone 正規化 + Python `Node.leading_comment`** — `NodeMeta::eq` / `Hash` は
  standalone コメントを新しい `leading_comment` スロット（TOML / JSON エンジンが使用）と古い
  `comment(standalone = true)` スロット（YAML レシーバーと hand-built fixture が現在も使用中）のど
  ちらにあっても同一概念として扱います。setter / remover は両スロットにアトミックに作用し、YAML シ
  リアライザは正規化されたビューを読むので `to_yaml(toml_ast)` の leading note が失われません。
  Python の `Node.leading_comment` getter / setter / remover は `Node.comment` をミラーし、TOML /
  JSONC ソースの standalone note を初めて Python 側に露出します。
- **TOML 1.1.0 文法** — `from_toml` が TOML v1.1.0（2025-12-18 公開）を解析します。四つの追加：
  **(A1)** インラインテーブルの改行と後尾カンマ許容、**(A2)** 基本文字列 `\xHH` バイトエスケープ
  （0x00..=0xFF）、**(A3)** `\e` = U+001B、**(A4)** time / date-time の秒省略（`t = 14:15` /
  `dt = 2010-02-03 14:15`）。厳密 1.0.0 用のエスケープハッチとして `TomlDialect::V1_0` と
  `from_toml_v1_0` を保持。1.0.0 ドキュメントは両方言で同じ解析結果になります。同時に space 区切り
  date-time 検出の off-by-one 索引バグ（`T` 無し date-time が 1.0 モードでも認識されなかった）を修
  正。
- **JSON dual-slot コメント保真** — JSONC パーサーは独立行の `// ...` を #114 が追加した
  `leading_comment` スロットに書き、同じ行の `// trailing` は `comment` に残します。オブジェクトメ
  ンバーや配列要素が同じノードで両方のコメントを保持でき、#112 の単一スロットモデルでは表現不能な形
  状が可能になります。`to_jsonc_text_pretty` は `leading_comment` を優先読みし、hand-built fixture
  向けに `comment` (`standalone = true`) の fallback を保持します。厳密 JSON (`to_json_text`) の挙動
  は変化しません（依然 `//` を出力しない）。
- **TOML 空行と dual-slot コメント保真** — `NodeMeta` に `leading_comment: Option<Comment>` と
  `blank_before: bool` を追加（どちらも構造的な `Hash` / `PartialEq` から除外）。section ヘッダーや
  AOT 要素が `]` の後ろにある行末コメントと、その上にある独立行の先頭コメントを同時保持できるように
  なりました。`to_toml` は元の空行区切りを再現します（`a = 1\n\nb = 2` は byte-stable に往復）。手構
  築ノードと YAML 由来ノードは writer の fallback 読み取りでそのままレンダリングできます。
- **JSON5 方言** — `pyrs_yaml_core::json::from_json5(text)` と
  `from_json_with_options(text, JsonParseOptions)` は JSON5 の全 4 軸（後尾カンマ、一重引用符文字
  列、引用符なし識別子キー、行/ブロックコメント）を受け入れます。各軸は個別に ON/OFF 可。`STRICT` ・
  `JSONC`・`JSON5` 定数をデフォルトとして提供します。
- **JSONC/JSON5 バインディングと CLI** — `pyrs_yaml.from_jsonc(str)` は YAML テキストを返し、
  `pyrs_yaml.load_jsonc(str)` は Python の dict / list を直接返します。`pyq from-json` に `--jsonc`
  と `--json5` フラグを追加し、`tsconfig.json` / `settings.json` を verb パイプラインに直通させま
  す。
- **JSONC コメント保持** — `from_jsonc` が拾った `// 行` と `/* ブロック */` コメントを AST の
  `NodeMeta::comment` に添付します（独立部は key ノード、行末部は value ノード）。PR #109 で導入し
  た TOML モデルと対応します。対となる `to_jsonc_text(node)` / `to_jsonc_text_pretty(node, indent)`
  が元の位置へ再出力します。ブロックコメントは AST には本文だけを保存するため、出力時には `//` に正
  規化されます。厳密 writer `to_json_text` / `to_json_text_pretty` はバイト単位で不変なので、消費側
  は保真を任意に選択できます。
- **pyq マルチドキュメント編集** — `-A/--all-docs` が全編集コマンド
  （set/delete/rename/move/append/insert/sort-keys）と `to-json -A`（JSON 配列、Python 対応）をカバ
  ー。各ドキュメントはストリーム内の自身のセグメントに対して splice（`MultiDocEditor` +
  `DirtyUnit::shifted`）：触れていないドキュメントと全ての `---` 区切り行はバイト単位で保持、パス不
  一致のドキュメントはスキップ（Python の try/skip 意味と一致、全不一致はエラー）、レイアウト異常の
  ドキュメントは単独でフォールバックし近隣を巻き込まない。
- **JSONC パース** — `pyrs_yaml_core::json::from_jsonc(text)` と
  `from_json_with_options(text, JsonParseOptions)` は、ホワイトスペースが許される任意的位置で
  `// 行` と `/* ブロック */` のコメントを受
    け入れます（TypeScript の `tsconfig.json` や VS Code の `settings.json` で使われる方言）。コメン
    トは除去され保持されません。末尾カンマや JSON5 固有の構文は引き続き拒否されるため、受理される言
    語は RFC 8259 の厳密な上位集合のままです。`from_json` の既定動作（モード）は変わりません。
- **TOML コメント忠実性** — パーサーがペアやセクションヘッダーの上に独立した行で現れる `# ...` コメ
  ントと、行末コメント (`key = value # ...` / `[name] # ...`) の両方をキャプチャし、共有 AST の
  `NodeMeta::comment` に添付します (独立部は key ノード、行末部は value ノード)。
  `to_toml(from_toml(src))` が元の位置に再出力するため、`pyq edit` や `YamlDocument.set()` は TOML
  往返中の注釈を剥がさなくなりました。空行区切りは設計文書に従い writer の既定レイアウトのまま。
- **TOML 数値ソース表記の忠実性** — `to_toml(from_toml(src))` が 16 進 (`0xDEADBEEF`)・8 進
  (`0o755`) 整数のソース表記と指数形浮動小数点 (`1e10`、`-3.14e-2`) をそのまま保持します。区切り `_`
  ・明示的な `+` 符号・負の radix 形 (`-0x1F`)・2 進数 (`0b101`) は YAML Core が再読込みできないた
  め 10 進に正規化されます。これにより共有 AST と YAML パイプラインの相互運用性を維持します。コメン
  ト忠実性と JSONC は設計ドキュメントに従い後続 PR で実装予定。
- **pyq 機能補完** — CLI が Python CLI の機能面に追従：`rename`/`move`/`append`/`insert`
  の splice 編集、`validate`（構文チェック、または `--schema rules.yaml` による
  スキーマ言語ルール検証）、`frontmatter`（`--body-out` で本文分割）、
  `get`/`fmt`/`to-json` の `-A/--all-docs` マルチドキュメント対応。配線中にコア
  エンジンのバグを発見：`move_path` が移動先 INSERT ユニットのみ返し、splice
  テキストに移動元サブツリーが残存（フォールバック時は invisible）。両ユニットを
  返し bindings はバッチ splice 経路で適用する仕様に修正。
- **`pyq` 絞り込み動詞** — マッチ列への jq スタイル構造化後処理： `--select 'PATH OP LITERAL'`、
  `--sort-by PATH` / `--desc`、`--unique`、 `--first` / `--last`、`--skip N` / `--take N`、
  `--join SEP`。 `get` と `from-*` で固定パイプライン `select -> sort -> unique -> slice` → `join`
  を適用。意図的にフラグ設計（式言語なし）：述語は微細構文 1 パース（約 40 行）、型不一致は false
  （jq の全順序との既知差）、起動は瞬時。
- **`pyq completion`** — bash・zsh・fish・PowerShell のシェル補完スクリプトを
  出力（`pyq completion bash > ...`）。`clap_complete` 実装（承認済みの CLI
  クレート依存追加。`pyrs-yaml-cli` バイナリ内に完結し Python 配布に影響せず）。
- **`pyq sort-keys`** — 任意パス（`$` はルート）のMapping キーを並び替え。
  `set`/`delete` と同じコア plan/splice エンジン経由で、その場書き換えにも
  標準出力にも対応し、Python CLI の `sort-keys` との対応差を解消。
- **コマンドラインインターフェース** — 新しい `pyrs-yaml` コマンド（ `pip install "pyrs-yaml[cli]"`
  でオプトイン、Python 3.10+ 必須）により、ライブラリの中核機能をターミナルから利用できます：`fmt`
  （コメント・アンカー・順序を保持するラウンドトリップ整形）、`get`（JSONPath クエリ、
  `--format yaml|json|text` 対応）、`set` / `delete` / `rename`（パスベースの編集、`--inplace`・
  `--string`・ `--create-missing` をサポート）、`validate`（CI フレンドリーな終了コード）、および
  `to-json` / `from-json` 変換。すべてのコマンドは `-` で stdin を読み、デフォルトで stdout に出力
  します。実装は純粋な Python（`python/pyrs_yaml/cli/`）で、
  [Cyclopts](https://github.com/BrianPugh/cyclopts) をオプション extra として使用するため、ベースイ
  ンストールは追加依存ゼロと Python 3.8 サポートを維持します。
- **CLI 拡張** — `sort-keys`（パス位置のマッピングキーをソート）、`move`
  （サブツリーを既存の宛先へ移動）、`frontmatter`（Markdown フロントマターを YAML で
  抽出、本文の分割も可能）、`compliance`（YAML Test Suite レポート、`--json` 対応）
  コマンドを追加。`fmt`/`get`/`set`/`delete`/`rename`/`sort-keys`/`validate`/
  `to-json` に `-A/--all-docs` マルチドキュメントモードを提供し、`validate` は相互排他の
  `--schema <名前>` と `--schema-file <パス>` に分割しました。未文書だった `python -m
  pyrs_yaml.compliance` エントリポイントはサブコマンドに置き換えられ削除されました。
- **`YamlStream` が import 可能に** — API リファレンスと型スタブどおり
  `from pyrs_yaml import YamlStream` が使えるようになりました。これまでこのクラスは
  `YAML().load_stream*()` の戻り値としてのみ得られ、ネイティブモジュールから
  エクスポートされていませんでした。
- **CLI `move --all-docs`** — `move` が `-A/--all-docs` をサポート。両方のパスが解決できる
  各ドキュメントでサブツリー移動を適用します（`set`/`delete`/`rename` と同じセマンティクス）。
  これによりマルチドキュメントフラグはすべての編集コマンドをカバーします。
- **ドキュメント↔API 整合性ガード** — `tests/test_docs_api.py` が全言語の
  ドキュメントページ内の `pyrs_yaml.…` 属性チェーン・`import pyrs_yaml…`・
  `from pyrs_yaml … import …` の参照を走査し、実行時に存在しないシンボルを
  参照していれば失敗します（約 965 件の宣言を検査）。
- **オプションのサードパーティタイププラグイン** — `!duration`（`pendulum.Duration`）、
  `!arrow`（`arrow.Arrow`）、`!ulid`（`ulid.ULID`）は、対応するライブラリがインストール
  されている場合に自動登録されます（`python/pyrs_yaml/plugins/_builtin.py` の
  `_register_third_party`）。各プラグインは独立したタグを使用するため、既存の
  `!timestamp` / `!date` / `!uuid` ハンドラには影響しません。標準ライブラリの
  `timedelta` が `!duration` にマッチすることはありません。
- **pydantic-settings の YAML ソース** — `PyrsYamlConfigSettingsSource`
  （`python/pyrs_yaml/settings.py`）は `pydantic_settings.YamlConfigSettingsSource` の
  ドロップイン代替で、PyYAML の代わりに pyrs-yaml（YAML 1.2 コアスキーマ）で解析します。
  遅延エクスポートされるため `import pyrs_yaml` に pydantic-settings は不要です。
  `pip install "pyrs-yaml[settings]"` でインストールします（Python 3.10+）。
  `dump_pydantic` と `parse_as` も同じモジュールレベル `__getattr__` の遅延エクスポートに変更されま
  した。
- **`pyq` — Rust ネイティブ CLI クレート** — `crates/pyrs-yaml-cli`（ワークスペースメンバー、clap ベ
  ース）は `pyrs-yaml-core` を jq/yq スタイルの CLI に直接接続し、実行時に Python を不要にします：
  `fmt`（コメント保持のラウンドトリップ）、`get <path>`（JSONPath-lite、`--json`/`--raw` 対応）、
  `set <path> <value>` と `delete <path>`（yq 風の編集、`--create-missing` と `-i/--inplace` 対応、
  出力はラウンドトリップシリアライザ経由でコメントと値のスタイルを保持）、`to-json`（順序保持）、
  `to-toml`、導入コマンド `from-json` / `from-toml` / `from-ini`。入力形式は拡張子で判定（`--input`
  で上書き）、stdin は `-`、失敗時は core の安定したエラーテキストで非ゼロ終了。
- **TOML と INI の交換フォーマット** — YAML を唯一の編集可能表現とするハブ・スポーク構成のマルチフ
  ォーマット対応：`from_toml`/`to_toml` で TOML テキスト ⇄ YAML テキストを変換（Rust `toml_edit`）、
  `load_toml` は TOML を Python 値へ直接読み出し（datetime は内蔵 `!timestamp` プラグイン経由、TOML
  文字列は再解決されない）、`load_ini` は標準ライブラリ configparser で INI を読み取り（厳格モード・
  読み取り専用）。TOML 出力は表現不能な構造を安定したエラーで拒否。ラウンドトリップ編集は YAML 限
  定。

#### 変更

- **granit-parser 1.1 → 1.3** — YAML イベントパーサを 1.1.0 から 1.3.0 へバンプしました。1.x 系列内
  でのセムバー互換のマイナー升级です：1.2.0 は特殊なドキュメントの制限向けのオプションの `Options`
  フィールドを追加し、1.2.1 はいくつかの解析結果を YAML 仕様に合わせて厳密化し、1.3.0 はデフォルト
  実装付きの 2 つの `Input` メソッド（`fetch_block_scalar_line` と
  `take_quoted_scalar_ascii_chunk`）を追加して、スキャナがブロック・引用符付きスカラーのバイトをよ
  り速く進めるようにしました。本プロジェクトは `Parser::new_from_str` 経由でパーサを消費し、
  `EventReceiver` / `SpannedEventReceiver` のみを実装して `Input` は実装しないため、ソース変更は不
  要でした——新規の trait メソッドはデフォルト実装に解決されます。全テスト正常：
  `cargo nextest run --all`（359）、`pytest`（1436 + 43 numpy）、純 Rust `--no-default-features` ビ
  ルド、および YAML テストスイートの適合ゲートは不変。
- **ネイティブ JSON / TOML コア** — `serde_json` と `toml_edit` 依存を完全に
  削除しました。`pyrs-yaml-core` は RFC 8259 に準拠した JSON エンジン（バイトレベル
  スキャナ、数字はソース表記を保持するため `from_json → to_json` がバイト安定で、
  大きな整数/浮動小数点の精度も失われません。型付き行/列エラー、末尾カンマ・先頭ゼロ・
  単独サロゲート・未エスケープ制御文字・複数ルート文書を厳格に拒否）と、 TOML 1.0
  全文法をカバーするエンジン（bare/quoted/dotted キー、basic/literal/マルチライン
  文字列、区切り `_` を許す 10/16/8/2 進整数、`inf`/`nan`/指数付き float、
  offset/local の日付・時刻・日時）を内蔵します。すべての拒否は granit-parser と同じ
  スタイルで 0-indexed の `line`/`col` を携えた `ParseError::Syntax` として
  報告されます。公開 API は不変、ラウンドトリップテストと `tests/test_toml.py` は
  新しいエンジンでグリーン。
- **内部の重複コード整理** — ベンチマーク fixture を共有ブロックの合成に変更、PyO3 の
  パス編集メソッドを既存の `apply_metadata_edit` ヘルパーへ委譲、重複したファイル読み込み/
  エラーマッピングと行オフセットの定型コードを共有関数に集約しました。公開動作の変更は
  ありません。jscpd で測った重複コード率は 5.25% から 3.45% に低下。
- **`YamlDocument.validate()` がコンパイル済み validator をキャッシュ** — スキーマ
  （JSON テキストまたは dict）に対する初回検証成功時に `jsonschema` validator を
  キャッシュし、以降の呼び出しではスキーマ解析・メタスキーマチェック・validator
  構築をスキップ。dict キーはオブジェクト同一性＋ディープコピー・スナップショット
  ガードで管理され、その場の変更は `==` で検出され透過的に再コンパイル。キャッシュ
  経路は `exceptions.best_match(validator.iter_errors(instance))` を送出し、
  `jsonschema.validate()` と完全に同一のセマンティクス。WSL 実測: `document_validate` −98%。
- **パーサー/シリアライザーカーネルの構造的重複排除** — mapping と sequence の
  レンダリングを単一の `write_container_node` スケルトンに統一（出力はバイト単位
  同一、`serialize_*` 中央値 −5~11%）；単一/複数文書パーサーは同一の `load_ast`
  エラー契約を共有；schema 解析チェーンは `bool_word`/`numeric_tail` を共有し、
  YAML 1.1 は core の null/bool 語をスカラーごとに再チェックしない；アンカー登録
  （`register_anchor`）と standalone/inline 注釈分類（`is_standalone_placement`）を
  AST と stream receiver で単一化。リポジトリ重複率 3.38% → 2.60%。

#### 修正

- **`\u` / `\x` エスケープ直後のマルチバイト文字でパーサが panic** — 固定長エスケープ読み取りが
  `&self.text[pos..pos+width]` をバイトオフセットで切っていた。JSON `\u` / TOML
  `\xHH`/`\uXXXX`/`\UXXXX` の後にマルチバイト文字が来るとスライスが文字途中に落ちプロセス abort
  （#153 の同族）。バイト切り＋UTF-8 検証に直し、不正エスケープはクリーンにエラー。方言 fuzz で発
  見、両パーサに決定的 Rust 回帰テストで固定。
- **非マージ値のリテラル `<<` キーが黙って破棄されていた** — `load(safe_dump({"<<": None}))` はキー
  を失い `{}` を返した。マージ解決器は Null/Scalar 値でも全ての `<<` をマージとして消費していた。
  YAML では `<<` は値がマッピングのエイリアス / インラインマッピング / そのシーケンスのときのみマー
  ジ。Null/プレーンスカラーの `<<`、およびエイリアスを含まず合成内容が無い `<<`（`<<: []`、
  `<<: [1, 2]`、`<<: {}`）は通常のキーとして往復保持される。Alias/mapping/sequence 経路（#166 の自己
  参照アンカーガード含む）は無変更、yaml-test-suite は 405/406 維持。往復プロパティ fuzz が非決定に
  検出（#163/#165/#166 級の欠陥）、決定的な Rust 回帰テストで固定。
- **TOML 深いネストがネイティブスタックを溢れプロセスを abort** — TOML パーサにはネスト予算が無く
  （JSON は `DEFAULT_MAX_DEPTH`、YAML は `parse` の `max_depth`）、`parse_value` →
  `parse_array`/`parse_inline_table` が無制限に再帰。深い配列/インラインテーブルは解釈器を即クラッシ
  ュ（検証済み：exit `0xC00000FD` STACK_OVERFLOW）— #166 の YAML merge オーバーフローの TOML 版。パ
  ーサは `depth` を追跡し 1000 超で `ParseError::MaxDepthExceeded` を返し JSON と対称に。プロセス内
  Python 境界テスト + subprocess クラッシュカナリア + 大スタック Rust テストで守る。
- **方言 writer/parser がドキュメントレベルの注釀を消失・誤配置** — 固定点プロパティが捕捉した 3 欠
  陥：(a) JSONC/JSON5 の値より前の file-leading `// note` が inline と誤分類（オフセット 0 の前に改
  行なし）され、writer が注釈する root ではなく最初の object メンバに取られ、空 `{}` や root スカラ
  ーでは完全に消失；(b) JSON 系・TOML writer は注釀本体を無加工で出力していたが parser は trim して
  保存するため、未 trim 注釀は pass 毎に行末空白振動——writer も出力時 trim し初回スペルから安定；(c)
  注釀のみ TOML 文書（`# note` 後に key なし）は再パースで注釀が落ち空 root が `""` 化——未消費の
  standalone 注釀を空 root テーブルに付加。5 フォーマットの leading 注釀がすべて逐字安定の固定点到達
  （3 件の targeted Rust テストで固定）。
- **ネスト式ブロックスカラー本体が親行のインデントを保持** — ネストキー下の字句/畳み込みスカラーが本
  体行を列 0 から固定 1 段で出力しており、`b: |` ヘッダ行の下一層に配置されず、すべてのネスト形態
  （ペア・シーケンス項・compact dash・任意深度）の出力が再パース不能または誤値になっていた。スカラー
  出力系は `block_base`（親行の列位置）を全書出箇所に貫流。7 種のネスト形態で往復が完全一致。TOML ホ
  ットスポットベンチが共有シリアライザ経由で発見。
- **シリアライザが再パース可能な YAML のみ出力** — テキストレベルゲートが検出した 5 つの欠陥：(a) 改
  行分岐での anchor/tag プリエミットが子ノード（スカラー/null/flow 容器）の自己ヘッダと二重化
  （`A: !a` … `!a null`）→ block 容器のみに限定；(b) flow 容器内のブロックスカラー（`[|`、`{k: >}`）
  とキー位置はダブルクォートに降格；(c) 自行を開始する flow 容器の行頭インデント欠落と complex key
  （`?`）の値標識 `:` の列 0 出力→いずれも親インデントに従うよう修正；(d) standalone コメント/tag 付
  き complex キーの曖昧テキスト（コメントは `?` 上部へ、本文は常に 1 段深い独立行へ）；(e) flow 容器
  内で先頭/末尾空白または `,[]`{} 埋め込みの plain スカラーはクォート化（素文出力だと token を途切れ
  るか再パースで消える）。さらに tag 付き空 block 容器はヘッダを `{}`/`[]` と同列化、compact dash 項
  は standalone コメント付き値をインライン化しない。9 個の targeted Rust テストと Python 回帰で各系
  統を固定。
- **TOML が合法な最小 i64 整数を拒否** — `from_toml`/`load_toml` が `-9223372036854775808`
  （`i64::MIN`）で失敗：符号付き経路は絶対値を先に解析するため反転前に溢れていた。現在符号は桁と同
  時に解析され（`i64::from_str` は負方向に累積）、符号付き浮動小数は指数スペルを保ち、旧 negate 経
  路は削除。新設の Python 側 Hypothesis 方言ファジング（`tests/test_property_dialects.py`、stdlib
  `json`/`tomllib`/`pyjson5` をオラクルとする型厳密比較で発見；JSON5 のベアラ `Infinity`/`NaN` と
  i64 超過の数値文字列という AST 曖昧スペル 2 種も固定）。Rust 回帰：
  `toml::parser::tests::i64_lower_bound_negative_integer_is_accepted`。
- **誤ってインデントされたフロースケンスの続き行が再び拒否される** — YAML パーサを granit-parser 1.3
  へアップグレードすると（*変更*を参照）、続き行のインデントが enclosing block key よりも浅い複数行
  フロアコレクション（yaml-test-suite `9C9N`：`flow: [a,` の次に行頭 column 0 の `b,`）を静かに *受
  理*するようになり、厳密性が `405/406 → 404/406` へ後退した — suite の ≥95% しきい値ゲートには見え
  ないため緑 CI を通過した。AST receiver のパース後 in-tree ガードが enclosing block のインデントを
  追跡し、インデントが足りないフロア続き行を拒否して `405/406` を回復する。ガードはパーサが既に計算
  済みの span のみを使うため、正しくインデントされた複数行フロアは影響を受けない。`9C9N` は
  `tests/test_yaml_suite.py` にケース単位のハードゲート（リテラル入力、`skipif` なし）として固定さ
  れ、加えて Rust ユニットテスト（`parser::tests::flow_continuation_under_indented_is_rejected`）を
  追加。
- **自己参照マージキーがネイティブスタックを溢れさせなくなった** — 展開が自分
  自身のアンカー（`a: &a` に `b: {<<: *a}`）へ戻る `<<` は `resolve_merge_keys` 内で
  無限に展開され、ネイティブスタックを枯渇させてインタプリタプロセス全体を
  落としていた（Windows 終了コード `0xC00000FD`、セグメンテーションフォールト）。
  循環ガードはマージ対の*収集*時のみ適用され、展開の*走査*時には適用されなかった
  ため、再帰的再進入は一度も検出されなかった。アンカーガードは別名展開と同様に
  パススコープ化：アンカー名はその展開が走査されている間パスにとどまり、既にその
  パス上の祖先へ戻るマージは再帰せず空の展開として終端する。非循環 AST は PyYAML の
  循環 dict を持てないため、自己マージはクラッシュせず `{}` に収まる。同じ変更で
  4 つの関連マージセマンティクス欠陥も修正：null / スカラー / 配列のマージソースは
  リテラル `<<` キーとして残らなくなり、マージ値として直接使われるインラインマップ
  （`<<: {x: 1}`）はマージされ、マージ配列内の非別名要素（`<<: [*a, {y: 2}]`）は
  インラインマップを保持する。Rust 6 件・Python 9 件の回帰テストでカバー
  （`merge::tests`、`tests/test_gaps.py::TestSelfReferentialMerge166`）。
  [@bourumir-wyngs](https://github.com/bourumir-wyngs) による #166 の報告。
- **NumPy シリアライズが GIL 無しで Python メモリを読まなくなった** — ndarray ライタは
  `unsafe { as_slice() }` で配列のデータバッファを借用し、その借用スライスを `py.detach` の**内側**
  （＝ GIL 解放後）に反復していた。`&[T]` は出所によらず `Send` なので借用チェッカーは捕捉できない
  が、このメモリは Python 所有であり他スレッドが並行して resize / 書き込みしうる：不健全なデータ競
  合 / UB で、並行時のみ顕在化。バッファは**GIL を保持したまま** Rust 所有メモリへスナップショット
  （`slice.to_vec()`）するようになり、スカラー→ノード変換のみをスレッド外で行う。bindings 層で唯一
  の `unsafe` バッファ借用であり、他の全 `py.detach` Closure は Rust 所有状態（AST、ソーステキス
  ト、`BufWriter<File>`）のみ触及と確認済み。回帰カバレッジは
  `tests/test_numpy.py::TestNumpyConcurrency`。[@bourumir-wyngs](https://github.com/bourumir-wyngs)
  による #165 の報告。
- **繰り返しエイリアス参照が `None` にならないよう修正** — `to_dict()` は**グローバル**
  な visited 集合の後でエイリアスを展開し、クリアすることがなかったため、各アンカーの
  **最初の**参照のみが値になり、それ以降は無言で `None` に劣化していた：
    ```yaml a: &x 1 b: *x      # 1 c: *x      # 従来は None、現在は 1```
    影響範囲は「2 回目の参照」より広く、同一コンテナ内の兄弟参照同士も汚染し合った
    （`{a: &x {p: 1}, b: {q: *x}, c: {q: *x}}` で `b` は値、`c` は `None`）。この guard を現在の再帰
    パスに限定し、1 回の展開中のみ push してその後 pop するようになったため、繰り返し参照と兄弟参照
    はそれぞれ完全に構築された値を得、真の循環は依然として終了する。`<<` マージ解決と AST 自体は影
    響を受けないことを確認済み。`tests/test_direct_load.py` に PyYAML との一致を固定する 6 例を追加
    し、誤った出力を期待値として固定していた 2 テストを書き直した。
    [@bourumir-wyngs](https://github.com/bourumir-wyngs) による #163 の報告。
- **第一キーの値がネストコンテナの場合のドキュメントヘッダーコメント消失を修正** —
  パーサーは進行中の全コンテナで単一のコメントスロットを共有しており、ネスト
  コンテナの start が未配置の standalone ヘッダーを消していた（parse 段階で
  破棄、`to_dict` では見えず `dump` で顕在化）。コンテナごとのスタック管理に変更。
- **splice 編集での先頭コメント重複を修正** — 再生成されたリージョンテキストが
  pair/item 自身の standalone コメントを含む場合、置換範囲が旧コメント行を
  覆わず両方が残っていた。plan がコメント行へ範囲を拡張（`pyq set`/`delete`
  と bindings の splice 経路が共通で修正）。
- **`!timestamp` がサポート全 Python で末尾 `Z` を受理** — `datetime.fromisoformat` は UTC-`Z` 接尾
  辞を 3.11 以降でしか認識せず、プラグインで `...Z` を `+00:00` に正規化し、3.8～3.10 における YAML
  `!timestamp` スカラーおよび `load_toml` / `from_toml` 経由の TOML datetime の
  `Invalid isoformat string` を解消。

#### パフォーマンス

- **イベントストリーム→Python オブジェクトの直接構築** — `safe_load`、`safe_loads`、
  `YAML().safe_load*` は完全な AST を構築してから `convert.rs` で再走査する代わりに、granit イベント
  ストリームを一度走査して Python オブジェクトを構築する。schema 解決・原文マップキー・重複キーエラ
  ーの意味論は完全に一致。アンカー/tag/merge/マルチドキュメントはゼロコストの事前除外で AST 経路にフ
  ォールバック。WSL 実測：スカラー中心の `safe_load` −21~25%、ファミリー全体 −13~18%、フォールバック
  は変化なし。
- **アンカー抽出のバイトゲート** — `extract_anchors` は `&` のバイト含有チェック 1 回で
  空を返し、アンカーなしドキュメントでは文字単位のクォート状態機械を完全にスキップ。
  Rust 側 `parse_*` ベンチの中央値で 11〜18% 改善、スキャン自体は 1.5µs → 38ns。
- **ストリームイベント辞書キーのインターン** — `parse_stream`/`load_stream` が各イベン
  トに出す固定キーを `pyo3::intern!` の常駐オブジェクトに統一し、キーごとの Python
  文字列確保を解消。WSL 実測: `parse_stream` −34%、`parse_stream_multidoc` −39%、
  `load_stream` −22%。
- **分解用マイクロベンチ** — `granit_events_*` を追加し、granit の純イベントパイプライン
  コストと AST 構築を分離（ベンチのみ）。
- **マルチドキュメント解析の文書ごとのディープコピーを削除** — `on_document_end` は
  完成したドキュメントをクローンではなくコレクションへ移動します（次のドキュメントで
  result は再構築されるためクローンは純粋なオーバーヘッド）。WSL 実測:
  `parse_all_docs` −9.7%、`safe_loads`（マルチドキュメント）−9.5%、`YAML().safe_loads` −6.7%。
- **ストリーミング書き込みは文書間で単一バッファを再利用** — 新しい `direct_dump_into` は各文書を再
  利用の `String` へ書き込み、`dump_iterable` はテキストが改行 1 つで終わる通常ケースで
  `normalize_doc` の再コピーをスキップ。WSL 実測: `dump_stream_multi_doc` −27.2%、`dump_stream`
  −4.4%。
- **AST ビルダのスカラ高速パス** — `unescape_double_quoted` はバックスラッシュなしでは
  即返答、`detect_chomping` はブロックスカラ毎に全文行を collect せず遅延取得。
  WSL 実測: `to_dict` 系 −4〜9%、スカラ型ロード −3〜4%、退行なし。

#### ドキュメント

- **numpy ガイドの 0 次元スカラ節を全ロケールで修正** — 旧文は「単一要素リストへ
  リシェイプ」（`assert data == [42]`）と説明したが、実挙動（`tests/test_numpy.py` で
  固定）は素のスカラーへシリアライズ（`assert data == 42`）。4 ロケールとも修正し、
  en 版に 0-D `bool` → `1.0` の rust-numpy 特性に関する警告 admonition を追加。

</details>

### [v0.15.0] — 2026-08-19

<details markdown="1">
<summary>Node メタデータ/スタイル API · Schema ファイル IO · 深編集 · cp314t の numpy 再開</summary>

#### 追加

- **ノードメタデータのセッター/ゲッター** — `Node.comment` / `Node.anchor` / `Node.tag` 読み取りプロ
  パティと `set_comment` / `set_anchor` / `set_tag`（および `remove_*` 系）を追加。エイリアスや存在
  しないパスへの編集はエラーになります。インラインスカラー値・シーケンス項目上のスタンドアロンコメン
  トは独自のインデント行に出力されるようになりました（`child:\n  # c\n  val` と `- a\n# c\n- b` の既
  存のラウンドトリップ不具合を修正）。
- **バーベイタムタグ** — `set_tag("!<tag:yaml.org,2002:str>")` はバーベイタムタグ（空ハンドル）を生
  成し、ソースから解析したバーベイタムタグはラウンドトリップで保持されます：`Tag` の `Display` は空
  ハンドルタグを `!<...>` で囲んで出力し、`parse_tag` は `!<...>` 形式を認識し、ストリームイベントは
  `Display` 経由でタグを直列化します。
- **スキーマファイル IO と一覧** — `load_schema(name, path)` はファイルからスキーマ定義を読み込んで
  登録し、`list_schemas()` は登録済みのすべてのスキーマ名（組み込み
  `failsafe`/`json`/`core`/`yaml1.1` + カスタム）を返します。
- **ノード style/format セッター/ゲッター** — `Node.scalar_style` / `Node.flow_style` /
  `Node.chomping` 読み取りプロパティと `set_scalar_style` / `set_flow_style` / `set_chomping` メソッ
  ド。ScalarStyle/Chomping が `Copy` を derive するようになりました。非スカラーノードは `None` 返却
  / no-op、エイリアスや存在しないパスはエラーになります。
- **スキーマ構造検証** — スキーマ定義の `validate` セクションで構造チェック（パス限定スカラー型、
  `sequence_of`/`mapping_of` コンテナ、`required`）を追加。
  `validate_against_schema(data, schema_yaml)` はすべての失敗を列挙して `YamlValidateError` を送出し
  ます。
- **`Node.copy()`** — サブツリーをドキュメントから独立した Python 値（dict/list/scalar）として深くコ
  ピーします。`set_value()` で貼り付けるのに便利です。
- **詳細編集 API** — `doc.set_many({path: value})` で複数パス（ワイルドカード `[*]` とディープスキャ
  ン `..` 対応）を単一スプライスバーストで設定。`doc.sort_keys()` でマッピングキーをその場で並べ替
  え。`Node.move(new_path)` でサブツリーを移動。`Node.path` / `Node.find_first()` /
  `Node.value_eq()` でパスアクセス・最初のワイルドカード検索・値比較を追加。
- **0.14+ 機能のプロパティベーステスト** — `validate_node` / schema 解析 / style round-trip の Rust
  proptest、`set_many` ワイルドカード / metadata 編集 / `sort_keys` の Python hypothesis テストを追
  加。`hypothesis` を `test` グループへ移動し、CI でプロパティテストが実行されるように。
- **シリアライザ修正** — 空フローコンテナ（`key: {}` / `key: []`）上のスタンドアロンコメントが無効な
  YAML を生成していたのを修正（インラインへ降格）。

#### 変更

- **フリースレッド (cp314t) ホイールで NumPy を再有効化** — cp314t ビルド引数から
  `--no-default-features` を削除。rust-numpy 0.29 はフリースレッド Python をサポートし、
  `numpy.ndarray` シリアライズがフリースレッドホイールで利用可能になりました（NumPy のインストールは
  実行時に自動検出）。

#### ドキュメント

- **全言語（en/zh/ja/ko）ドキュメントの古い参照を修正** — `saphyr-parser` → `granit-parser`、YAML 準
  拠率 98.1% → 99.75%（スイート 405/406 件）、ABI3 サポート 3.9–3.13 → 3.8–3.15（py3.9+ → py3.8+）、
  ベンチマーク表を現在の CodSpeed CI 数値（パース 21〜43 倍、シリアライズ 55〜177 倍 PyYAML 比高速）
  に更新。Rust 側ベンチマーク章を Criterion から divan へ移行（`benches/yaml_bench.rs` →
  `crates/pyrs-yaml/benches/yaml_bench.rs`）。

</details>

### [v0.14.1] — 2026-08-15

<details markdown="1">
<summary>引用符とエスケープの境界：単一引用符のバックスラッシュ、BOM、非文字、多バイト改行</summary>

#### 修正

- **バックスラッシュ+制御文字/非文字を含む単一引用スカラー** — このような値は二重引用を使用するよう
  になりました。単一引用は制御文字/非文字をエスケープできません。
- **非文字と BOM の引用** — `needs_quotes` / `needs_double_quoted` は U+FFFE/U+FFFF/平面末尾非文字お
  よび U+FEFF（BOM）を引用必須として扱うようになりました。
- **二重引用エスケープ幅** — U+FFFF を超える符号位置は 8 桁の `\Uxxxxxxxx` 形式で出力します（4 桁の
  `\u` は BMP 専用）。
- **折り返し plain スカラーの継続インデント** — 継続インデントを値の開始列から導出し、ネストしたシー
  ケンス/マッピング項目の継続行が親ブロックインデントを超えるようにしました。
- **マルチバイト折り返し境界** — `wrap_plain_scalar` は折り返しスライスを文字境界に切り詰め、4 バイ
  ト UTF-8 が境界をまたぐ際の panic を防ぎます。
- **publish テスト要件に `hypothesis`** — `.ci/requirements-test.txt` に `hypothesis>=6.113.0` を固
  定し、公開ワークフローがプロパティテストを実行できるようにしました。

#### 追加

- **`scripts/fuzz_panics.py`** — dump/parse/edit/冪等性にわたる敵対的戦略を用いたローカル大規模
  Hypothesis fuzz ハーネス。

</details>

### [v0.14.0] — 2026-08-14

<details markdown="1">
<summary>YAML Schema 言語と差替可能な解決 · 引用符スカラー · 空コレクション</summary>

#### 追加

- **YAML Schema Language** — 正規表現パターンを YAML 型にマッピングする
  カスタムスキーマを定義可能。`register_schema()` で登録。
- **インライン dict スキーマ** — `schema` パラメータに `dict` を直接渡せる。
- **Community Plugins** — `CustomType` ベースクラスによる
  カスタムノード型の登録。`register_type()` で登録。
- **組み込みプラグイン** — `!timestamp`（datetime）と `!set` がデフォルトで登録済み。

#### 変更

- **スキーマ解決がプラグイン可能に** — `SchemaResolver` トレイト + `Schema` 列挙型 + グローバル
  `SchemaRegistry`。組み込みスキーマはゼロコストディスパッチを維持。
- **`node_to_pyobject` と `direct_dump` が `CustomType` をチェック** — タグ付きスカラは
  `from_yaml()` で変換、Python オブジェクトは `to_yaml()` でシリアライズ。
- **`get()` はリテラルキーのみ** — `YamlDocument.get()` は `.` や `[` を含むキーを JSONPath と推定し
  なくなり、常にトップレベルのマッピングキーとして扱います（`__getitem__`/`__setitem__` と一貫）。パ
  スアクセスは `find()`/`node()` を利用してください。

#### 修正

- **クォート付きスカラーは常に文字列として読み込まれる** — 暗黙の型解決はプレーンスカラーのみに適用
  （YAML 1.2）。`safe_load('"true"')` は文字列 `"true"`（`True` ではない）を返す。シリアライザはドキ
  ュメント（`to_yaml`）経路でも負数を正しく往復させます。
- **一重/二重引用符のみのキーが往復保存される** — 単一の `'` または `"` であるマッピングキーは引用ス
  カラーとして出力され、解析不能な YAML になりません。
- **空コレクションは `{}`/`[]` を出力** — 空のマッピング/シーケンスのダンプが、再解析で `None` にな
  る空ドキュメントを生成しなくなります。

</details>

### [v0.13.0] — 2026-08-10

<details markdown="1">
<summary>MSRV 1.96 と edition 2024 · 直接ライタと読込高速化 · granit 移行</summary>

#### 変更

- **Rust MSRV を 1.96 に引き上げ、edition を 2024 に変更** — 両 crate は `rust-version = "1.96"` お
  よび `edition = "2024"` を宣言します。CI は `build`/`test-freethreaded` ジョブを Rust 1.96 に固定
  し、決定論的な wheel ビルドを実現します。また、`msrv-check` ジョブを追加し、MSRV で
  `cargo check`/`cargo test` を実行して静かな MSRV ドリフトを防ぎます（`rust-lint` ジョブは `stable`
  のまま）。バージョンの床は PyO3 0.29 自身の基線（rustc 1.83）よりも上に設定され、std API の先行対
  応（例: `assert_matches!`、1.96 で安定化）を目的としています。`TAG_REGISTRY`（タグハンドラ管理）が
  `std::sync::LazyLock` にリファクタされ、`Mutex<Option<...>>` の間接レイヤーが除去されました。

#### パフォーマンス

- **`safe_dump` / `from_dict` / `dump_file` / `dump_iterable`: direct writer** — 中間 `CustomNode`
  AST を介さず Python→YAML シリアライズ。単一パス `direct_dump` が従来の 2 パス `pyobject_to_node` +
  `to_yaml` を置換。`safe_dump` で 7 倍高速化（28ns→4ns）、`from_dict` で 6 倍高速化（35ns→6ns）。
  (#60)
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

- **`max_depth` をストリーム & frontmatter API に追加** —
  `parse_stream(yaml, on_event, max_depth)`、`read_markdown(path, schema, max_depth)`、
  `read_markdown_str(content, schema, max_depth)` が `max_depth` を受け付ける（デフォルト 1000）。ス
  トリーム解析はコアの `parse_stream_with_options` によりネスト深さ制限を強制するようになった（従来
  のストリームイベントには深さ制限がなかった）。
- **Pydantic 統合** — `dump_pydantic()` は Pydantic モデルを YAML 文字列に
  シリアライズ（`model_dump(mode='json')` + `safe_dump`）；`parse_as()`
  は YAML 文字列を Pydantic モデルインスタンスにパース。両方とも遅延インポート、
  pydantic へのハード依存なし。(#61)

#### 内部

- **`py/mod.rs` の分割** — 巨大な 1786 行のモジュールを `document.rs`（YamlDocument）、
  `yaml_instance.rs`（YAML クラス）、 `functions.rs`（モジュールレベル関数）、`stream_iterator.rs`、
  `walk_helpers.rs` に分割。`mod.rs` は 128 行に削減。(#61)
- **`needs_quotes()` ガード + `double_quoted_scalar()` コンストラクタ** — `'true'` / `'42'` /
  `'null'` のような文字列は、コアスキーマで再パース時に誤読されないようダブルクォートのスカラーとし
  て出力（`pyobject_to_node` + `json_value_to_node`）。
- **CodSpeed ベンチマークを `codspeed-divan-compat` に統一** — `exclude-allocations` でアロケータノ
  イズを除去。クロスライブラリのベンチマークを `tests/test_benchmark_crosslib.py` に統合し、共通の
  `tests/data/yaml_samples.py` フィクスチャとストリーミングのカバレッジを追加。

</details>

### [v0.12.1] — 2026-08-06

<details markdown="1">
<summary>`set(create_missing)` · `walk`/`scalars` · monorepo · 解析ホットパス</summary>

#### 追加

- **`set(create_missing=True)`** — 編集パス上の欠落中間マッピングキーがネストしたマッピングとして作
  成されます（例: `a: 1` に対して `a.b.c` を設定すると `b` と `c` が作成されます）；未解決のインデッ
  クスセグメントは依然としてエラーとなり、パス上のスカラー中間ノードも依然として例外を発生させます。
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

- **モノレポワークスペース** — ソースコードを `crates/pyrs-yaml-core/`（純粋 Rust、PyO3 なし）と
  `crates/pyrs-yaml/`（PyO3 バインディング）に分割。ルート `Cargo.toml` はワークスペースになりまし
  た。旧 `src/` ディレクトリと `build.rs` は削除されました。
- **pyproject.toml** — `tool.maturin.manifest-path` を
  `crates/pyrs-yaml/Cargo.toml` に追加。
- **パースホットパス** — 単一パスコメント/アンカー抽出、遅延重複キー検出、`shift_insert` マージプリ
  ペンド、および単一ドキュメントパース用の `DocumentEnd` ディープクローンのスキップにより、大規模ド
  キュメントのパースコストを約 19% 削減（CodSpeed: parse[large] +13.9%、parse[medium] +16.6%、
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

</details>

### [0.11.7] — 2026-08-04

<details markdown="1">
<summary>常に失敗していた検査を release-guard の静的断言へ · numpy 追跡</summary>

#### 変更

- **stub-build-check から release-guard に置換** — v0.10.0 の `--generate-stubs` 失敗モードを再現す
  るために意図的に失敗する常時失敗のコンテナビルド（`validate.yml`）を、リポジトリが正しい場合に**
  合格する** 3 つの静的アサーションに置換：`grep` で `publish.yml` が `--generate-stubs` に対してガ
  ードされ、`git ls-files` がコミットされた `.pyi` が追跡されていることを確認し、`test -f` が
  `py.typed` の存在を確認します。ジョブは正しい状態で緑の CI を返し、回帰時のみ赤になります。

#### 追加

- **Numpy free-threaded 追跡** — ROADMAP.md が `rust-numpy` の free-threaded
  サポート状況（PyO3/rust-numpy#476）を追跡するようになり、Rust バインディングが
  成熟した際に cp314t wheel での ndarray シリアライズを再有効化する依存関係として
  管理されます。

</details>

### [0.11.6] — 2026-08-04

<details markdown="1">
<summary>cp314t wheel から numpy を除去 · free-threaded CI · インストール文書</summary>

#### 変更

- **Free-threaded（cp314t）wheel が numpy なしに** — `--no-default-features` でビルドされるため、
  rust-numpy は完全に除外されます（バイナリが小さく、ランタイムプローブなし）。`numpy.ndarray` に対
  する `safe_dump` は free-threaded ビルドで `YamlTypeError` を発生させます；GIL ビルド（Python
  3.8-3.15）は完全な ndarray シリアライズを保持します。

#### 追加

- **Free-threaded CI 検証** — `test-freethreaded` ジョブが `--no-default-features` でビルドとテスト
  を行うようになり、出荷される free-threaded wheel 構成と一致します。
- **インストールドキュメント** — `docs/{en,zh,ja,ko}` が free-threaded
  wheel が numpy なしであることを明記（cp314t での ndarray シリアライズは利用不可）。

</details>

### [0.11.5] — 2026-08-04

<details markdown="1">
<summary>パーサ堅牢性 3/4/5：監査結果ゼロでクローズ</summary>

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

</details>

### [0.11.4] — 2026-08-04

<details markdown="1">
<summary>空キー重複を許容（2JQS）· 正しい拒否も計上 · tab 復号</summary>

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

</details>

### [0.11.3] — 2026-08-03

<details markdown="1">
<summary>ストリーム書き出し · 行オフセットキャッシュ · publish 事前検証 · 適合報告</summary>

#### 追加

- ストリーミング書き込み：`YAML.dump_stream(file_obj, iterable)` / `YAML.dump_file(path, iterable)`
  — ドキュメントレベルの一定メモリ、自動 `---` セパレータ、`explicit_start`/`explicit_end` フラグ付
  き
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

</details>

### [0.11.2] — 2026-08-03

<details markdown="1">
<summary>解析は splice 適格計算を省略 · 線形カーソルレイアウト検査</summary>

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

</details>

### [0.11.0] — 2026-08-02

<details markdown="1">
<summary>メスを入れる逐次化：編集反映で文書全体を組み替えない</summary>

#### 追加

- **外科的シリアライズ** — 全 AST ノードのバイトレベルソーススパン追跡；セグメントベーススプライス —
  編集はタッチされた領域のみ再生成、未変更テキストはバイトコピー
- プロパティテスト（proptest、新規開発依存）
- 10MB 編集フラッシュベンチマーク（divan）

#### 変更

- `flush_source` がセグメントスプライスを使用；フロースタイル領域、非デフォルトレイアウト文書、マー
  ジキー、CRLF/BOM 文書、materialize 後（シングルバーストモデル）では全量シリアライズにフォールバッ
  ク
- スプライス編集は `---`/`...`/ディレクティブマーカー行を未変更バイトとして保持
  （全量シリアライズは以前それらを削除 — 意図的な動作差）

</details>

### [0.10.0] — 2026-08-01

<details markdown="1">
<summary>インプレース編集 API と編集ベンチマーク</summary>

#### 追加

- **インプレース編集** — フォーマットメタデータを失わずに解析済みドキュメントを編集：
    - パス API：`doc.set(path, value)`、`doc.insert(path, index, value)`、
      `doc.append(path, value)`、`doc.delete(path)`、`doc.rename(path, new_key)`、 JSONPath スタイル
      のパス（`$.a.b[0]`）；ルート用糖衣構文 `doc["key"] = value` と `del doc["key"]`
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

</details>

### [0.9.0] — 2026-08-01

<details markdown="1">
<summary>CPython 3.13/3.14/3.15 と no-GIL · タグハンドラ · pydantic · `.pyi`</summary>

#### 追加

- **Python 3.13、3.14、3.15 サポート** — PyO3 `abi3-py38` wheel が Python 3.8-3.15 をカバー（GIL ビ
  ルド）；`abi3t` + `abi3t-py315` は free-threaded 安定 ABI を提供
- **Free-threaded CPython（GIL なし）サポート** — `#[pymodule(gil_used = false)]` がモジュールを
  free-threaded Python 向けにスレッドセーフと宣言；`Py_GIL_DISABLED` cfg フラグで numpy をゲート
  （rust-numpy は free-threaded 未対応 — free-threaded ビルドでは `--no-default-features` で numpy
  feature を無効化）
- **CI free-threaded ジョブ** — 新しい `test-freethreaded` ワークフロージョブが
  Python 3.14t でコンパイルとテストを検証
- **`pyo3-build-config` ビルド依存** — `build.rs` 経由で
  `#[cfg(Py_GIL_DISABLED)]`、`#[cfg(Py_3_15)]` などのコンパイラフラグを有効化
- **`numpy` をオプション化** — `numpy` feature の背後にゲート（デフォルト有効）；
  `Py_GIL_DISABLED` 下では自動的に除外
- **`allow_duplicate_keys`** — `YAML(allow_duplicate_keys=True)`、
  `parse(..., allow_duplicate_keys=True)`、`parse_file`、`safe_load`、`safe_loads`、
  `parse_all_docs` がすべてフラグを受け入れます；重複マッピングキーはデフォルトで
  `YamlDuplicateKeyError` を発生、許可時は `last value wins`
- **`SerializeOptions` の拡張** — `doc.to_yaml_with_options()` が `width`（行ラップ、0 = 無効）、
  `indent_mapping`、`indent_sequence`、 `indent_offset` を既存の
  `indent_size`/`explicit_start`/`explicit_end`/ `sort_keys`/`max_depth` とともに追加
  （`src/py/mod.rs:432`）
- **タグハンドラレジストリ** — `register_tag("!custom")` デコレータとインペラティブフォーム +
  `clear_tag_handlers()`；登録タグを持つスカラーノードはハンドラを介して変換されます
  （`src/py/tag_registry.rs`）
- **優先度付きタグハンドラチェーン** — 複数のハンドラがタグごとに昇順 `priority` で実行；
  `YamlTagSkip` はハンドラが次のハンドラに通すことを許可、 fallback は元の値を保持
- **Pydantic 統合** — `parse_as(Model, yaml, **yaml_kwargs)` が YAML をパースし
  Pydantic v2 モデルに対して検証；pydantic がない場合は `ImportError` を
  ガイド付きで発生（`python/pyrs_yaml/pydantic.py`）
- **`.pyi` 型スタブ** — maturin によって自動生成されコミットされ、`register_tag`、`parse_as`、
  `to_yaml_with_options` および新しい例外が型チェッカーから見えるようになります。

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
- **pydantic テスト依存関係** — `pydantic>=2.10.6` を `[dependency-groups] test` と
  `.ci/requirements-test.txt` に追加（ci.yml 内の `uv sync` による SSOT）

#### 修正

- **Windows DLL 読み込み** — `src/py/tag_registry.rs` から `#[cfg(test)]` ブロックを削除し、Windows
  での `import pyrs_yaml` を修正（`250b8d0`）
- **Python 3.8 互換性** — `pydantic.py` に `from __future__ import annotations`
  を追加（`63d2495`）
- **CI pydantic スキップ** — `pytest.importorskip("pydantic")` を追加し、
  pydantic が未インストールでもテストがパスするよう修正（`7be011d`）
- **CI の Windows でのグロブ展開** — `pip install dist/*.whl` の
  `shell: bash` を追加（PowerShell は `*` を展開しない）（`2f7778d`）
- **文字列以外を返すタグハンドラが `YamlTagError` を発生** — 非 `str` 値を返すハンドラ（以前は黙っ
  て無視され元のスカラーを保持）が、`Tag handler '!x' must return a string` でエラーを発生
  （`src/py/mod.rs:resolve_tags`）
- **`to_yaml_with_options` インデント配線** — `indent_mapping`/`indent_sequence`/ `indent_offset` が
  シリアライザによって尊重されるようになりました（以前は死んだフィールド）；省略時はそれぞれ
  `indent_size`/0 にデフォルト（`src/serializer.rs`）
- **`width` が小さな値でハングしない** — `width < continuation indent` の場合、永久ループの代わりに
  未ラップで残りを出力するフォールバックに（`src/serializer.rs:write_plain_scalar`）
- **`remove_tag(name)`** — タグハンドラの登録解除用新関数；
  `register_tag`/`clear_tag_handlers` を補完（`src/py/tag_registry.rs`）
- **`duplicate-key` エラーが多言語化** — `YamlDuplicateKeyError` メッセージが全 4 ロケールで
  `format_i18n_error` を経由するようになりました（`src/i18n/locales/*.yml`）

</details>

### [0.8.0] — 2026-07-30

<details markdown="1">
<summary>`YAML()` インスタンス API · Python Node API · `MergedView` · ライフサイクル警告</summary>

#### 追加

- **`YAML()` インスタンス API** — 再利用可能な設定付き
  `YAML(typ="rt"|"safe"|"full", schema="core"|"yaml1.1", max_depth=1000)`； `.parse()`、
  `.safe_load()`、`.safe_loads()`、`.parse_file()`、 `.parse_all_docs()` メソッド
- **Python `Node` API** — AST 操作のための `Node` クラス：`find()`、`filter()`、`walk()`、
  `to_yaml()`、`parent`、`children`、`root_type`、`value`；JSONPath 風クエリ言語（`$.key.sub`、
  `$.arr[0]`、`$..deep`）
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

</details>

### [0.7.1] — 2026-07-30

<details markdown="1">
<summary>ryaml とのベンチ比較 · 適合基準の引き上げ · Divan 移行</summary>

#### 追加

- **ryaml ベンチマーク比較** — `tests/test_benchmark.py` が PyYAML と ruamel.yaml と並んで `ryaml`
  （Rust YAML ライブラリ）とも比較するようになり；`benchmark_compare.py` が機能比較レポートとして書
  き直されました（`tests/test_benchmark.py:25-28`、`.github/workflows/ci.yml:219`）
- **CI 準拠閾値の引き上げ** — YAML Test Suite 準拠ゲートが `test_compliance_report()` で 70% から
  75% に増加；有効パースレートゲート 95%（`tests/test_yaml_suite.py:251`）
- **CI 依存関係の統合** — パブリッシュワークフローとローカル開発全体の統一テスト依存関係管理のた
  め、`.ci/requirements-test.txt` と `.ci/requirements-test-lite.txt` を追加
- **ベンチマークの近代化** — 高速な C 拡張ベースの統計ベンチマークのため `pytest-benchmark` から
  `pytest-codspeed` へ移行；全 CI ジョブが `-r .ci/requirements-test.txt` を使用するようになりました
- **Rust ベンチマークを Divan に移行** — `codspeed-criterion-compat` を `codspeed-divan-compat`
  v5.0.1 に置換；16 個のベンチマークを Criterion グループから `#[divan::bench]` 属性に書き直し
  （`Cargo.toml`、`benches/yaml_bench.rs`）

#### 変更

- CI ベンチマークジョブがクロスライブラリ比較用に `ryaml` をインストール
- `benchmark_compare.py` はタイミングを `pytest-benchmark` に委譲し、
  機能比較/レポートツールとして機能するようになりました

</details>

### [0.7.0] — 2026-07-29

<details markdown="1">
<summary>シリアライザの `max_depth` 防御 · ホットパス最適化 · pytest-benchmark</summary>

#### 追加

- **シリアライザ `max_depth` ガード** — `serialize_node_internal` が再帰深度を追跡し、制限（デフォ
  ルト 1000）を超えると `YamlMaxDepthError` を発生（パーサーの保護と一致、
  `src/serializer.rs:135-145`）
- **シリアライザホットパス最適化** — ブロックスタイルシリアライズを対象とした
  5 つの最適化で約 4.9% のラウンドトリップ高速化：
    - `write_anchor_tag` および `write_inline_comment` の None チェックをインライン化
    （全ノードの約 99% でメソッドコールを除去）
    - `write_indent` のホット/コールドパス分離（キャッシュレベル ≤64 の直接インデックス）
    - `write_plain_scalar` の短小 ASCII 英数字文字列（≤8 文字）用高速パス
    - `write_scalar_for_key` の Plain スカラー用直接ディスパッチ（ディスパッチチェーンを回避）
- **pytest-benchmark 移行** — Python ベンチマークが統計的厳密さ、構造化 JSON 出力、CI 統合のため生
  `time.perf_counter()` から `pytest-benchmark` へ移行（`tests/test_benchmark.py` + 更新済み
  `tests/test_performance.py`）

#### 変更

- Python ベンチマークで生 `timeit` の代わりに `pytest-benchmark` を使用
- CI ベンチマークジョブがスタンドアロンスクリプトの代わりに
  `pytest --benchmark-json` を実行

#### 削除

- `write_inline_comment` メソッド — 全呼び出し箇所でインライン化
- シリアライザからの `Comment` インポート — 不要になった

</details>

### [0.6.0] — 2026-07-27

<details markdown="1">
<summary>非同期逐次化 · JSON Schema 検証 · `to_json()` · 増分再解析</summary>

#### 追加

- **非同期シリアライズ** — `asyncio.run_in_executor` 経由の `safe_dumps_async`、`safe_dump_async`、
  `safe_loads_async`、 `safe_load_async`（`python/pyrs_yaml/async_dump.py`）
- **JSON Schema 検証** — `YamlValidateError` 例外 + `YamlDocument.validate(schema)` メソッド（`str`
  または `dict` を接受）； Python `jsonschema` モジュールに委譲
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

</details>

### [0.5.0] — 2026-07-27

<details markdown="1">
<summary>`Serializer::write_node` 修正 · `YAML_SCHEMA` 定数 · 開発文書</summary>

#### 修正

- **`Serializer::write_node`** — `block_mapping`/`block_sequence` 内の
  `values.iter().next().unwrap()` での `.unwrap()` を安全なインデックスアクセスに
  置換し、エッジケース AST での潜在的パニックを除去
- **`YAML_SCHEMA` 定数** — 誤字 `yamorg2002` を
  `yamlorg2002` に修正（YAML 1.2 仕様 URL と一致）
- **開発ドキュメント** — `AGENTS.md` を更新し、Python コマンドに必須の
  `uv run` プレフィックスと Rust コマンドに直接 `cargo` を明記

</details>

### [0.4.0] — 2026-07-27

<details markdown="1">
<summary>132 件の補完テスト：i18n・複数ドキュメント・バイト入力・套件逐例</summary>

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
- **YAML Test Suite 個別ケーステスト** — 8 進数、16 進数、科学表記法、NaN、無限大、マージキー、明示
  的/暗黙的キー、bool/null 変種、ブロックスカラーストリップ（`|-`）、フローコレクション
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

</details>

### [0.3.0] — 2026-07-27

<details markdown="1">
<summary>NumPy ndarray 逐次化（N-D）· 引用符スカラーの型 · 負数の往復</summary>

#### 追加

- **NumPy ndarray シリアライズ** — `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()` が
  全次元（0-D から N-D）の `numpy.ndarray` をサポートするようになりました
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

- **負の数ラウンドトリップ** — YAML 1.2 のブロックシーケンスに `-` で始まるプレーンスカラーは含めら
  れないため、負の数はシリアライズ時に引用され、整数/浮動小数点数として正しくパースされるようになり
  ました
- **N-D 配列サポート** — 1-D のみに限定されず任意次元の配列をサポートするよう
  `PyArray1<T>` を `PyArrayDyn<T>` に置換
- **正しいネスト深さ** — 多次元配列がちょうど N レベルのネストを生成
  （shape[1..] が内部次元を処理、ルート次元が `plain_sequence` でラップ）
- `to_dict()` と `safe_load()` 内のエイリアス解決 — エイリアスが
  `None` ではなく参照値に解決されるようになりました
- `safe_loads()` が単純な `split("---")` を使用しなくなり、
  saphyr のドキュメントイベントを使用
- パース中のマッピング/シーケンスタグが廃棄されなくなりました
- `format_scalar_for_key()` が Literal/Folded ブロックスカラー形式を処理するようになりました

#### 変更

- ndarray 型ディスパッチ用の依存関係として `numpy` crate（v0.29）を追加
- PyO3 を 0.21 から 0.29 にアップグレード
- 15 個以上のボイラープレート `CustomNode` 構築を
  `plain_scalar()`/`plain_mapping()`/`plain_sequence()`/`plain_null()` コンストラクタに置換
- シリアライザが `write_anchor_tag()` と `write_inline_comment()` ヘルパーを抽出
- パーサーが `detect_flow_style()` ヘルパーを抽出
- 死んだコードを削除：`ParseOptions`、`find_inline_comment`、
  `find_standalone_comment_before`、`format_yaml_type`（テスト専用）
- 6 個の重複テストファイルを統合し、9 個の診断スクリプトを `scripts/` に移動
- キー/インデックス/型コンテキスト付きのエラーメッセージを改善

</details>

### [0.1.0] — 2026-07-25

<details markdown="1">
<summary>初回リリース：YAML 1.2 のフルメタデータ AST · 往復保持 · PyYAML 互換 API</summary>

#### 追加

- saphyr-parser による YAML 1.2 準拠の初期リリース
- 完全なメタデータ（コメント、アンカー、タグ、チョンピング、スカラー形式）付き
  カスタム AST
- コメント、アンカー、タグ、フォーマットのラウンドトリップ保持
- PyYAML 互換 API（`safe_load`/`safe_dump`）
- `from_dict`/`from_json` 変換関数
- YAML フロント matter 抽出用 `read_markdown`/`read_markdown_str`
- チョンピングインジケータ付きブロックスカラー（`|`/`>`、`|-`/`|+`/`>-`/`>+`）
- エスケープシーケンス（`\n`、`\t`、`\uXXXX`、`\xXX`）
- YAML 1.2 型解決（null、bool、int、float、無限大、NaN）
- マージキー解決（`<<: *alias`）
- 複合キー（シーケンス/マッピングをキーとして）

</details>
