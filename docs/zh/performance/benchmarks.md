---
title: 基准测试
description: pyrs-yaml 的性能基准测试结果，包括解析、序列化和往返性能对比。
tags:
  - docs
status: new
---

## 基准测试

!!! note "基准测试环境"
    CodSpeed CI 以 **模拟（simulation）** 模式运行（`mode: simulation`），统计的是指令数而非墙钟时间。下文的墙钟数字由 `scripts/bench_snapshot.py` 生成：它对每个用例分批计时，取九批中最快的一批；读数连同测量时的解释器、库版本与夹具大小一并提交在 `.ci/perf-snapshot.json`。加速比仅供参考，会随解释器、硬件与文档而变化。

pyrs-yaml 的性能基准测试。CodSpeed 的模拟数据用于追踪相对趋势；下文的墙钟表格是来自单台机器的分批计时读数，可用 `scripts/bench_snapshot.py` 复现。

### 方法论

- **工具：** CI 中使用 `codspeed-divan-compat`（Rust）与 `pytest-codspeed`（Python）；墙钟表格由 `scripts/bench_snapshot.py` 生成
- **轮次：** 每个用例九批，每批至少 0.1 秒；报告最快的一批
- **指标：** 微秒（Python 墙钟与 Rust 插桩）；CodSpeed 报告指令数而非时间

### 解析性能

<!-- bench: layout=sizes format=yaml operation=parse sizes=small,medium,large baseline=PyYAML -->
| YAML 大小 | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | 相比 PyYAML 加速 |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------ |
| 小 (~100 B) | 4.4 µs | 151 µs | 347 µs | 1.9 µs | 5.7 µs | **35×** |
| 中 (~690 B) | 27 µs | 976 µs | 2.3 ms | 10 µs | 33 µs | **37×** |
| 大 (~2 KB) | 72 µs | 2.3 ms | 5.4 ms | 27 µs | 86 µs | **32×** |

### 序列化性能

<!-- bench: layout=sizes format=yaml operation=serialize sizes=small,medium,large baseline=PyYAML -->
| YAML 大小 | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | 相比 PyYAML 加速 |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------ |
| 小 (~100 B) | 0.42 µs | 86 µs | 227 µs | 0.97 µs | 3.1 µs | **204×** |
| 中 (~690 B) | 1.6 µs | 568 µs | 1.4 ms | 6.1 µs | 17 µs | **347×** |
| 大 (~2 KB) | 5.4 µs | 1.4 ms | 3.3 ms | 17 µs | 42 µs | **261×** |

### 往返性能

<!-- bench: layout=sizes format=yaml operation=roundtrip sizes=small,medium,large baseline=PyYAML -->
| YAML 大小 | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | 相比 PyYAML 加速 |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------ |
| 小 (~100 B) | 5.7 µs | 265 µs | 619 µs | 3.1 µs | 9.7 µs | **47×** |
| 中 (~690 B) | 30 µs | 1.7 ms | 3.6 ms | 17 µs | 51 µs | **55×** |
| 大 (~2 KB) | 82 µs | 3.9 ms | 9.1 ms | 48 µs | 135 µs | **48×** |

### JSON

同一段 JSON 文本走三个读取器。大文档上 `orjson` 更快；这一行照登，因为它是度量，不是为了好看。

<!-- bench: layout=sizes format=json operation=parse sizes=config,large baseline=json -->
| 文档 | pyrs-yaml | json | orjson | 相比 json 加速 |
| --- | ---: | ---: | ---: | ---: |
| 配置 (~123 B) | 0.56 µs | 1.3 µs | 0.45 µs | **2.3×** |
| 大 (~1.9 KB) | 7.8 µs | 9.8 µs | 4.5 µs | **1.3×** |

### TOML

TOML 文本分别交给 `pyrs_yaml.load_toml`、标准库的 `tomllib` 与 `toml-rs` 的绑定 `rtoml`。`rtoml` 直接解码成字典，两个夹具上仍快约 1.5 倍。付费的不再是节点模型：`load_toml` 现在把扫描器自己的表树直接走成 Python 对象，这一行剩下的差距就是那棵树。本机实测显示，扫描加建树的成本已经接近 `rtoml` 整个解析的成本，所以下一步目标是根本不建树。

