---
title: 使用 pyrs-yaml 进行配置管理
description: 端到端教程，展示如何解析、编辑、校验并重新序列化 YAML 配置文件，完整保留所有元数据。
tags:
  - docs
  - tutorial
status: new
---

## 使用 pyrs-yaml 进行配置管理

本教程通过一个真实场景：管理微服务应用的 YAML 配置文件。您将学习如何解析、查看、编辑、校验并写回 YAML 文件——同时保留每一条注释、锚点、标签和格式选择。

## 环境搭建

```bash title="从 PyPI 安装"
pip install pyrs-yaml
```

## 1. 配置文件

我们从一个包含注释、锚点、merge 键以及块式/流式混合格式的 YAML 配置文件开始：

```yaml title="config.yaml"
# Application configuration (v2.0)
app:
  name: my-service
  version: 2.0

# Default database settings
database: &default-db
  host: localhost
  port: 5432
  name: mydb

# Environment-specific overrides
environments:
  staging:
    <<: *default-db
    host: staging.example.com
    debug: true

  production:
    <<: *default-db
    host: prod.example.com
    port: 5432
    debug: false

# Feature flags
features:
  - name: login
    enabled: true
  - name: export
    enabled: true
  - name: reporting
    enabled: false

# Custom scalar example
threshold: 0x1F  # hex value (should be parsed as int)
```

## 2. 解析文件

```python title="解析文件"
import pyrs_yaml

doc = pyrs_yaml.parse_file("config.yaml")
print(f"Parsed: {doc.get('app.name')} v{doc.get('app.version')}")
# Parsed: my-service v2.0
```

**要点**：所有注释、锚点、标签和格式都会在内存中保留。该文档是一个 `YamlDocument` 对象，而非原始 Python dict。

## 3. 查看值

使用路径 API（类 JSONPath）或 Node API（基于树）：

```python title="查看值"
# Path API — simple and direct
db_host = doc.get("database.host")
print(f"Database host: {db_host}")

# Node API — access metadata and formatting
db_node = doc.node().find("$.database")
print(f"Database is flow style: {db_node.flow_style}")  # False (block)
print(f"Database anchor: {db_node.anchor}")  # "default-db"
```

## 4. 编辑值并保留元数据

编辑值时，其注释、锚点、标签和引用样式都会被保留。编辑直接在 AST 上进行——不涉及字符串操作：

```python title="就地编辑值"
# Change the production port
doc.set("$.environments.production.port", 5444)

# Change the app name while keeping its comment
doc.set("$.app.name", "my-service-v2")

# Add a comment to document a change
prod_node = doc.node().find("$.environments.production")
prod_node.set_comment("overridden for v2 rollout")
```

## 5. 操作元数据

pyrs-yaml 不止于值编辑——您还可以读写 YAML 元数据本身：

```python title="读写元数据"
# Read existing metadata
debug_node = doc.node().find("$.environments.staging.debug")
print(f"Debug comment: {debug_node.comment}")  # None

# Add a tag to document a custom type
import_node = doc.node().find("$.threshold")
import_node.set_tag("!!int")
print(f"Threshold tag: {import_node.tag}")  # "!!int"

# Add an anchor for later reference
prod_db = doc.node().find("$.environments.production")
prod_db.set_anchor("prod-db")
```

## 6. 控制格式

切换标量引用方式、块式/流式布局和 chomping 指示符：

```python title="控制格式"
# Switch the threshold to single-quoted for clarity
doc.node().find("$.threshold").set_scalar_style("single_quoted")

# Switch the staging environment to compact flow style
staging = doc.node().find("$.environments.staging")
staging.set_flow_style(True)
```

## 7. 使用通配符批量编辑

使用 `set_many` 对每个匹配的路径应用更改——适合开关类操作：

```python title="通配符批量编辑"
# Disable ALL debug flags across every environment
doc.set_many(
    {
        "$.environments[*].debug": False,
    }
)

# Disable all features at once
doc.set_many(
    {
        "$.features[*].enabled": False,
    }
)
```

## 8. 排序键

为便于阅读，对顶层键和环境键排序：

```python title="排序键"
doc.sort_keys()  # sort the root mapping
doc.sort_keys("$.environments")  # sort the environments
```

## 9. 根据 Schema 校验

定义带结构规则的 schema 并校验配置：

```python title="根据 schema 校验"
schema = """\
name: app-config
extends: core
validate:
  - path: $.app.name
    type: str
    required: true
  - path: $.environments.*.debug
    type: bool
  - path: $.threshold
    type: int
"""

# Validate — raises YamlValidateError on failure
pyrs_yaml.validate_against_schema(doc.to_yaml(), schema)
print("Configuration is valid!")
```

## 10. 深度复制子树

将子树复制为独立的 Python 值（与文档分离）：

```python title="深度复制子树"
# Copy the staging configuration for reuse
staging_config = doc.node().find("$.environments.staging").copy()
print(staging_config)  # {'host': 'staging.example.com', 'debug': False, ...}
```

## 11. 移动子树

在同一文档内移动子树：

```python title="移动子树"
# Move the reporting feature to a new section
doc.node().find("$.features[2]").move("$.deprecated-features")
```

## 12. 写回文件

最后，将编辑后的文档序列化回 YAML：

```python title="写回文件"
output = doc.to_yaml()
with open("config-updated.yaml", "w", encoding="utf-8") as f:
    f.write(output)
```

输出会保留**一切**——注释、锚点、merge 键、格式以及我们做出的所有修改：

```yaml title="config-updated.yaml"
# Application configuration (v2.0)
app:
  name: my-service-v2
  version: 2.0

# Default database settings
database: &default-db
  host: localhost
  port: 5432
  name: mydb

# Environment-specific overrides
environments:
  # overridden for v2 rollout
  production: &prod-db
    <<: *default-db
    host: prod.example.com
    port: 5444
    debug: false

  staging:
    <<: *default-db
    debug: false
    host: staging.example.com
```

## 总结

在本教程中，您：

- :material-file-code: **解析**了 YAML 文件，完整保留元数据
- :material-magnify: 使用路径 API 和 Node API **查看**了值
- :material-pencil: **编辑**了值、注释、锚点、标签和格式
- :material-format-list-bulleted: 使用 `set_many` **批量编辑**（通配符）
- :material-sort: 为可读性**排序**了键
- :material-check-decagram: 根据 schema 进行了**校验**
- :material-content-copy: **复制**并**移动**了子树
- :material-sync: **序列化**回 YAML，所有内容都保留了下来

### 下一步

- :material-rocket-launch: [快速开始](../quick-start.md) — 几分钟内上手
- :material-pencil: [就地编辑指南](../guides/editing.md) — 完整编辑 API 参考
- :material-check-decagram: [自定义 Schema 指南](../guides/custom-schema.md) — 定义您自己的 schema
- :material-book-open-page-variant: [API 参考](../api/reference.md) — 完整 API 文档
