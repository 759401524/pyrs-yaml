# I18n 翻译规范

本文档定义 pyrs-yaml 项目的国际化（I18n）翻译规范，确保所有语言版本的文档保持一致的术语、格式和结构。

---

## 1. 基本原则

### 1.1 技术标识符不翻译

以下内容**必须保留英文原文**：

- **语言与生态**：Python, Rust, PyO3, CPython, Cargo, pip, PyPI, maturin, wheel, manylinux
- **PyO3/Rust 标识符**：`PyObject`, `PyResult`, `PyErr`, `GIL`, `#[pyfunction]`, `#[pyclass]`, `crate`, `trait`, `ABI3`
- **Python 异常名**：`ValueError`, `TypeError`, `RuntimeError`, `KeyError`, `AttributeError`（作为类名或代码标识时）
- **API 与字段名**：`from_python()`, `into_py()`, `user_id`, `timeout_ms`
- **CLI 与配置**：`pip install`, `cargo build`, `maturin develop`, `pyproject.toml`, `PYTHONPATH`
- **项目内部名称**：`CustomNode`, `YamlDocument`, `IndexMap`, `saphyr-parser`, `AST`

### 1.2 代码示例保持可执行

- 代码块（````python`, ````rust`, ````bash`）内部的代码、命令行、终端输出**绝对不可翻译**
- 代码注释默认不翻译，保持原文以防破坏代码对齐
- 行内代码（`` `PyObject` ``, `` `ValueError` ``）绝对不可翻译

### 1.3 Markdown 语法保护

- **Snippet 引入语法**：`--8<-- "examples/hello.py"`，路径与语法标识符不可翻译
- **Admonitions**：`!!! note "Title"` 中的 `note` 类型标识符不可翻译，仅 `"Title"` 可翻译
- **Content Tabs**：`=== "Python"` 中的编程语言名不可翻译
- **链接路径**：`[Text](path/to/file.md)` 中的 `path/to/file.md` 不可翻译
- **Frontmatter**：`title`, `description` 可翻译；`tags`, `categories` 等标识符不可翻译

---

## 2. 术语标准译法

| 英文术语 | 中文 (zh) | 日文 (ja) | 韩文 (ko) | 备注 |
|---------|----------|----------|----------|------|
| round-trip | 往返 | ラウンドトリップ | 순환 | 统一简称 |
| chomping | chomping 指示符 | チョンピング 인ジ케이터 | 촙핑 지시자 | 保留英文 |
| frontmatter | Front Matter | Front Matter | Front Matter | 保留英文 |
| parse | 解析 | パース | 파싱 | |
| serialize | 序列化 | シリアライ즈 | 직렬화 | |
| scalar | 标量 | スカラー | 스칼라 | |
| mapping | 映射 | マッピング | 매핑 | |
| sequence | 序列 | シーケンス | 시퀀스 | |
| anchor | 锚点 | アンカー | 앵커 | |
| alias | 别名 | エイリア스 | 별칭 | |
| comment | 注释 | コメント | 주석 | |
| tag | 标签 | タグ | 태그 | |
| schema | 模式 | スキーマ | 스키마 | |
| backend | 后端 | 백エンド | 백엔드 | |
| binding | 绑定 | バインディング | 바인딩 | |
| runtime | 运行时 | ランタイム | 런타임 | |
| interpreter | 解释器 | インタープリ터 | 인터프리터 | |
| GIL | GIL | GIL | GIL | 保留英文 |
| CustomNode | CustomNode | CustomNode | CustomNode | 保留英文 |
| YamlDocument | YamlDocument | YamlDocument | YamlDocument | 保留英文 |
| IndexMap | IndexMap | IndexMap | IndexMap | 保留英文 |
| PyO3 | PyO3 | PyO3 | PyO3 | 保留英文 |

---

## 3. 文档结构规范

### 3.1 标题层级

- 页面标题使用 `#` (H1)
- 主要章节使用 `##` (H2)
- 子章节使用 `###` (H3)
- **不允许跳过层级**（如 H1 后直接 H3）

### 3.2 Frontmatter

