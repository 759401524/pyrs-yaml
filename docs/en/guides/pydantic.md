---
title: Pydantic Integration
description: Parse YAML into Pydantic v2 models, serialize models to YAML, and load BaseSettings via pydantic-settings with pyrs-yaml as the parser.
tags:
  - docs
status: new
---

## Pydantic Integration

pyrs-yaml integrates with [Pydantic](https://docs.pydantic.dev/) v2 and
[pydantic-settings](https://docs.pydantic.dev/latest/concepts/pydantic_settings/) to
turn YAML into validated models and back. Both are optional dependencies:

- `pip install pydantic` (or `pip install 'pyrs-yaml[pydantic]'`) for model
  parsing and serialization
- `pip install 'pyrs-yaml[settings]'` for `BaseSettings` loading (pulls in
  `pydantic-settings`)

### Parsing YAML into a Model

`parse_as()` parses YAML and validates it against a Pydantic `BaseModel`
subclass, returning a model instance. Any `**yaml_kwargs` are forwarded to the
`YAML()` constructor (for example `resolve_merges`).

```python title="Parse YAML into a Pydantic model"
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


user = pyrs_yaml.parse_as(User, "name: Alice\nage: 30")
print(user.name)  # Alice
print(user.age)  # 30
```

`parse_as()` raises:

- `ImportError` — pydantic is not installed
- `TypeError` — `model` is not a `BaseModel` subclass
- `pydantic.ValidationError` — the parsed data fails model validation

### Serializing a Model to YAML

`dump_pydantic()` serializes a Pydantic model to a YAML string. It calls
`model_dump(mode="json")` first so that string-typed fields stay strings — a
zip code like `"10001"` is not coerced to an integer — then delegates to
`safe_dump`.

```python title="Serialize a Pydantic model to YAML"
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


yaml_str = pyrs_yaml.dump_pydantic(User(name="Alice", age=30))
print(yaml_str)
# name: Alice
# age: 30
```

`dump_pydantic()` raises:

- `ImportError` — pydantic is not installed
- `TypeError` — `model` is not a `BaseModel` instance

### Loading Settings with pydantic-settings

`PyrsYamlConfigSettingsSource` is a drop-in replacement for
`pydantic_settings.YamlConfigSettingsSource`. It reads YAML config file(s) with
pyrs-yaml's YAML 1.2 parser, then feeds the values into a `BaseSettings` model
alongside env vars, dotenv, and secrets — with the same priority and behavior.

```python title="Load BaseSettings from a YAML file"
from pydantic_settings import BaseSettings, SettingsConfigDict
import pyrs_yaml


class Settings(BaseSettings):
    app_name: str

    model_config = SettingsConfigDict(yaml_file="config.yaml")

    @classmethod
    def settings_customise_sources(
        cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
    ):
        return (
            init_settings,
            env_settings,
            dotenv_settings,
            file_secret_settings,
            pyrs_yaml.PyrsYamlConfigSettingsSource(settings_cls),
        )
```

The source supports the same options as `YamlConfigSettingsSource`:

- `yaml_file` — path or list of paths (declared via `SettingsConfigDict` or passed in)
- `yaml_file_encoding` — file encoding
- `yaml_config_section` — dot-notation path to a nested section
- `deep_merge` — merge multiple files deeply instead of replacing

!!! note "Lazy import"
    `import pyrs_yaml` never requires pydantic or pydantic-settings. Accessing
    `pyrs_yaml.parse_as`, `pyrs_yaml.dump_pydantic`, or
    `pyrs_yaml.PyrsYamlConfigSettingsSource` without the corresponding dependency
    installed raises `ImportError` with installation hints.

### Round-Trip with Comments

Because `parse_as()` is built on `safe_load`, comment and anchor preservation
is not part of the model path — use `parse()` with a `YamlDocument` for
round-trip editing, and `parse_as()` only when you need a validated model.

!!! tip "Choose the right parse path"
    Use `parse_as()` for config validation, and `parse()` when comments,
    anchors, or formatting must survive a round trip.

### See Also

- [Parsing YAML](parsing.md) — Parse strings, files, and multiple documents
- [Serialization](serialization.md) — Convert YAML documents to and from Python objects
- [Configuration Management](tutorial-config-management.md) — End-to-end walkthrough
- [API Reference](../api/reference.md) — Full signatures for `parse_as`, `dump_pydantic`, and `PyrsYamlConfigSettingsSource`
