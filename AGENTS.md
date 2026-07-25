# Agent 开发指南：`pyyaml-rs`

## 1. 项目愿景与核心目标

- **核心目标**：构建一个 **完全自研底层逻辑** 的高性能 Python YAML 处理库。
- **核心卖点**：
  1. **极致的定制权**：通过自定义 AST（抽象语法树），实现业务特定的节点操作与格式化。
  2. **完美的 Round-Trip（往返解析）**：100% 保留原有的注释、空行、缩进、引号风格（单/双/无引号）和键值顺序。
  3. **高性能**：利用 Rust 的零拷贝和内存安全特性，超越纯 Python 实现。
  4. **YAML 1.2 合规**：使用 saphyr-parser，通过 98.1% 的 YAML Test Suite。

---

## 2. 技术栈与依赖

```toml
[dependencies]
# PyO3: 必须包含 "extension-module" 特性
pyo3 = { version = "0.21", features = ["extension-module"] }

# indexmap: 用于保证 Mapping (字典) 的插入/解析顺序
indexmap = { version = "2.2", features = ["serde"] }

# saphyr-parser: YAML 1.2 完全合规的解析器 (Event-based API)
saphyr-parser = "0.0.11"

# serde_json: JSON 转换支持
serde_json = "1.0"
```

🚨 **绝对禁止**：
- **禁止** 使用 `serde_yaml`、`serde_yml` 或任何基于 `serde` 的 YAML 库。
- **禁止** 使用 `yaml-rust2` 的高级 API `YamlLoader::load_from_str()`。

---

## 3. 核心架构规范

### 3.1 自定义 AST (`src/ast.rs`)
- **规则**：AST 节点 (`CustomNode`) 必须包含足够的元数据以支持 Round-Trip。
- **必须包含的元数据**：
  - `ScalarStyle`：区分 Plain, SingleQuoted, DoubleQuoted, Literal (`|`), Folded (`>`)。
  - `Chomping`：块标量的 chomping 指示符 (Strip, Clip, Keep)。
  - `comment`：记录行尾注释或独立行注释。
  - `anchor` / `alias`：记录锚点名称和别名引用。
  - `tag`：记录 YAML 标签。
- **数据结构**：Mapping 必须使用 `IndexMap<CustomNode, CustomNode>` 以保持顺序。

### 3.2 解析器 (`src/parser/`)
- **规则**：基于 saphyr-parser 的 Event API 构建 AST。
- **模块结构**（单一职责原则）：
  ```
  src/parser/
  ├── mod.rs              # 核心解析逻辑 (AstReceiver)
  └── yaml/
      ├── comment.rs      # 注释提取与匹配
      ├── merge.rs        # 合并键 (<<) 解析
      ├── scalar.rs       # 转义序列 & chomping 检测
      └── types.rs        # YAML 1.2 类型解析
  ```

### 3.3 序列化器 (`src/serializer.rs`)
- **规则**：完全掌控输出格式，禁止依赖任何第三方 dump 函数。
- **状态管理**：必须维护 `current_indent_level`（当前缩进层级）和 `indent_size`（每层空格数，通常为 2）。

---

## 4. Rust 与 PyO3 编码规范

### 4.1 错误处理
- **禁止** 在业务逻辑中使用 `.unwrap()` 或 `.expect()`。
- **必须** 将所有可能失败的 Rust 操作映射为 Python 异常。

### 4.2 GIL 与性能
- **规则**：在进行耗时的纯 Rust 计算时，**必须释放 Python GIL**。

### 4.3 文档规范
- 所有公开函数必须有 `///` 文档注释
- 包含 `# Arguments`、`# Returns`、`# Errors`、`# Examples` 部分
- 使用 `cargo clippy -- -D warnings` 确保无警告

---

## 5. 测试驱动开发

### 5.1 测试用例设计原则
1. **Round-Trip 断言**：最核心的测试。
2. **边缘情况覆盖**：特殊字符、多行字符串、锚点、空值等。

---

## 6. Agent 标准工作流

1. **需求分析与架构评估**
2. **编写失败测试 (Red)**
3. **实现代码 (Green)**
4. **本地验证 (Refactor)**
5. **输出报告**

---

## 7. 常见陷阱与避坑指南

| 现象 | 根本原因 | 解决方案 |
| :--- | :--- | :--- |
| **解析后注释全部丢失** | 未正确提取和附加注释 | 检查 `comment.rs` 中的注释提取逻辑 |
| **字典键的顺序在 Python 端乱了** | `CustomNode::Mapping` 使用了 `HashMap` | 必须使用 `indexmap::IndexMap` |
| **Python 端调用时卡死/极慢** | 未释放 GIL | 使用 `py.allow_threads(|| { ... })` |

---

> **💡 Agent 终极心法**：
> "我是 YAML 格式的绝对掌控者。我使用 saphyr-parser 实现 YAML 1.2 合规，通过自定义 AST 记忆灵魂（注释与格式），通过序列化器重塑肉身。我的代码没有 `.unwrap()`，我的测试覆盖所有边缘情况。"
