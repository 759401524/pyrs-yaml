# Agent 开发指南：`pyamlium-custom` (自实现 PyO3 YAML 库)

## 1. 项目愿景与核心目标 (Project Context)
- **核心目标**：构建一个 **完全自研底层逻辑** 的高性能 Python YAML 处理库。
- **核心卖点**：
  1. **极致的定制权**：通过自定义 AST（抽象语法树），实现业务特定的节点操作与格式化。
  2. **完美的 Round-Trip（往返解析）**：100% 保留原有的注释、空行、缩进、引号风格（单/双/无引号）和键值顺序。
  3. **高性能**：利用 Rust 的零拷贝和内存安全特性，超越纯 Python 实现。

---

## 2. 技术栈与依赖红线 (Dependencies & Constraints)

Agent 在修改 `Cargo.toml` 时，必须严格遵守以下依赖规则：

```toml
[dependencies]
# PyO3: 必须包含 "extension-module" 特性
pyo3 = { version = "0.21", features = ["extension-module"] }

# indexmap: 必须开启，用于保证 Mapping (字典) 的插入/解析顺序
indexmap = { version = "2.2", features = ["serde"] } 

# yaml-rust2: 仅作为底层词法分析器(Scanner)的参考或辅助，绝不使用其高级 API
yaml-rust2 = "0.9" 
```

🚨 **绝对禁止 (Strictly Forbidden)**：
- **禁止** 使用 `serde_yaml`、`serde_yml` 或任何基于 `serde` 的 YAML 库。它们在设计上就会丢弃注释和格式。
- **禁止** 使用 `yaml-rust2` 的高级 API `YamlLoader::load_from_str()`。该 API 会丢失注释和原始标量风格。**必须使用底层的 `Scanner` (Token 流) 或完全自写词法分析器。**

---

## 3. 核心架构规范 (Architecture Rules)

项目必须严格遵循 **三段式架构**，代码需拆分到独立的模块中：

### 3.1 自定义 AST (`src/ast.rs`)
- **规则**：AST 节点 (`CustomNode`) 必须包含足够的元数据以支持 Round-Trip。
- **必须包含的元数据**：
  - `ScalarStyle`：区分 Plain, SingleQuoted, DoubleQuoted, Literal (`|`), Folded (`>`)。
  - `comment`：记录行尾注释或独立行注释。
  - `anchor` / `alias`：记录锚点名称和别名引用。
- **数据结构**：Mapping 必须使用 `IndexMap<CustomNode, CustomNode>` 以保持顺序。

### 3.2 解析器 (`src/parser.rs`)
- **规则**：基于 Token 流构建 AST。
- **实现路径**：
  1. 使用 `yaml-rust2::yaml::Scanner` 获取 Token 流（如 `Token::Scalar`, `Token::Comment`, `Token::MappingStart`）。
  2. 编写状态机或递归下降解析器，将 Token 流转换为 `CustomNode`。
  3. 遇到 `Token::Comment` 时，必须将其挂载到当前上下文的节点上（行尾注释）或作为独立节点插入（独立行注释）。

### 3.3 序列化器 (`src/serializer.rs`)
- **规则**：完全掌控输出格式，禁止依赖任何第三方 dump 函数。
- **状态管理**：必须维护 `current_indent_level`（当前缩进层级）和 `indent_size`（每层空格数，通常为 2）。
- **标量处理**：
  - **单引号**：内部的单引号必须转义为两个单引号 (`''`)。
  - **双引号**：必须正确处理反斜杠转义 (`\n`, `\t`, `\\`, `\"`)。
  - **多行字符串 (`|` 和 `>`)**：必须精确计算基础缩进，并在换行时补充相应的空格，防止 YAML 解析器将其误判为层级变化。

---

## 4. Rust 与 PyO3 编码规范 (Coding Standards)

### 4.1 错误处理 (Error Handling)
- **禁止** 在业务逻辑中使用 `.unwrap()` 或 `.expect()`。
- **必须** 将所有可能失败的 Rust 操作映射为 Python 异常：
  ```rust
  // ✅ 正确
  let content = std::fs::read_to_string(path)
      .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
  
  // ❌ 错误
  let content = std::fs::read_to_string(path).unwrap();
  ```

### 4.2 GIL 与性能 (GIL & Performance)
- **规则**：在进行耗时的纯 Rust 计算（如大文件解析、复杂的 AST 遍历）时，**必须释放 Python GIL**，以避免阻塞 Python 主线程。
  ```rust
  #[pyfunction]
  fn parse_large_file(py: Python, path: &str) -> PyResult<PyYamlDocument> {
      // 释放 GIL 进行文件读取和解析
      let ast = py.allow_threads(|| {
          let content = std::fs::read_to_string(path).map_err(...)?;
          parse_to_custom_ast(&content).map_err(...)
      })?;
      Ok(PyYamlDocument { ast })
  }
  ```

