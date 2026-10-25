---
title: ベンチマーク
description: pyrs-yaml のパフォーマンスベンチマーク。パース、シリアライズ、ラウンドトリップの各指標を掲載します。
tags:
  - docs
status: new
---

!!! note "ベンチマーク環境"
    CodSpeed CI は **シミュレーション**モード（`mode: simulation`）で実行され、実時間ではなく命令数を測定します。以下のウォールクロック値は `scripts/bench_snapshot.py` が生成します。各ケースをバッチ単位で計時し、9 バッチのうち最速のものを記録します。測定値は、測定時のインタープリタ、ライブラリのバージョン、フィクスチャのサイズとともに `.ci/perf-snapshot.json` にコミットされています。倍率は目安であり、インタープリタ、ハードウェア、文書によって変動します。

pyrs-yaml のパフォーマンスベンチマーク。CodSpeed のシミュレーション数値は相対トレンドの追跡用であり、以下のウォールクロック表は単一マシンでのバッチ計時による測定値で、`scripts/bench_snapshot.py` で再現できます。

## 測定方法

- **ツール:** CI では `codspeed-divan-compat`（Rust）と `pytest-codspeed`（Python）。ウォールクロック表は `scripts/bench_snapshot.py` が生成
- **反復:** 各ケースを 9 バッチ、各バッチは 0.1 秒以上に調整し、最速のバッチを報告
- **指標:** マイクロ秒（Python のウォールクロックと Rust の計測値）。CodSpeed は時間ではなく命令数を報告

## パースパフォーマンス

<!-- bench: layout=sizes format=yaml operation=parse sizes=small,medium,large baseline=PyYAML -->
| YAMLサイズ | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | PyYAML比の高速化 |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------- |
| 小（約100 B） | 4.4 µs | 151 µs | 347 µs | 1.9 µs | 5.7 µs | **35×** |
| 中（約690 B） | 27 µs | 976 µs | 2.3 ms | 10 µs | 33 µs | **37×** |
| 大（約2 KB） | 72 µs | 2.3 ms | 5.4 ms | 27 µs | 86 µs | **32×** |

## シリアライズパフォーマンス

<!-- bench: layout=sizes format=yaml operation=serialize sizes=small,medium,large baseline=PyYAML -->
| YAMLサイズ | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | PyYAML比の高速化 |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------- |
| 小（約100 B） | 0.42 µs | 86 µs | 227 µs | 0.97 µs | 3.1 µs | **204×** |
| 中（約690 B） | 1.6 µs | 568 µs | 1.4 ms | 6.1 µs | 17 µs | **347×** |
| 大（約2 KB） | 5.4 µs | 1.4 ms | 3.3 ms | 17 µs | 42 µs | **261×** |

## ラウンドトリップパフォーマンス

<!-- bench: layout=sizes format=yaml operation=roundtrip sizes=small,medium,large baseline=PyYAML -->
| YAMLサイズ | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | PyYAML比の高速化 |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------- |
| 小（約100 B） | 5.7 µs | 265 µs | 619 µs | 3.1 µs | 9.7 µs | **47×** |
| 中（約690 B） | 30 µs | 1.7 ms | 3.6 ms | 17 µs | 51 µs | **55×** |
| 大（約2 KB） | 82 µs | 3.9 ms | 9.1 ms | 48 µs | 135 µs | **48×** |

### JSON

同じ JSON テキストを 3 つの読み取り器に通します。大きい文書では `orjson` の方が速い。この行は見栄えのためではなく測定結果なので載せています。

<!-- bench: layout=sizes format=json operation=parse sizes=config,large baseline=json -->
| 文書 | pyrs-yaml | json | orjson | json比の高速化 |
| --- | ---: | ---: | ---: | ---: |
| 設定 (~123 B) | 0.56 µs | 1.3 µs | 0.45 µs | **2.3×** |
| 大 (~1.9 KB) | 7.8 µs | 9.8 µs | 4.5 µs | **1.3×** |

### TOML

TOML テキストを `pyrs_yaml.load_toml`、標準ライブラリの `tomllib`、`toml-rs` のバインディング `rtoml` に分けます。`rtoml` は辞書へ直接デコードするので、二つのフィクスチャではまだ約 1.5 倍速く読めます。コストになっていたノードモデルはもう払いません — `load_toml` はスキャナのテーブルツリーをそのまま Python に組み立てます — 残る差はそのツリーです。同じ機械での測定では、走査とツリー構築だけで `rtoml` の解析全体とほぼ等しいため、次の目標はツリーを作らないことです。

