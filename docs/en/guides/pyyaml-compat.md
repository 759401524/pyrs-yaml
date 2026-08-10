---
title: PyYAML Compatibility
description: Using pyrs-yaml as a drop-in replacement for PyYAML, including migration checklist and key differences.
tags:
  - docs
status: new
---

## PyYAML Compatibility

!!! tip "Migration"
    pyrs-yaml is a drop-in replacement for PyYAML — replace `import yaml` with
    `import pyrs_yaml as yaml` and most code works unchanged, with significantly
    better performance and round-trip preservation.

pyrs-yaml provides a **drop-in replacement** for PyYAML, making migration straightforward.

### Simple Migration

```python
# Before
import yaml

data = yaml.safe_load(yaml_text)
yaml_str = yaml.safe_dump(data)

# After
import pyrs_yaml as yaml

data = yaml.safe_load(yaml_text)
yaml_str = yaml.safe_dump(data)
```

### Compatible API

| PyYAML Function | pyrs-yaml Equivalent | Notes |
|-----------------|---------------------|-------|
| `yaml.safe_load()` | `pyrs_yaml.safe_load()` | ✅ Identical |
| `yaml.safe_loads()` | `pyrs_yaml.safe_loads()` | ✅ Identical |
| `yaml.safe_dump()` | `pyrs_yaml.safe_dump()` | ✅ Identical |
| `yaml.safe_dumps()` | `pyrs_yaml.safe_dumps()` | ✅ Identical |
| `yaml.load()` | `pyrs_yaml.safe_load()` | ⚠️ Use safe variant |
| `yaml.dump()` | `pyrs_yaml.safe_dump()` | ⚠️ Use safe variant |

### Key Differences

#### What pyrs-yaml Does Better

| Feature | PyYAML | pyrs-yaml |
|---------|--------|-----------|
| Round-trip preservation | ❌ Loses comments/anchors | ✅ Preserves everything |
| Performance | Baseline | **25–40× faster** |
| Type hints | Partial | ✅ Full `.pyi` stubs |
| ABI3 wheel | N/A | ✅ Single wheel for all Python versions |
| i18n errors | ❌ English only | ✅ English + Chinese |

#### What to Watch For

1. **Anchor/alias handling**: PyYAML loses anchors on round-trip; pyrs-yaml preserves them
2. **Comment position**: pyrs-yaml may reorder some comments in complex nested structures
3. **Flow style**: Both preserve, but output formatting may differ slightly
4. **Error messages**: pyrs-yaml uses i18n error messages with more context

### Migration Checklist

- [ ] Replace `import yaml` with `import pyrs_yaml as yaml`
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
import pyrs_yaml


def load_config(path):
    return pyrs_yaml.parse_file(path).to_dict()


def save_config(data, path):
    doc = pyrs_yaml.parse(pyrs_yaml.safe_dump(data))
    pyrs_yaml.dump_file(data, path)
```