<!-- bench: layout=sizes format=toml operation=parse sizes=hot,config baseline=rtoml -->
| 文档 | pyrs-yaml | tomllib | rtoml | 相比 rtoml 加速 |
| --- | ---: | ---: | ---: | ---: |
| 热点 (~1.8 KB) | 29 µs | 212 µs | 20 µs | **0.7×** |
| 配置 (~2.1 KB) | 45 µs | 358 µs | 31 µs | **0.7×** |

### JSON5

一个小的 JSON5 文档对 `pyjson5`——它是生态里唯一独立的 JSON5 读取器。这里我们更慢，JSON5 读取也因此是一个待办的优化点。

<!-- bench: layout=sizes format=json5 operation=parse sizes=config baseline=pyjson5 -->
| 文档 | pyrs-yaml | pyjson5 | 相比 pyjson5 加速 |
| --- | ---: | ---: | ---: |
| 配置 (~83 B) | 2.0 µs | 0.82 µs | **0.4×** |

### Rust 侧基准测试（divan）

由 Rust 的 CodSpeed 链路（`cargo codspeed`，模拟模式）测得：这些是插桩时间，不是原生墙钟时间。夹具来自 `pyrs_yaml_core::bench_inputs`，其中 "small" 文档为 19 B，不是 Python 表格使用的 95 B 文档，因此同名行不可直接比较。

| 操作 | 时间 |
|------|------|
| 解析（小） | 85.6 µs |
| 解析（中） | 277.4 µs |
| 解析（大） | 840.6 µs |
| 解析（含锚点） | 254.3 µs |
| 解析（含注释） | 164.9 µs |
| 解析（块标量） | 123.8 µs |
| 序列化（小） | 7.9 µs |
| 序列化（中） | 32.2 µs |
| 序列化（大） | 76.1 µs |
| 序列化（含锚点） | 27.0 µs |
| 序列化（块标量） | 15.4 µs |
| 往返（小） | 91.7 µs |
| 往返（中） | 303.1 µs |
| 往返（大） | 910.0 µs |

### 要点总结

1. :material-trending-up: **pyrs-yaml 解析比 PyYAML 快 32–37 倍、序列化快 204–347 倍**
2. :material-trending-up: **pyrs-yaml 解析比 ruamel.yaml 快 75–85 倍、序列化快 540–832 倍**，同时提供相同的往返特性
3. :material-trending-up: **往返比 PyYAML 快 47–55 倍、比 ruamel.yaml 快 109–119 倍**
4. :material-scale-balance: **Rust 后端的对手把结果一分为二**——`yaml-rs` 只产出普通对象而非可往返的文档树，解析快 2.3–2.7 倍、往返快 1.7–1.8 倍；作为交换，pyrs-yaml 的序列化快 2.3–3.7 倍
5. :material-bolt: **Rust 侧解析**没有 Python 调用开销；模拟链路按操作拆解解析成本
6. :material-bolt: **序列化**在同一模拟链路中始终低于解析成本
7. :material-chart-line: **速度优势在所有文档尺寸下均保持**

### 说明

- Python 表格：2026-10-25 由 `scripts/bench_snapshot.py` 记录的分批计时墙钟读数；快照（`.ci/perf-snapshot.json`）记录了环境、库版本与夹具大小
- CodSpeed CI 以模拟模式运行并报告指令数，用于追踪相对趋势
- 加速比仅供参考，会随解释器、硬件与文档大小而变化
- PyYAML 基准测试使用 `safe_load`/`safe_dump`（相同安全保证）
- ruamel.yaml 基准测试使用默认设置
- `yaml-rs` 与 `ryaml` 交回的是普通 Python 对象，因此它们的行与 pyrs-yaml 的行不是同一件事——后者构建的是后续编辑仍能保住格式的文档
- Python 夹具为 95 B / 687 B / 1949 B（`tests/data/yaml_samples.py`）