### 4.3 内存与类型转换
- 将 `CustomNode` 转换为 Python 对象时，优先使用 `PyDict` 和 `PyList`。
- 避免在 Python 和 Rust 之间频繁传递大型字符串，尽量在 Rust 侧完成字符串的拼接和格式化。

---

## 5. 测试驱动开发 (TDD Requirements)

YAML 的边缘情况极多，**没有测试的代码等同于有 Bug 的代码**。Agent 在实现任何功能前，必须先编写 Python 测试。

### 5.1 测试用例设计原则
1. **Round-Trip 断言**：最核心的测试。
   ```python
   def test_roundtrip_preserves_comments():
       original = "key: value # 这是一个注释\n"
       doc = pyamlium_custom.parse(original)
       assert doc.to_yaml() == original
   ```
2. **边缘情况覆盖**：
   - 包含特殊字符的标量（如 `:`, `#`, `-`, `*`, `&`, `[`, `]`, `{`, `}`）。
   - 多行字符串（`|` 和 `>`）及其不同的缩进级别。
   - 嵌套的锚点 (`&anchor`) 和别名 (`*alias`)。
   - 空值 (`null`, `~`, 或留空)。
   - 键为复杂类型（如列表或字典作为键，虽然罕见但 YAML 规范允许）。

---

## 6. Agent 标准工作流 (Agent Workflow)

当接收到开发任务时，Agent 必须按以下步骤执行：

1. **需求分析与架构评估**：
   - 确认需求是否影响 AST 结构。如果需要新增元数据，先修改 `src/ast.rs`。
   - 确认需求是否涉及解析或序列化逻辑。
2. **编写失败测试 (Red)**：
   - 在 `tests/` 目录下编写 Python 测试用例，包含目标 YAML 字符串。
   - 运行 `pytest` 确认测试失败。
3. **实现代码 (Green)**：
   - 修改 `parser.rs` 或 `serializer.rs`。
   - **自检清单**：
     - [ ] 我是否使用了底层的 Token 流/Scanner 而不是 `YamlLoader`？
     - [ ] 我是否处理了所有 `ScalarStyle` 的转义逻辑？
     - [ ] 序列化器的缩进计算是否正确（特别是多行字符串）？
     - [ ] 我是否移除了所有的 `.unwrap()`？
4. **本地验证 (Refactor)**：
   - 运行 `cargo clippy -- -D warnings` 确保无警告。
   - 运行 `maturin develop --release` 重新编译。
   - 运行 `pytest tests/` 确保所有测试通过。
5. **输出报告**：
   - 向用户说明修改了哪些模块。
   - 提供修改前后的 YAML 对比（如果涉及格式变化），证明 Round-Trip 特性未被破坏。

---

## 7. 常见陷阱与避坑指南 (Troubleshooting)

| 现象 | 根本原因 | 解决方案 |
| :--- | :--- | :--- |
| **解析后注释全部丢失** | 错误地使用了 `yaml-rust2` 的 `YamlLoader`。 | 改用 `yaml-rust2::yaml::Scanner` 遍历 Token，手动捕获 `Token::Comment`。 |
| **重新生成的 YAML 缩进错乱** | 序列化器中 `current_indent_level` 计算错误，或多行字符串未补充基础缩进。 | 检查 `serializer.rs` 中的缩进逻辑。对于多行字符串，必须按行分割并手动 `push_str` 对应层级的空格。 |
| **字典键的顺序在 Python 端乱了** | `CustomNode::Mapping` 使用了 `HashMap` 或 `BTreeMap`。 | 必须使用 `indexmap::IndexMap`。 |
| **包含 `#` 或 `:` 的字符串被错误解析** | 词法分析器未正确处理引号内的特殊字符。 | 在 Parser 中，当处于 `SingleQuoted` 或 `DoubleQuoted` 状态时，忽略 `#` (注释) 和 `:` (键值分隔符) 的特殊语义。 |
| **Python 端调用时卡死/极慢** | 在解析大文件时未释放 GIL。 | 在 `pyo3` 的 `#[pyfunction]` 中，将核心解析逻辑包裹在 `py.allow_threads(|| { ... })` 中。 |

---

> **💡 Agent 终极心法**：
> "我是 YAML 格式的绝对掌控者。我不依赖黑盒库，我通过 Token 理解结构，通过 AST 记忆灵魂（注释与格式），通过序列化器重塑肉身。我的代码没有 `.unwrap()`，我的测试覆盖所有边缘情况。"
