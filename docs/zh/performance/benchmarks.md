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

| YAML 大小 | pyrs-yaml | PyYAML | ruamel.yaml | 相比 PyYAML 加速 |
|-----------|-----------|--------|-------------|------------------|
| 小 (~100 B) | 4.6 µs | 134 µs | 311 µs | **29×** |
| 中 (~690 B) | 24 µs | 839 µs | 2.0 ms | **34×** |
| 大 (~2 KB) | 67 µs | 2.0 ms | 5.0 ms | **30×** |

### 序列化性能

| YAML 大小 | pyrs-yaml | PyYAML | ruamel.yaml | 相比 PyYAML 加速 |
|-----------|-----------|--------|-------------|------------------|
| 小 (~100 B) | 0.37 µs | 83 µs | 201 µs | **223×** |
| 中 (~690 B) | 1.3 µs | 481 µs | 1.1 ms | **368×** |
| 大 (~2 KB) | 4.5 µs | 1.2 ms | 2.6 ms | **265×** |

### 往返性能

| YAML 大小 | pyrs-yaml | PyYAML | ruamel.yaml | 相比 PyYAML 加速 |
|-----------|-----------|--------|-------------|------------------|
| 小 (~100 B) | 5.3 µs | 238 µs | 545 µs | **45×** |
| 中 (~690 B) | 26 µs | 1.5 ms | 3.3 ms | **59×** |
| 大 (~2 KB) | 70 µs | 3.5 ms | 7.8 ms | **50×** |

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

1. :material-trending-up: **pyrs-yaml 解析比 PyYAML 快 29–34 倍、序列化快 223–368 倍**
2. :material-trending-up: **pyrs-yaml 解析比 ruamel.yaml 快 68–81 倍、序列化快 541–876 倍**，同时提供相同的往返特性
3. :material-trending-up: **往返比 PyYAML 快 45–59 倍、比 ruamel.yaml 快 102–125 倍**
4. :material-bolt: **Rust 侧解析**没有 Python 调用开销；模拟链路按操作拆解解析成本
5. :material-bolt: **序列化**在同一模拟链路中始终低于解析成本
6. :material-chart-line: **速度优势在所有文档尺寸下均保持**

### 说明

- Python 表格：2026-10-20 由 `scripts/bench_snapshot.py` 记录的分批计时墙钟读数；快照（`.ci/perf-snapshot.json`）记录了环境、库版本与夹具大小
- CodSpeed CI 以模拟模式运行并报告指令数，用于追踪相对趋势
- 加速比仅供参考，会随解释器、硬件与文档大小而变化
- PyYAML 基准测试使用 `safe_load`/`safe_dump`（相同安全保证）
- ruamel.yaml 基准测试使用默认设置
- Python 夹具为 95 B / 687 B / 1949 B（`tests/data/yaml_samples.py`）
