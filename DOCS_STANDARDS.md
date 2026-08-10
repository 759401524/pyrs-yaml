# Agent 指令：Zensical & Markdown 文档编写规范

当被要求编写、修改或生成 Zensical / MkDocs Material Markdown 文档时，**必须严格遵循**以下规则。这些规则基于 Python Markdown 引擎与 Zensical 扩展机制，确保生成的文档语法正确、机器可读，并能被静态站点生成器正确渲染。

适用范围：本仓库 `docs/{en,ja,ko,zh}/`（站点由 Zensical 构建，配置见 `zensical.toml`）及 Agent 生成的任何 Markdown。

## 1. 核心约束

- **结构完整性**：绝对不得破坏 Markdown 的缩进、空行、代码围栏（```）、列表层级或组件边界。
- **机器可读性**：语法标识符、文件路径、代码块语言声明、标签名必须保持绝对准确，禁止使用占位符。
- **扩展感知**：使用 Admonitions、Content Tabs 等高级语法前，默认当前环境已启用对应的 `pymdownx` 扩展。

## 2. Frontmatter 规则

每个文档**必须**以 YAML Frontmatter 开始，包裹在 `---` 之间。

- **必填字段**：必须包含 `title` 和 `description`。
- **原生字段**：仅使用 `title`, `description`, `tags`, `icon`, `hide`, `status`, `search` 等 Zensical 原生支持字段。
- **禁用非标准字段**：**禁止**使用 `categories` 字段（除非明确指令说明项目安装了分类插件），如需分类请使用 `tags` 替代。

生成模板：

```yaml
---
title: 页面标题
description: 页面描述。
tags:
  - docs
status: active
---
```

## 3. Snippet 引用规则

当需要复用外部文件内容时，使用 Snippet 语法。

- 语法格式：`--8<-- "path/to/file.ext"` （**必须**包含双引号）。
- 路径必须相对于文档根目录（`docs/`）。
- **禁止幻觉**：必须基于已知文件树生成路径，若无法验证文件存在，改用内联代码块代替。

## 4. Admonitions (提示框) 规则

用于突出显示提示、警告或注意事项。

- `!!!`：默认展开的提示框。
- `???`：可折叠的提示框，默认折叠。
- `???+`：可折叠的提示框，**默认展开**（适用于重要警告或长篇规则）。
- **致命缩进规则**：标识符（`!!!` / `???` / `???+`）下方的所有内容**必须统一缩进 4 个空格**。禁止使用 Tab 缩进，禁止 2 空格缩进。

生成示例：

```markdown
!!! note "标题"
    正文内容必须缩进4个空格。

???+ warning "可折叠且默认展开"
    此内容默认可见，用户可手动折叠。正文必须缩进4个空格。
```

## 5. Content Tabs (标签页) 规则

当同一主题存在多种语言、工具或环境示例时，必须使用 Content Tabs。

- 语法：`=== "Tab名称"`。
- 依赖：此语法需项目启用 `pymdownx.tabbed` 扩展（并设置 `alternate_style: true`）。
- **致命缩进规则**：`===` 下方的所有内容**必须统一缩进 4 个空格**。

生成示例：

```markdown
=== "Python"
    ```python
    def hello() -> str:
        return "Hello"
    ```
    示例说明（同样需缩进4空格）。

=== "Rust"
    ```rust
    fn main() {
        println!("Hello");
    }
    ```
```

## 6. Code Blocks (代码块) 规则

- **语言必填**：围栏代码块（```）必须声明语言类型（如 `python`, `bash`, `text`, `yaml`, `toml`）。
- **行内代码强制要求**：API 名称、字段名、命令名、错误名、文件路径**必须**使用反引号包裹（例如 `YamlDocument`、`ValueError`、`pyproject.toml`）。
- 代码注释应简洁准确，区分命令与终端输出。

## 7. Links & Paths (链接与路径) 规则

- **内部链接**：使用项目内相对路径，例如 `[规则说明](rules/syntax.md)`。
- **稳定锚点策略**：因 Zensical 对中文标题自动生成的 slug 不稳定，**必须**采用以下两种方式之一生成稳定锚点：
  1. **显式 HTML ID**：在目标位置添加 `<span id="target-id"></span>`，并使用 `[链接](#target-id)` 指向它。
  2. **Attribute Lists**：在标题末尾追加 `{ #target-id }`（需启用 `attr_list` 扩展），例如 `## 标题 { #target-id }`。
- 链接文本必须具备可读性，禁止使用 "点击这里" 等无意义文本。

## 8. HTML & Shortcodes 规则

- 仅在原生 Markdown 无法满足表达需求时使用 HTML 或 Shortcodes。
- HTML 标签必须完整闭合，属性必须带引号。
- 宏调用（如 `{{ macro() }}`）必须确认已在环境中注册，禁止输出未知的 Jinja 模板语法。

## 9. 生成前自检清单（强制执行）

在输出最终 Markdown 内容前，**必须**作为代码审查者静默执行以下校验循环。若检查未通过，需自行修正后再输出：

1. [ ] **Frontmatter**：是否位于文件首行？是否剔除了 `categories` 等非标准字段？
2. [ ] **Snippet**：`--8<--` 语法是否正确包含双引号？路径是否基于真实存在上下文？
3. [ ] **缩进深度**：所有 `!!!`, `???`, `???+`, `===` 内部的内容，是否精确使用了 **4 个空格** 缩进？（禁止 Tab，禁止 2 空格）
4. [ ] **代码块**：围栏代码是否声明了语言？行内代码是否包裹了所有路径和 API？
5. [ ] **锚点**：内部跳转链接是否指向了显式的 HTML `id` 或 `{ #id }`，而非中文自动 slug？
6. [ ] **结构闭合**：所有 HTML 标签是否闭合？代码围栏是否成对出现？
