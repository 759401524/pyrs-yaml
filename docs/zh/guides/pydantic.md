---
title: Pydantic 集成
description: 将 YAML 解析为 Pydantic v2 模型、将模型序列化为 YAML，并使用 pyrs-yaml 作为解析器通过 pydantic-settings 加载 BaseSettings。
tags:
  - docs
status: new
---

## Pydantic 集成

pyrs-yaml 与 [Pydantic](https://docs.pydantic.dev/) v2 及
[pydantic-settings](https://docs.pydantic.dev/latest/concepts/pydantic_settings/) 集成，
可将 YAML 转换为经过验证的模型，反之亦然。两者均为可选依赖：

- 模型解析与序列化：`pip install pydantic`（或 `pip install 'pyrs-yaml[pydantic]'`）
- 加载 `BaseSettings`：`pip install 'pyrs-yaml[settings]'`（会安装 `pydantic-settings`）

### 将 YAML 解析为模型

`parse_as()` 解析 YAML 并针对 Pydantic `BaseModel` 子类进行验证，返回模型实例。
所有 `**yaml_kwargs` 会被转发给 `YAML()` 构造函数（例如 `resolve_merges`）。

```python title="Parse YAML into a Pydantic model"
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


user = pyrs_yaml.parse_as(User, "name: Alice\nage: 30")
print(user.name)  # Alice
print(user.age)  # 30
```

`parse_as()` 会抛出：

- `ImportError` — 未安装 pydantic
- `TypeError` — `model` 不是 `BaseModel` 子类
- `pydantic.ValidationError` — 解析后的数据未通过模型验证

### 将模型序列化为 YAML

`dump_pydantic()` 将 Pydantic 模型序列化为 YAML 字符串。它首先调用
`model_dump(mode="json")`，以保持字符串类型字段为字符串（例如 `"10001"` 这样的邮政编码
不会被强制转为整数），然后委托给 `safe_dump`。

```python title="Serialize a Pydantic model to YAML"
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


yaml_str = pyrs_yaml.dump_pydantic(User(name="Alice", age=30))
print(yaml_str)
# name: Alice
# age: 30
```

`dump_pydantic()` 会抛出：

- `ImportError` — 未安装 pydantic
- `TypeError` — `model` 不是 `BaseModel` 实例

### 使用 pydantic-settings 加载设置

`PyrsYamlConfigSettingsSource` 是 `pydantic_settings.YamlConfigSettingsSource`
的直接替代品。它使用 pyrs-yaml 的 YAML 1.2 解析器读取 YAML 配置文件，然后将值
与环境变量、dotenv 和密钥一起 feed 给 `BaseSettings` 模型 —— 优先级和行为完全相同。

```python title="Load BaseSettings from a YAML file"
from pydantic_settings import BaseSettings, SettingsConfigDict
import pyrs_yaml


class Settings(BaseSettings):
    app_name: str

    model_config = SettingsConfigDict(yaml_file="config.yaml")

    @classmethod
    def settings_customise_sources(
        cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
    ):
        return (
            init_settings,
            env_settings,
            dotenv_settings,
            file_secret_settings,
            pyrs_yaml.PyrsYamlConfigSettingsSource(settings_cls),
        )
```

该源支持与 `YamlConfigSettingsSource` 相同的选项：

- `yaml_file` — 单个路径或路径列表（通过 `SettingsConfigDict` 声明，或直接传入）
- `yaml_file_encoding` — 文件编码
- `yaml_config_section` — 点表示法的嵌套 section 路径
- `deep_merge` — 以深度合并多个文件，而非覆盖

!!! note "延迟导入"
    `import pyrs_yaml` 本身并不需要 pydantic 或 pydantic-settings。在未安装对应
    依赖的情况下访问 `pyrs_yaml.parse_as`、`pyrs_yaml.dump_pydantic` 或
    `pyrs_yaml.PyrsYamlConfigSettingsSource`，会抛出带有安装提示的 `ImportError`。

### 带注释的往返

由于 `parse_as()` 基于 `safe_load`，因此注释和锚点的保留不在模型路径的
考虑范围内 —— 需要往返编辑时请使用 `parse()` 配合 `YamlDocument`，
仅在需要经过验证的模型时使用 `parse_as()`。

!!! tip "选择正确的解析路径"
    配置验证使用 `parse_as()`；当注释、锚点或格式需要在往返中保留时使用 `parse()`。

### 另见

- [解析 YAML](parsing.md) — 解析字符串、文件和多个文档
- [序列化](serialization.md) — 在 YAML 文档与 Python 对象之间转换
- [配置管理](tutorial-config-management.md) — 端到端 walkthrough
- [API 参考](../api/reference.md) — `parse_as`、`dump_pydantic` 和 `PyrsYamlConfigSettingsSource` 的完整签名