每个翻译的 markdown 文件**必须**包含带有语言标识的 YAML frontmatter：

```markdown
---
title: 页面标题
lang: zh
---
```

支持的语言代码：

- `en` — 英文（源语言）
- `zh` — 中文
- `ja` — 日文
- `ko` — 韩文

### 3.3 章节对齐

- 目标语言应与英文源文件保持相同的 `##` (H2) 章节结构
- 目标语言**可以**添加额外的 `###` (H3) 子章节补充说明
- 目标语言**可以**在页面顶部添加 H1 标题（英文源文件通常已有）
- 目标语言**不应**删除英文源文件中的任何 H2 章节

### 3.4 额外内容

目标语言可以添加英文源文件没有的 H2 章节，但需遵循以下规则：

**允许的情况**：

- 目标语言读者特有的技术说明（如中文的"重复键"、"序列化选项"）
- 翻译示例和对照（如 i18n.md 的翻译前后对比）
- 目标语言生态系统的特定说明

**要求**：

1. 确保内容是有价值的技术补充，而非重复或冗余
2. 在 PR 描述中说明添加原因
3. 高价值内容应考虑反向翻译到英文源文件
4. 使用 `scripts/check_i18n.py` 验证，确保差异在允许范围内

**自动化检查**：

- `scripts/check_i18n.py` 会检测章节数量差异
- 允许的额外章节数配置在 `ALLOW_EXTRA_SECTIONS` 中
- 超出阈值的差异将导致检查失败

---

## 4. 翻译流程

### 4.1 新增翻译

1. 在 `docs/{locale}/` 目录下创建对应文件
2. 添加正确的 frontmatter（`title` + `lang`）
3. 翻译所有用户可见文本
4. 保留所有技术标识符、代码块、链接路径
5. 运行 `python scripts/check_i18n.py` 验证

### 4.2 更新翻译

1. 当英文源文件变更时，同步更新所有目标语言文件
2. 检查新增章节是否需要翻译
3. 检查术语是否一致
4. 运行验证脚本

### 4.3 质量检查

提交 PR 前运行以下检查：

```bash
# 运行自动化 I18n 检查
python scripts/check_i18n.py

# 运行 changelog 镜像检查
python scripts/check_changelog_mirrors.py

# 运行测试
uv run pytest tests/ -v
```

### 4.4 自动化保障

项目配置了以下自动化检查：

- **Pre-commit Hook**：`prek.toml` 中配置了 `i18n-check` hook，在提交时自动运行 `check_i18n.py`
- **CI 检查**：`.github/workflows/ci.yml` 中配置了 `i18n-check` job，在 PR 时自动运行
- **Changelog 镜像检查**：`check_changelog_mirrors.py` 确保 changelog 结构对齐

---

## 5. 占位符规范

### 5.1 Rust 格式占位符

- 格式：`%{key}` 或 `{}` 或 `{:?}`
- **不可丢失**：翻译后必须保留相同数量和名称的占位符
- **不可翻译变量名**：如 `%{detail}` 中的 `detail` 不可翻译

示例：

```yaml
# 正确
yaml-parse-error: "YAML 解析错误: %{detail}"

# 错误（变量名被翻译）
yaml-parse-error: "YAML 解析错误: %{详情}"
```

### 5.2 Python 格式占位符

- 格式：`{name}`, `%s`, `%d`
- 同上规则

---

## 6. 常见问题

### Q: 是否可以添加英文没有的章节？

A: 可以，但应满足以下条件：

- 内容是目标语言读者特有的有价值补充
- 不会与英文源文件内容冲突
- 在 PR 中说明添加原因

### Q: 术语翻译与术语表不一致怎么办？

A: 以本规范术语表为准。如发现更好的译法，请提交 PR 更新术语表。

### Q: 代码示例中的注释需要翻译吗？

A: 默认不翻译。如果注释包含重要解释且翻译有助于理解，可以翻译，但需确保：

- 不破坏代码对齐
- 不影响代码可执行性

---

## 7. 维护

本规范由项目维护者定期更新。如有疑问或建议，请提交 Issue 或 PR。

**最后更新**: 2026-08-10
