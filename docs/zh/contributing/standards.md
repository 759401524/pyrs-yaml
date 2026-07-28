---

title: 编码标准
lang: zh

## 编码标准

贡献 pyrs-yaml 时请遵循以下标准。

### Rust

#### 风格

- 提交前使用 `cargo fmt`
- 遵循 [Rust API 指南](https://rust-lang.github.io/api-guidelines/)
- `#[allow(unused_imports)]` 仅在必要时使用（测试、特性标志）

#### 错误处理

- **绝不在业务逻辑中使用 `.unwrap()` 或 `.expect()`**
- 将所有 Rust 错误转换为 Python 异常
- 可能失败的函数使用 `PyResult<T>`
- 将特定错误映射到特定 Python 异常类型

```rust
// 正确
let content = std::fs::read_to_string(path)
    .map_err(|e| YamlParseError::new_err(format_i18n_error("file-read-error", ...)))?;

// 错误
let content = std::fs::read_to_string(path).unwrap();
```

#### 文档

- 所有公开函数必须有 `///` 文档注释
- 包含 `# Arguments`、`# Returns`、`# Errors`、`# Examples` 部分
- 文档注释用英文编写（Rust 惯例）
- 内部函数的文档注释可以用中文

```rust
/// 将 YAML 字符串解析为 CustomNode AST。
///
/// # Arguments
/// * `yaml` — YAML 内容字符串
///
/// # Returns
/// 解析后的 AST 根节点，失败时返回 `Err(String)`
///
/// # Errors
/// 返回格式为 `"YAML parse error: line N, col M: <msg>"` 的 `Err(String)`
///
/// # Examples
/// ```
/// let ast = pyrs_yaml::parser::parse("key: value").unwrap();
/// ```
pub fn parse(yaml: &str) -> Result<CustomNode, String> {
```

#### PyO3 签名注解

每个 `#[pyfunction]` 和 `#[pymethods]` 必须使用 `#[pyo3(signature = "...")]` 标注类型：

```rust
#[pyo3(signature = (yaml: "str", resolve_merges: "bool" = true, schema: "str" = "core") -> "YamlDocument")]
fn parse(...) -> YamlDocument { ... }
```

#### GIL 管理

- 在耗时计算期间使用 `py.detach()` 或 `py.allow_threads()` 释放 GIL
- 在文件 I/O 或解析期间绝不持有 GIL

```rust
// 正确
let ast = py.detach(|| {
    parser::parse_with_options(&yaml_str, resolve_merges)
        .map_err(|e| YamlParseError::new_err(...))?
})?;

// 错误 — 在解析期间持有 GIL
let ast = parser::parse_with_options(&yaml_str, resolve_merges)?;
```

#### Clippy

运行 `cargo clippy -- -D warnings` — 将所有警告视为错误。

### Python

#### 风格

- 遵循 [PEP 8](https://peps.python.org/pep-0008/)
- 到处使用类型提示
- 文档字符串使用 Google 风格
- 代码检查配置在 `ruff.toml` 中（运行 `ruff check`）

```python
def parse(yaml: str, resolve_merges: bool = True, schema: str = "core") -> YamlDocument:
    """将 YAML 字符串解析为 YamlDocument。

    Args:
        yaml: 包含 YAML 内容的字符串
        resolve_merges: 是否解析合并键（默认：True）
        schema: YAML 模式配置（"core"、"json"、"failsafe"、"yaml11"）

    Returns:
        包含解析后 YAML 的 YamlDocument

    Raises:
        YamlParseError: YAML 无效时
    """
```

#### 测试

- 代码前先写测试（TDD）
- 必要时使用 `uv run --frozen pytest` 和 fixture
- 测试边缘情况：空输入、特殊字符、大文档
- 包含往返保存断言
- Pytest 配置在 `pytest.ini` 中（asyncio_mode = auto，自定义标记）

### Git

- 提交消息使用命令式："Add feature X"（不是 "Added feature X"）
- 每次提交一个逻辑变更
- 提交前运行 `cargo test` 和 `uv run --frozen pytest tests/`

### 文档

- 更改行为时更新文档
- 使用可复制粘贴和运行的代码示例
- 保持示例简洁但完整
