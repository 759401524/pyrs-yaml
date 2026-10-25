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
| 小（約100 B） | 4.5 µs | 127 µs | 300 µs | 1.8 µs | 5.0 µs | **28×** |
| 中（約690 B） | 23 µs | 814 µs | 1.9 ms | 9.9 µs | 28 µs | **35×** |
| 大（約2 KB） | 63 µs | 2.0 ms | 4.8 ms | 24 µs | 74 µs | **32×** |

## シリアライズパフォーマンス

<!-- bench: layout=sizes format=yaml operation=serialize sizes=small,medium,large baseline=PyYAML -->
| YAMLサイズ | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | PyYAML比の高速化 |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------- |
| 小（約100 B） | 0.39 µs | 77 µs | 180 µs | 0.85 µs | 2.7 µs | **198×** |
| 中（約690 B） | 1.3 µs | 453 µs | 1.1 ms | 5.5 µs | 15 µs | **339×** |
| 大（約2 KB） | 4.6 µs | 1.1 ms | 2.4 ms | 14 µs | 37 µs | **245×** |

## ラウンドトリップパフォーマンス

<!-- bench: layout=sizes format=yaml operation=roundtrip sizes=small,medium,large baseline=PyYAML -->
| YAMLサイズ | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | PyYAML比の高速化 |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------- |
| 小（約100 B） | 5.1 µs | 221 µs | 524 µs | 2.8 µs | 8.3 µs | **43×** |
| 中（約690 B） | 26 µs | 1.3 ms | 3.2 ms | 16 µs | 43 µs | **52×** |
| 大（約2 KB） | 70 µs | 3.2 ms | 7.5 ms | 41 µs | 117 µs | **46×** |

### JSON

同じ JSON テキストを 3 つの読み取り器に通します。大きい文書では `orjson` の方が速い。この行は見栄えのためではなく測定結果なので載せています。

<!-- bench: layout=sizes format=json operation=parse sizes=config,large baseline=json -->
| 文書 | pyrs-yaml | json | orjson | json比の高速化 |
| --- | ---: | ---: | ---: | ---: |
| 設定 (~123 B) | 0.47 µs | 1.1 µs | 0.34 µs | **2.3×** |
| 大 (~1.9 KB) | 6.0 µs | 8.5 µs | 3.8 µs | **1.4×** |

### TOML

TOML テキストを `pyrs_yaml.load_toml`、標準ライブラリの `tomllib`、`toml-rs` のバインディング `rtoml` に分けます。`rtoml` は辞書へ直接デコードするため、二つのフィクスチャで約 1.9 倍速く読めます。`load_toml` は YAML の書式保存編集が乗る共有ドキュメントツリーを先に組み立てており、そのツリーが差です。短縮は残っている課題です。

<!-- bench: layout=sizes format=toml operation=parse sizes=hot,config baseline=rtoml -->
| 文書 | pyrs-yaml | tomllib | rtoml | rtoml比の高速化 |
| --- | ---: | ---: | ---: | ---: |
| ホット (~1.8 KB) | 32 µs | 175 µs | 17 µs | **0.5×** |
| 設定 (~2.1 KB) | 51 µs | 301 µs | 26 µs | **0.5×** |

### JSON5

小さな JSON5 文書 1 本を `pyjson5` と。独立系の JSON5 読み取り器はこれが唯一です。ここではこちらが遅く、JSON5 読み取りは残っている最適化課題です。

<!-- bench: layout=sizes format=json5 operation=parse sizes=config baseline=pyjson5 -->
| 文書 | pyrs-yaml | pyjson5 | pyjson5比の高速化 |
| --- | ---: | ---: | ---: |
| 設定 (~83 B) | 1.7 µs | 0.67 µs | **0.4×** |

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

1. :material-trending-up: **pyrs-yamlはPyYAMLより解析で28〜35倍、シリアライズで198〜339倍高速**
2. :material-trending-up: **pyrs-yamlはラウンドトリップ機能を維持しながらruamel.yamlより解析で67〜82倍、シリアライズで465〜832倍高速**
3. :material-trending-up: **ラウンドトリップはPyYAMLより43〜52倍、ruamel.yamlより102〜125倍高速**
4. :material-scale-balance: **Rust バックエンドの相手が結果を分けます** — `yaml-rs` はラウンドトリップツリーではなく素のオブジェクトを返すため、解析で2.4〜2.6倍、ラウンドトリップで1.6〜1.9倍速い。代わりに pyrs-yaml のシリアライズは2.2〜4.1倍速くなります
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
