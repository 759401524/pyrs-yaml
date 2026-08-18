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
| **pyrs-yaml** | **1.5 ms** | — |
| PyYAML | 57.7 ms | 慢 38 倍 |
| ruamel.yaml | 127.9 ms | 慢 85 倍 |

#### 序列化速度（大 YAML，约 2 KB）

| 库 | 时间 | 加速比 |
|----|------|--------|
| **pyrs-yaml** | **0.17 ms** | — |
| PyYAML | 30.2 ms | 慢 177 倍 |
| ruamel.yaml | 63.1 ms | 慢 371 倍 |

#### 往返速度（大 YAML，约 2 KB）

| 库 | 时间 | 加速比 |
|----|------|--------|
| **pyrs-yaml** | **1.6 ms** | — |
| PyYAML | 87.9 ms[^1] | 慢 55 倍 |
| ruamel.yaml | 191.0 ms[^1] | 慢 119 倍 |

[^1]: PyYAML/ruamel 的往返时间为同一基准测试中解析与序列化时间之和的估算值。

### 功能对比

| 功能 | pyrs-yaml | PyYAML | ruamel.yaml |
|------|-----------|--------|-------------|
| **YAML 1.2 合规** | :material-check: | :material-check: | :material-check: |
| **独立注释** | :material-check: | :material-close: | :material-check: |
| **内联注释** | :material-check: | :material-close: | :material-check: |
| **锚点/别名** | :material-check: | :material-close: | :material-check: |
| **标签（显式）** | :material-check: | :material-close: | :material-check: |
| **块标量** | :material-check: | :material-check: | :material-check: |
| **流集合** | :material-check: | :material-check: | :material-check: |
| **合并键（<<）** | :material-check: | :material-close: | :material-check: |
| **复杂键** | :material-check: | :material-check: | :material-check: |
| **往返保留** | :material-check: | :material-close: | :material-check: |
| **Python 绑定** | :material-check: | :material-check: | :material-check: |
| **ABI3（py3.8+）** | :material-check: | :material-close: | :material-close: |
| **类型存根（.pyi）** | :material-check: | :material-check: | :material-close: |
| **国际化错误消息** | :material-check: | :material-close: | :material-close: |
| **Rust 后端** | :material-check: | :material-close: | :material-close: |
| **性能** | :material-rocket-launch: 最快 | :material-snail: 慢 | :material-snail: 慢 |

### 总结

#### 选择 pyrs-yaml 的场景

- **性能至关重要** — 比 PyYAML 解析快 21–43 倍、序列化快 55–177 倍
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

```python
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