<!-- bench: layout=sizes format=toml operation=parse sizes=hot,config baseline=rtoml -->
| 文書 | pyrs-yaml | tomllib | rtoml | rtoml比の高速化 |
| --- | ---: | ---: | ---: | ---: |
| ホット (~1.8 KB) | 29 µs | 212 µs | 20 µs | **0.7×** |
| 設定 (~2.1 KB) | 45 µs | 358 µs | 31 µs | **0.7×** |

### JSON5

小さな JSON5 文書 1 本を `pyjson5` と。独立系の JSON5 読み取り器はこれが唯一です。ここではこちらが遅く、JSON5 読み取りは残っている最適化課題です。

<!-- bench: layout=sizes format=json5 operation=parse sizes=config baseline=pyjson5 -->
| 文書 | pyrs-yaml | pyjson5 | pyjson5比の高速化 |
| --- | ---: | ---: | ---: |
| 設定 (~83 B) | 2.0 µs | 0.82 µs | **0.4×** |

## Rust側ベンチマーク（divan）

Rust の CodSpeed レグ（`cargo codspeed`、シミュレーションモード）で測定した値です。これはネイティブのウォールクロックではなく計測器を介した時間で、フィクスチャは `pyrs_yaml_core::bench_inputs` 由来です。「small」は 19 B で、Python 表の 95 B 文書とは異なるため、同名の行は直接比較できません。

| 操作 | 時間 |
|-----------|------|
| パース（小） | 85.6 µs |
| パース（中） | 277.4 µs |
| パース（大） | 840.6 µs |
| パース（アンカー） | 254.3 µs |
| パース（コメント） | 164.9 µs |
| パース（ブロックスカラー） | 123.8 µs |
| シリアライズ（小） | 7.9 µs |
| シリアライズ（中） | 32.2 µs |
| シリアライズ（大） | 76.1 µs |
| シリアライズ（アンカー） | 27.0 µs |
| シリアライズ（ブロックスカラー） | 15.4 µs |
| ラウンドトリップ（小） | 91.7 µs |
| ラウンドトリップ（中） | 303.1 µs |
| ラウンドトリップ（大） | 910.0 µs |

## 主な知見

1. :material-trending-up: **pyrs-yamlはPyYAMLより解析で32〜37倍、シリアライズで204〜347倍高速**
2. :material-trending-up: **pyrs-yamlはラウンドトリップ機能を維持しながらruamel.yamlより解析で75〜85倍、シリアライズで540〜832倍高速**
3. :material-trending-up: **ラウンドトリップはPyYAMLより47〜55倍、ruamel.yamlより109〜119倍高速**
4. :material-scale-balance: **Rust バックエンドの相手が結果を分けます** — `yaml-rs` はラウンドトリップツリーではなく素のオブジェクトを返すため、解析で2.3〜2.7倍、ラウンドトリップで1.7〜1.8倍速い。代わりに pyrs-yaml のシリアライズは2.3〜3.7倍速くなります
5. :material-bolt: **Rust側パース**は Python 呼び出しのオーバーヘッドがなく、シミュレーションレグが操作ごとにコストを分解します
6. :material-bolt: **シリアライズ**は同じシミュレーションレグで常にパースコストを下回ります
7. :material-chart-line: **高速化の優位性は、あらゆるドキュメントサイズで一貫している**

## 注記

- Python 表：2026-10-25 に `scripts/bench_snapshot.py` が記録したバッチ計時のウォールクロック値。スナップショット（`.ci/perf-snapshot.json`）に環境、ライブラリのバージョン、フィクスチャのサイズを記録
- CodSpeed CI はシミュレーションモードで命令数を報告し、相対トレンドの追跡に使います
- 倍率は目安であり、インタープリタ、ハードウェア、文書サイズによって変動します
- PyYAMLのベンチマークは `safe_load` / `safe_dump` を使用（同等の安全保証）
- ruamel.yamlのベンチマークはデフォルト設定を使用
- `yaml-rs` と `ryaml` が返するのは素の Python オブジェクトです。pyrs-yaml の行が組むのは、その後の編集が書式を保存できるドキュメントなので、同じ仕事ではありません
- Python フィクスチャは 95 B / 687 B / 1949 B（`tests/data/yaml_samples.py`）
