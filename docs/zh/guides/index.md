---
title: 用户指南
description: 使用 pyrs-yaml 解析、序列化、编辑和集成 YAML 的指南。
tags:
  - docs
status: new
---

## 用户指南

用户指南分为两个部分：

### 核心

核心 YAML 操作 — 解析、序列化、往返保留、就地编辑和流式解析。

- [解析 YAML](parsing.md) — 解析字符串、文件和多个文档
- [序列化](serialization.md) — 在 YAML 文档和 Python 对象之间转换
- [PyYAML 兼容](pyyaml-compat.md) — 直接替换 API
- [往返保留](round-trip.md) — 注释、锚点、标签和格式全部保留
- [就地编辑](editing.md) — 通过 JSONPath 路径编辑文档，不丢失格式
- [流式解析](streaming.md) — 常量内存的增量解析

### 集成

高级功能 — 自定义 Schema、插件开发、社区插件、配置管理、Markdown 头信息、i18n 错误消息和 NumPy ndarray 支持。

- [自定义 Schema](custom-schema.md) — 定义自定义 YAML Schema 用于类型解析
- [插件开发](plugin-development.md) — 构建自定义标签处理器和节点类型
- [社区插件](community-plugins.md) — datetime、UUID、decimal 等内置插件
- [配置管理](tutorial-config-management.md) — 端到端实战
- [Markdown 头信息](frontmatter.md) — 从 Markdown 文件中提取 YAML 头信息
- [i18n 错误消息](i18n.md) — 本地化错误消息
- [NumPy ndarray](numpy.md) — 将 numpy 数组序列化为 YAML
- [Pydantic 集成](pydantic.md) — 将 YAML 解析为 Pydantic 模型并加载 BaseSettings
