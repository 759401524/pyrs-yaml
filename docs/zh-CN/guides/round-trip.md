# ---

---

title: Round-Trip Preservation
lang: zh-CN

## 往返保留

This is pyyaml-rs's **killing feature** — what makes it unique among Python YAML libraries.

### What is 往返保留?

Round-trip preservation means: **parse YAML → modify → serialize back → output is identical (or semantically equivalent) to the input.**

```python
original = """
# Server configuration
server:
  host: 0.0.0.0
  port: 8080  # main port

# Database anchor
database: &db
  host: localhost
  port: 5432

api:
  <<: *db
  endpoint: /api/v1
"""

doc = pyyaml_rs.parse(original)
output = doc.to_yaml()

# All formatting and metadata preserved
assert "# Server configuration" in output
assert "# main port" in output
assert "&db" in output
assert "<<: *db" in output
```

### What Gets Preserved

| Element | Preserved? | Notes |
|---------|------------|-------|
| Standalone comments | ✅ | Before keys and values |
| Inline comments | ✅ | At end of lines |
| Anchors (`&name`) | ✅ | Full anchor syntax |
| Aliases (`*name`) | ✅ | Alias references resolved |
| Merge keys (`<<`) | ✅ | Resolved by default |
| Tags (`!!str`, `!!int`) | ✅ | Explicit tags preserved |
| Scalar styles | ✅ | Plain, quoted, literal, folded |
| Chomping (`\|-`, `>-`) | ✅ | Block scalar indicators |
| Flow/block style | ✅ | `[]`/`{}` vs block preserved |
| Key order | ✅ | `IndexMap` guarantees order |

### PyYAML vs pyyaml-rs Round-Trip

```python
original = "# Comment\nkey: value  # inline\n"

# PyYAML: loses everything
yaml.safe_dump(yaml.safe_load(original))
# Output: 'key: value\n'  ❌

# pyyaml-rs: preserves everything
doc = pyyaml_rs.parse(original)
doc.to_yaml()
# Output: '# Comment\nkey: value  # inline\n'  ✅
```

### 性能

Round-trip performance vs competitors:

| Library | Round-trip (large) | Comments | Anchors | Tags |
|---------|-------------------|----------|---------|------|
| **pyyaml-rs** | **0.08 ms** | ✅ | ✅ | ✅ |
| PyYAML | 2.98 ms | ❌ | ❌ | ❌ |
| ruamel.yaml | 6.79 ms | ✅ | ✅ | ✅ |

**pyyaml-rs is 37× faster than PyYAML and 85× faster than ruamel.yaml** while preserving everything.
