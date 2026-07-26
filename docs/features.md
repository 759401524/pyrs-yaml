# Features

pyyaml-rs is designed to be a **drop-in replacement** for PyYAML while adding powerful features that PyYAML lacks.

## YAML 1.2 Compliance

Powered by **saphyr-parser**, pyyaml-rs achieves **98.1% pass rate** on the YAML Test Suite.

## Perfect Round-Trip

Unlike PyYAML, pyyaml-rs **preserves all formatting and metadata**:

- **Comments** — standalone and inline
- **Anchors** (`&name`) and **aliases** (`*name`)
- **Tags** (`!!str`, `!!int`, etc.)
- **Chomping indicators** (`\|-`, `\|+`, `>-`, `>+`)
- **Scalar styles** (plain, single-quoted, double-quoted, literal, folded)
- **Flow/block formatting** — `[]`/`{}` vs block style preserved

## Performance

Rust backend delivers **25–40× speedup** over PyYAML:

| Operation | pyyaml-rs | PyYAML |
|-----------|-----------|--------|
| Parse (large) | 0.07 ms | 1.83 ms |
| Serialize (large) | 0.08 ms | 2.96 ms |
| Round-trip | 0.08 ms | 2.98 ms |

## Custom AST

The **CustomNode** AST gives you full control over YAML structure:

- Inspect and modify nodes programmatically
- Add custom metadata (comments, anchors, tags)
- Build YAML from scratch with full formatting control
- Advanced use cases: template engines, config generators, code formatters

## PyYAML Compatibility

Drop-in replacement with familiar API:

```python
import pyyaml_rs as yaml  # Use as 'yaml' for easy migration

yaml.safe_load(yaml_text)
yaml.safe_dump(data)
yaml.safe_loads(yaml_text)
yaml.safe_dumps(data)
```

## Additional Features

- **Markdown frontmatter extraction** — `read_markdown()` for blog/content tools
- **JSON ↔ YAML conversion** — `from_json()` / `from_dict()`
- **Multi-document parsing** — `parse_all_docs()`
- **i18n error messages** — `set_language("zh-CN")` for bilingual errors
- **Type hints** — Full `.pyi` stubs for IDE support

## Supported YAML Constructs

| Feature | Support |
|---------|---------|
| YAML 1.2 spec | ✅ Full |
| Comments (standalone) | ✅ Preserved |
| Comments (inline) | ✅ Preserved |
| Anchors & aliases | ✅ Preserved |
| Tags (explicit) | ✅ Preserved |
| Block scalars (`\|`, `>`) | ✅ Preserved |
| Chomping indicators | ✅ Preserved |
| Flow collections (`{}`, `[]`) | ✅ Preserved |
| Merge keys (`<<`) | ✅ Resolved |
| Complex keys | ✅ Supported |
| Escape sequences | ✅ Supported |
| Multi-document | ✅ Supported |
