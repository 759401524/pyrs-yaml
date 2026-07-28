# Round-Trip Preservation

This is pyrs-yaml's **killing feature** — what makes it unique among Python YAML libraries.

## What is Round-Trip Preservation?

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

doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# All formatting and metadata preserved
assert "# Server configuration" in output
assert "# main port" in output
assert "&db" in output
assert "<<: *db" in output
```

## What Gets Preserved

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

## PyYAML vs pyrs-yaml Round-Trip

```python
original = "# Comment\nkey: value  # inline\n"

# PyYAML: loses everything
yaml.safe_dump(yaml.safe_load(original))
# Output: 'key: value\n'  ❌

# pyrs-yaml: preserves everything
doc = pyrs_yaml.parse(original)
doc.to_yaml()
# Output: '# Comment\nkey: value  # inline\n'  ✅
```

## Performance

Round-trip performance vs competitors:

| Library | Round-trip (large) | Comments | Anchors | Tags |
|---------|-------------------|----------|---------|------|
| **pyrs-yaml** | **0.08 ms** | ✅ | ✅ | ✅ |
| PyYAML | 2.98 ms | ❌ | ❌ | ❌ |
| ruamel.yaml | 6.79 ms | ✅ | ✅ | ✅ |

**pyrs-yaml is 37× faster than PyYAML and 85× faster than ruamel.yaml** while preserving everything.
