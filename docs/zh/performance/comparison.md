---
title: 与其他库对比
description: pyrs-yaml 与 PyYAML、ruamel.yaml 的全面对比，涵盖性能和功能特性。
tags:
  - docs
status: new
---

## 与其他库对比

pyrs-yaml 与两款最流行的 Python YAML 库进行对比。

### 性能对比

#### 解析速度（大 YAML，约 2 KB）

| 库 | 时间 | 加速比 |
|----|------|--------|
| **pyrs-yaml** | 67 µs | — |
| PyYAML | 2.0 ms | 30× slower |
| ruamel.yaml | 5.0 ms | 75× slower |

#### 序列化速度（大 YAML，约 2 KB）

| 库 | 时间 | 加速比 |
|----|------|--------|
| **pyrs-yaml** | 4.5 µs | — |
| PyYAML | 1.2 ms | 265× slower |
| ruamel.yaml | 2.6 ms | 583× slower |

#### 往返速度（大 YAML，约 2 KB）

| 库 | 时间 | 加速比 |
|----|------|--------|
| **pyrs-yaml** | 70 µs | — |
| PyYAML | 3.5 ms | 50× slower |
| ruamel.yaml | 7.8 ms | 111× slower |

### 与「直接产出原生对象」的 Rust 解析器的取舍

pyrs-yaml 是**同时保留往返 AST**（注释、锚点、标签与源码 span 能在 load→edit→dump 后存活）的 YAML 库中最快的。与只反序列化为普通 Python 对象、且**不提供**往返编辑的 Rust 解析器（本仓库横向基准里的 `yaml-rs`）相比，结果是一分为二的：

- **序列化：** pyrs-yaml 在相同夹具上领先约 2–4 倍（见上表）。
- **解析为 `YamlDocument`：** pyrs-yaml 比 `yaml-rs` 慢约 2.5 倍，因为它构建了 `yaml-rs` 从不产出的更丰富的往返结构。这是有意的设计取舍——多出的开销正是注释/锚点/顺序保真的来源，而非可优化掉的回退。

若你只*读取* YAML、需要解析的最后一点速度、且从不把改动按原格式写回，可选 `yaml-rs` 这类纯原生解析器；若往返保真重要，则选 pyrs-yaml——即便在此，它解析仍比 PyYAML 快 29–34 倍、序列化快 223–368 倍。

### 功能对比

| 功能 | pyrs-yaml | PyYAML | ruamel.yaml |
|------|-----------|--------|-------------|
| **YAML 1.2 合规** | :material-check: | :material-check: | :material-check: |
| **独立注释** | :material-check: | :material-close: | :material-check: |
| **内联注释** | :material-check: | :material-close: | :material-check: |
| **锚点/别名** | :material-check: | :material-check: | :material-check: |
| **标签（显式）** | :material-check: | :material-check: | :material-check: |
| **块标量** | :material-check: | :material-check: | :material-check: |
| **流集合** | :material-check: | :material-check: | :material-check: |
| **合并键（<<）** | :material-check: | :material-check: | :material-check: |
| **复杂键** | :material-check: | :material-check: | :material-check: |
| **往返保留** | :material-check: | :material-close: | :material-check: |
| **Python 绑定** | :material-check: | :material-check: | :material-check: |
| **ABI3（py3.8+）** | :material-check: | :material-close: | :material-close: |
| **类型存根（.pyi）** | :material-check: | :material-close: | :material-close: |
| **国际化错误消息** | :material-check: | :material-close: | :material-close: |
| **Rust 后端** | :material-check: | :material-close: | :material-close: |
| **性能** | :material-rocket-launch: 最快 | :material-snail: 慢 | :material-snail: 慢 |

### 总结

#### 选择 pyrs-yaml 的场景

- **性能至关重要** — 比 PyYAML 解析快 29–34 倍、序列化快 223–368 倍
- **往返保留是核心需求** — 保留注释、锚点、标签
- **需要 PyYAML 兼容性** — 即插即用 API
- **需要类型提示** — 完整 `.pyi` 存根
- **需要一个 wheel** — ABI3 兼容 Python 3.8–3.15

#### 选择 PyYAML 的场景

- 您已在使用它且不需要往返保留
- 需要与现有代码最大兼容
- 性能不是关注点

#### 选择 ruamel.yaml 的场景

- 需要功能最全面的 YAML 解析器
- 正在进行复杂的 YAML 操作
- 性能不是关注点（它是三个选项中最慢的）

### 迁移路径

```python title="迁移步骤"
# 第一步：安装
pip install pyrs-yaml

# 第二步：替换导入
# 之前：
import yaml

# 之后：
import pyrs_yaml as yaml

# 第三步：测试
# 运行现有测试以验证兼容性
```

大多数代码无需修改即可工作。主要差异：

1. 往返输出将保留注释和格式
2. 错误消息更详细且可本地化
3. 性能将显著提升
