# Agent 开发指南：`pyamlium-custom` (自实现 PyO3 YAML 库)

## 1. 项目愿景与核心目标 (Project Context)

- **核心目标**：构建一个 **完全自研底层逻辑** 的高性能 Python YAML 处理库。
- **核心卖点**：
  1. **极致的定制权**：通过自定义 AST（抽象语法树），实现业务特定的节点操作与格式化。
  2. **完美的 Round-Trip（往返解析）**：100% 保留原有的注释、空行、缩进、引号风格（单/双/无引号）和键值顺序。
  3. **高性能**：利用 Rust 的零拷贝和内存安全特性，超越纯 Python 实现。
  4. **YAML 1.2 合规**：使用 saphyr-parser，通过 98.1% 的 YAML Test Suite。

---

## 2. 技术栈与依赖 (Dependencies)

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

🚨 **绝对禁止 (Strictly Forbidden)**：
- **禁止** 使用 `serde_yaml`、`serde_yml` 或任何基于 `serde` 的 YAML 库。它们在设计上就会丢弃注释和格式。
- **禁止** 使用 `yaml-rust2` 的高级 API `YamlLoader::load_from_str()`。

---

## 3. 核心架构规范 (Architecture Rules)

项目遵循 **模块化架构**，代码需拆分到独立的模块中：

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
- **实现路径**：
  1. 使用 `saphyr_parser::Parser` 的 Event-based API。
  2. 实现 `SpannedEventReceiver` 获取事件位置信息。
  3. 从原始文本提取注释和锚点名称。

### 3.3 序列化器 (`src/serializer.rs`)
- **规则**：完全掌控输出格式，禁止依赖任何第三方 dump 函数。
- **状态管理**：必须维护 `current_indent_level`（当前缩进层级）和 `indent_size`（每层空格数，通常为 2）。
- **标量处理**：
  - **单引号**：内部的单引号必须转义为两个单引号 (`''`)。
  - **双引号**：必须正确处理反斜杠转义 (`\n`, `\t`, `\\`, `\"`)。
  - **多行字符串 (`|` 和 `>`)**：必须精确计算基础缩进，并在换行时补充相应的空格。

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
- **规则**：在进行耗时的纯 Rust 计算（如大文件解析）时，**必须释放 Python GIL**。
  ```rust
  #[pyfunction]
  fn parse(py: Python, yaml: &str) -> PyResult<YamlDocument> {
      let ast = py.allow_threads(|| {
          parser::parse(yaml).map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
      })?;
      Ok(YamlDocument { ast })
  }
  ```

### 4.3 内存与类型转换
- 将 `CustomNode` 转换为 Python 对象时，优先使用 `PyDict` 和 `PyList`。
- 避免在 Python 和 Rust 之间频繁传递大型字符串。

---

## 5. 测试驱动开发 (TDD Requirements)

YAML 的边缘情况极多，**没有测试的代码等同于有 Bug 的代码**。

### 5.1 测试用例设计原则
1. **Round-Trip 断言**：最核心的测试。
   ```python
   def test_roundtrip_preserves_comments():
       original = "# Comment\nkey: value  # inline\n"
       doc = pyamlium_custom.parse(original)
       assert doc.to_yaml() == original
   ```
2. **边缘情况覆盖**：
   - 包含特殊字符的标量
   - 多行字符串及其不同的缩进级别
   - 嵌套的锚点和别名
   - 空值
   - 复杂键

---

## 6. Agent 标准工作流 (Agent Workflow)

1. **需求分析与架构评估**：
   - 确认需求是否影响 AST 结构。
   - 确认需求是否涉及解析或序列化逻辑。
2. **编写失败测试 (Red)**：在 `tests/` 目录下编写 Python 测试用例。
3. **实现代码 (Green)**：修改 `parser/` 或 `serializer.rs`。
4. **本地验证 (Refactor)**：
   - `cargo clippy -- -D warnings`
   - `maturin develop --release`
   - `pytest tests/`
5. **输出报告**：向用户说明修改了哪些模块。

---

## 7. 常见陷阱与避坑指南 (Troubleshooting)

| 现象 | 根本原因 | 解决方案 |
| :--- | :--- | :--- |
| **解析后注释全部丢失** | 未正确提取和附加注释 | 检查 `comment.rs` 中的注释提取逻辑 |
| **字典键的顺序在 Python 端乱了** | `CustomNode::Mapping` 使用了 `HashMap` | 必须使用 `indexmap::IndexMap` |
| **Python 端调用时卡死/极慢** | 未释放 GIL | 使用 `py.allow_threads(|| { ... })` |

---

> **💡 Agent 终极心法**：
> "我是 YAML 格式的绝对掌控者。我使用 saphyr-parser 实现 YAML 1.2 合规，通过自定义 AST 记忆灵魂（注释与格式），通过序列化器重塑肉身。我的代码没有 `.unwrap()`，我的测试覆盖所有边缘情况。"
