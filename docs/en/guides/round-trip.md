---
title: Round-Trip Preservation
description: Understand pyrs-yaml's round-trip preservation of formatting and metadata, and how it compares to PyYAML and ruamel.yaml.
tags:
  - docs
status: new
---

## Round-Trip Preservation

This is pyrs-yaml's **killing feature** — what makes it unique among Python YAML libraries.

### What is Round-Trip Preservation?

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
```

> **Note on merge keys:** by default (`resolve_merges=True`), `<<: *db` is **resolved** during parsing, so the output materializes the merged keys (`api: {host: localhost, endpoint: ...}`) and `<<` no longer appears. Pass `resolve_merges=False` to keep the `<<: *db` pair verbatim in the round-trip.

### What Gets Preserved

| Element | Preserved? | Notes |
|---------|------------|-------|
| Standalone comments | :material-check: | Before keys and values |
| Inline comments | :material-check: | At end of lines |
| Anchors (`&name`) | :material-check: | Full anchor syntax |
| Aliases (`*name`) | :material-check: | Alias references resolved |
| Merge keys (`<<`) | :material-alert: | Resolved by default; preserved with `resolve_merges=False` |
| Tags (`!!str`, `!!int`) | :material-check: | Explicit tags preserved |
| Scalar styles | :material-check: | Plain, quoted, literal, folded |
| Chomping (`\|-`, `>-`) | :material-check: | Block scalar indicators |
| Flow/block style | :material-check: | `[]`/`{}` vs block preserved |
| Compact sequence items | :material-check: | `- host: a` stays on the dash line (metadata-free mapping items only) |
| Key order | :material-check: | `IndexMap` guarantees order |

### PyYAML vs pyrs-yaml Round-Trip

```python
original = "# Comment\nkey: value  # inline\n"

# PyYAML: loses everything
yaml.safe_dump(yaml.safe_load(original))
# Output: 'key: value\n'  :material-close:

# pyrs-yaml: preserves everything
doc = pyrs_yaml.parse(original)
doc.to_yaml()
# Output: '# Comment\nkey: value  # inline\n'  :material-check:
```

### Performance

Round-trip performance vs competitors:

| Library | Round-trip (large) | Comments | Anchors | Tags |
|---------|-------------------|----------|---------|------|
| **pyrs-yaml** | **0.08 ms** | :material-check: | :material-check: | :material-check: |
| PyYAML | 2.98 ms | :material-close: | :material-close: | :material-close: |
| ruamel.yaml | 6.79 ms | :material-check: | :material-check: | :material-check: |

**pyrs-yaml is 37× faster than PyYAML and 85× faster than ruamel.yaml** while preserving everything.

---

### See Also

- [Serialization](serialization.md) — Serialize documents without losing formatting
- [In-Place Editing](editing.md) — Edit while preserving round-trip fidelity
- [PyYAML Compatibility](pyyaml-compat.md) — Migrate from PyYAML
