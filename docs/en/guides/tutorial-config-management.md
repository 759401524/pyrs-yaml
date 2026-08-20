---
title: Configuration Management with pyrs-yaml
description: An end-to-end tutorial showing how to parse, edit, validate, and re-serialize a YAML configuration file with full metadata preservation.
tags:
  - docs
  - tutorial
status: new
---

## Configuration Management with pyrs-yaml

This tutorial walks through a realistic scenario: managing a YAML
configuration file for a microservice application. You'll learn how to
parse, inspect, edit, validate, and write back a YAML file — all while
preserving every comment, anchor, tag, and formatting choice.

## Setup

```bash title="Install from PyPI"
pip install pyrs-yaml
```

## 1. The Configuration File

We start with a YAML configuration file that has comments, anchors, merge
keys, and a mix of block and flow formatting:

```yaml title="config.yaml"
# Application configuration (v2.0)
app:
  name: my-service
  version: 2.0

# Default database settings
database: &default-db
  host: localhost
  port: 5432
  name: mydb

# Environment-specific overrides
environments:
  staging:
    <<: *default-db
    host: staging.example.com
    debug: true

  production:
    <<: *default-db
    host: prod.example.com
    port: 5432
    debug: false

# Feature flags
features:
  - name: login
    enabled: true
  - name: export
    enabled: true
  - name: reporting
    enabled: false

# Custom scalar example
threshold: 0x1F  # hex value (should be parsed as int)
```

## 2. Parse the File

```python title="Parse the file"
import pyrs_yaml

doc = pyrs_yaml.parse_file("config.yaml")
print(f"Parsed: {doc.get('app.name')} v{doc.get('app.version')}")
# Parsed: my-service v2.0
```

**Key point**: All comments, anchors, tags, and formatting are preserved
in memory. The document is a `YamlDocument` object, not a raw Python dict.

## 3. Inspect Values

Use the path API (JSONPath-like) or the Node API (tree-based):

```python title="Inspect values"
# Path API — simple and direct
db_host = doc.get("database.host")
print(f"Database host: {db_host}")

# Node API — access metadata and formatting
db_node = doc.node().find("$.database")
print(f"Database is flow style: {db_node.flow_style}")  # False (block)
print(f"Database anchor: {db_node.anchor}")  # "default-db"
```

## 4. Edit Values with Metadata Preservation

When you edit a value, its comment, anchor, tag, and quoting style are
preserved. The edit is performed in-place on the AST — no string
manipulation:

```python title="Edit values in place"
# Change the production port
doc.set("$.environments.production.port", 5444)

# Change the app name while keeping its comment
doc.set("$.app.name", "my-service-v2")

# Add a comment to document a change
prod_node = doc.node().find("$.environments.production")
prod_node.set_comment("overridden for v2 rollout")
```

## 5. Manipulate Metadata

pyrs-yaml goes beyond value editing — you can read and write the YAML
metadata itself:

```python title="Read and write metadata"
# Read existing metadata
debug_node = doc.node().find("$.environments.staging.debug")
print(f"Debug comment: {debug_node.comment}")  # None

# Add a tag to document a custom type
import_node = doc.node().find("$.threshold")
import_node.set_tag("!!int")
print(f"Threshold tag: {import_node.tag}")  # "!!int"

# Add an anchor for later reference
prod_db = doc.node().find("$.environments.production")
prod_db.set_anchor("prod-db")
```

## 6. Control Formatting

Switch scalar quoting, block/flow layout, and chomping indicators:

```python title="Control formatting"
# Switch the threshold to single-quoted for clarity
doc.node().find("$.threshold").set_scalar_style("single_quoted")

# Switch the staging environment to compact flow style
staging = doc.node().find("$.environments.staging")
staging.set_flow_style(True)
```

## 7. Batch Edit with Wildcards

Use `set_many` to apply changes to every matching path — useful for
toggle-like operations:

```python title="Batch edit with wildcards"
# Disable ALL debug flags across every environment
doc.set_many(
    {
        "$.environments[*].debug": False,
    }
)

# Disable all features at once
doc.set_many(
    {
        "$.features[*].enabled": False,
    }
)
```

## 8. Sort Keys

For readability, sort the top-level keys and environment keys:

```python title="Sort keys"
doc.sort_keys()  # sort the root mapping
doc.sort_keys("$.environments")  # sort the environments
```

## 9. Validate Against a Schema

Define a schema with structural rules and validate the configuration:

```python title="Validate against a schema"
schema = """\
name: app-config
extends: core
validate:
  - path: $.app.name
    type: str
    required: true
  - path: $.environments.*.debug
    type: bool
  - path: $.threshold
    type: int
"""

# Validate — raises YamlValidateError on failure
pyrs_yaml.validate_against_schema(doc.to_yaml(), schema)
print("Configuration is valid!")
```

## 10. Deep-Copy a Subtree

Copy a subtree as a standalone Python value (detached from the document):

```python title="Deep-copy a subtree"
# Copy the staging configuration for reuse
staging_config = doc.node().find("$.environments.staging").copy()
print(staging_config)  # {'host': 'staging.example.com', 'debug': False, ...}
```

## 11. Move a Subtree

Relocate a subtree within the same document:

```python title="Move a subtree"
# Move the reporting feature to a new section
doc.node().find("$.features[2]").move("$.deprecated-features")
```

## 12. Write Back to File

Finally, serialize the edited document back to YAML:

```python title="Write back to file"
output = doc.to_yaml()
with open("config-updated.yaml", "w", encoding="utf-8") as f:
    f.write(output)
```

The output preserves **everything** — comments, anchors, merge keys,
formatting, and all the edits we made:

```yaml title="config-updated.yaml"
# Application configuration (v2.0)
app:
  name: my-service-v2
  version: 2.0

# Default database settings
database: &default-db
  host: localhost
  port: 5432
  name: mydb

# Environment-specific overrides
environments:
  # overridden for v2 rollout
  production: &prod-db
    <<: *default-db
    host: prod.example.com
    port: 5444
    debug: false

  staging:
    <<: *default-db
    debug: false
    host: staging.example.com
```

## Summary

In this tutorial you:

- :material-file-code: **Parsed** a YAML file with full metadata preservation
- :material-magnify: **Inspected** values using the path API and Node API
- :material-pencil: **Edited** values, comments, anchors, tags, and formatting
- :material-format-list-bulleted: **Batch-edited** with wildcards using `set_many`
- :material-sort: **Sorted** keys for readability
- :material-check-decagram: **Validated** against a schema
- :material-content-copy: **Copied** and **moved** subtrees
- :material-sync: **Serialized** back to YAML with everything preserved

### Next Steps

- :material-rocket-launch: [Quick Start](../quick-start.md) — Get started in minutes
- :material-pencil: [In-Place Editing Guide](../guides/editing.md) — Full editing API reference
- :material-check-decagram: [Custom Schema Guide](../guides/custom-schema.md) — Define your own schemas
- :material-book-open-page-variant: [API Reference](../api/reference.md) — Complete API documentation
