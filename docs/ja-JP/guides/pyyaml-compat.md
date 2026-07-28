# ---

---

title: PyYAML Compatibility
lang: ja-JP

## PyYAML 互換性

pyyaml-rs provides a **drop-in replacement** for PyYAML, making migration straightforward.

### Simple Migration

```python
# Before
import yaml
data = yaml.safe_load(yaml_text)
yaml_str = yaml.safe_dump(data)

# After
import pyyaml_rs as yaml
data = yaml.safe_load(yaml_text)
yaml_str = yaml.safe_dump(data)
```

### Compatible API

| PyYAML Function | pyyaml-rs Equivalent | Notes |
|-----------------|---------------------|-------|
| `yaml.safe_load()` | `pyyaml_rs.safe_load()` | ✅ Identical |
| `yaml.safe_loads()` | `pyyaml_rs.safe_loads()` | ✅ Identical |
| `yaml.safe_dump()` | `pyyaml_rs.safe_dump()` | ✅ Identical |
| `yaml.safe_dumps()` | `pyyaml_rs.safe_dumps()` | ✅ Identical |
| `yaml.load()` | `pyyaml_rs.safe_load()` | ⚠️ Use safe variant |
| `yaml.dump()` | `pyyaml_rs.safe_dump()` | ⚠️ Use safe variant |

### Key Differences

#### What pyyaml-rs Does Better

| Feature | PyYAML | pyyaml-rs |
|---------|--------|-----------|
| Round-trip preservation | ❌ Loses comments/anchors | ✅ Preserves everything |
| パフォーマンス | Baseline | **25–40× faster** |
| Type hints | Partial | ✅ Full `.pyi` stubs |
| ABI3 wheel | N/A | ✅ Single wheel for all Python versions |
| i18n errors | ❌ English only | ✅ English + Chinese |

#### What to Watch For

1. **Anchor/alias handling**: PyYAML loses anchors on round-trip; pyyaml-rs preserves them
2. **Comment position**: pyyaml-rs may reorder some comments in complex nested structures
3. **Flow style**: Both preserve, but output formatting may differ slightly
4. **Error messages**: pyyaml-rs uses i18n error messages with more context

### Migration Checklist

- [ ] Replace `import yaml` with `import pyyaml_rs as yaml`
- [ ] Test all YAML parsing/saving workflows
- [ ] Verify round-trip output matches expectations
- [ ] Check anchor/alias behavior (if used)
- [ ] Review error handling for custom error messages

### Example Migration

```python
# Old code
import yaml

def load_config(path):
    with open(path) as f:
        return yaml.safe_load(f)

def save_config(data, path):
    with open(path, "w") as f:
        yaml.safe_dump(data, f)

# New code
import pyyaml_rs

def load_config(path):
    return pyyaml_rs.parse_file(path).to_dict()

def save_config(data, path):
    doc = pyyaml_rs.parse(pyyaml_rs.safe_dump(data))
    pyyaml_rs.dump_file(data, path)
```
