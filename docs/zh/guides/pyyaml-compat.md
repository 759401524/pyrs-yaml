---
title: PyYAML 兼容性
description: pyrs-yaml 与 PyYAML 的兼容性说明，包含迁移指南和 API 差异对比。
tags:
  - docs
status: new
---

pyrs-yaml 提供了 PyYAML 的**直接替换**，使迁移变得简单。

## 简单迁移

```python title="简单迁移"
# 旧代码
import yaml

data = yaml.safe_load(yaml_text)
yaml_str = yaml.safe_dump(data)

# 新代码
import pyrs_yaml as yaml

data = yaml.safe_load(yaml_text)
yaml_str = yaml.safe_dump(data)
```

## 兼容 API

| PyYAML 函数 | pyrs-yaml 等价物 | 说明 |
|-------------|----------------|------|
| `yaml.safe_load()` | `pyrs_yaml.safe_load()` | :material-check: 完全相同 |
| `yaml.safe_loads()` | `pyrs_yaml.safe_loads()` | :material-check: 完全相同 |
| `yaml.safe_dump()` | `pyrs_yaml.safe_dump()` | :material-check: 完全相同 |
| `yaml.safe_dumps()` | `pyrs_yaml.safe_dumps()` | :material-check: 完全相同 |
| `yaml.load()` | `pyrs_yaml.safe_load()` | :material-alert: 使用安全变体 |
| `yaml.dump()` | `pyrs_yaml.safe_dump()` | :material-alert: 使用安全变体 |

## 主要区别

### pyrs-yaml 的优势

| 特性 | PyYAML | pyrs-yaml |
|------|--------|-----------|
| 往返 | :material-close: 丢失注释/锚点 | :material-check: 保留所有内容 |
| 性能 | 基准 | **解析快 21-43 倍、序列化快 55-177 倍** |
| 类型提示 | 部分支持 | :material-check: 完整 `.pyi` 桩文件 |
| ABI3 wheel | 无 | :material-check: 单个 wheel 支持所有 Python 版本 |
| i18n 错误 | :material-close: 仅英文 | :material-check: 英文 + 中文 |

#### 注意事项

1. **锚点/别名处理**: PyYAML 在往返时丢失锚点；pyrs-yaml 保留它们
2. **注释位置**: pyrs-yaml 在复杂嵌套结构中可能会重新排列某些注释
3. **流式风格**: 两者都保留，但输出格式可能略有不同
4. **错误消息**: pyrs-yaml 使用具有更多上下文的 i18n 错误消息

## 迁移检查清单

- [ ] 将 `import yaml` 替换为 `import pyrs_yaml as yaml`
- [ ] 测试所有 YAML 解析/保存工作流
- [ ] 验证往返输出符合预期
- [ ] 检查锚点/别名行为（如果使用）
- [ ] 检查自定义错误消息的错误处理

## 迁移示例

```python title="完整迁移示例"
# 旧代码
import yaml


def load_config(path):
    with open(path) as f:
        return yaml.safe_load(f)


def save_config(data, path):
    with open(path, "w") as f:
        yaml.safe_dump(data, f)


# 新代码
import pyrs_yaml


def load_config(path):
    return pyrs_yaml.parse_file(path).to_dict()


def save_config(data, path):
    pyrs_yaml.dump_file(data, path)
```

---

### 另请参阅

- [解析 YAML](parsing.md) — 高级解析选项
- [序列化](serialization.md) — 详细序列化选项
- [往返保留](round-trip.md) — 注释和锚点在往返中保留
