"""CLI ↔ Binding parity gate.

Pillar 1 of the objective requires the CLI and the Python Binding to carry
equivalent functionality -- nothing may exist in Core alone. Until now this
invariant was only documented in CHANGELOG ("CLI format parity" +
"load_json completes the load_* family parity"), not enforced.

This module turns the two surfaces into an executable contract:

- Every CLI `to-X` / `from-X` / `load_X` transformation has a callable
  binding-level counterpart (top-level function or `YamlDocument` method)
  for the same X.
- Every `load_*` sibling in the family is present (json / jsonc / json5 / toml)
  so the strict-vs-loose loader matrix is complete.
- Editing verbs (`fmt`, `get`, `set`, `delete`, `rename`, `sort-keys`, `move`,
  `frontmatter`) route through the `Node` / `YamlDocument` API.
- `validate` and `compliance` verbs map to `validate_against_schema` and
  `compliance_report`.

Failures here mean the parity claim in `CHANGELOG.md` would be false, so the
release process can no longer silently drift the two surfaces apart.
"""

from __future__ import annotations

import importlib

import pytest

import pyrs_yaml

# ── CLI command inventory ─────────────────────────────────────────────────
# Names of every registered CLI command (see python/pyrs_yaml/cli/app.py).
# Kept literal so the test catches a rename/removal on either side rather
# than silently deriving a smaller set when the CLI module fails to import.
CLI_COMMANDS: frozenset[str] = frozenset(
    {
        "fmt",
        "get",
        "set",
        "delete",
        "rename",
        "sort-keys",
        "move",
        "frontmatter",
        "validate",
        "to-json",
        "from-json",
        "to-toml",
        "from-toml",
        "to-jsonc",
        "from-jsonc",
        "to-json5",
        "from-json5",
        "compliance",
    }
)


@pytest.fixture(scope="module")
def cli_app():
    """Load the CLI module or skip if cyclopts (Python 3.10+ extra) is absent."""
    try:
        mod = importlib.import_module("pyrs_yaml.cli")
    except ImportError as exc:  # pragma: no cover - depends on env
        pytest.skip(f"CLI extra unavailable: {exc}")
    app = getattr(mod, "app", None) or getattr(mod, "APP", None) or getattr(mod, "_app", None)
    if app is None:
        # Fall back to the submodule directly if the package does not re-export.
        from pyrs_yaml.cli import app as app_mod  # type: ignore[assignment]

        app = getattr(app_mod, "app", None)
    if app is None:  # pragma: no cover
        pytest.skip("Could not locate the cyclopts App instance")
    return app


def test_cli_command_surface_matches_inventory(cli_app):
    """The registered CLI command set is exactly the inventory above."""
    # cyclopts exposes registered commands via `App._commands` (name → Command).
    commands = getattr(cli_app, "_commands", None)
    if commands is None:  # pragma: no cover - cyclopts private API drift
        pytest.skip("cyclopts internal _commands layout changed; update test adapter")
    # cyclopts registers `--help` / `-h` / `--version` as pseudo-commands on the
    # root App; they are not part of the pyrs-yaml surface — filter them out.
    actual = {name for name in commands if not name.startswith("-")}
    assert actual == set(CLI_COMMANDS), (
        f"CLI surface drift: missing={CLI_COMMANDS - actual}, extra={actual - CLI_COMMANDS}"
    )


# ── Format conversion parity ────────────────────────────────────────────────
# Each CLI `to-X` / `from-X` must have a matching binding surface. `to-X`
# routes through `YamlDocument.to_X()`; `from-X` through the top-level
# `from_X` string-to-string converter; and (where the loader family applies)
# a direct-to-Python `load_X`.

_FORMAT_CONVERSIONS = [
    # (cli-name, doc-method, top-level-from-X, top-level-load-X or None)
    ("to-json", "to_json", None, None),
    ("from-json", None, "from_json", "load_json"),
    ("to-toml", "to_toml", None, None),
    ("from-toml", None, "from_toml", "load_toml"),
    ("to-jsonc", "to_jsonc", None, None),
    ("from-jsonc", None, "from_jsonc", "load_jsonc"),
    ("to-json5", "to_json5", None, None),
    ("from-json5", None, "from_json5", "load_json5"),
]


@pytest.mark.parametrize("cli_name,doc_method,from_name,load_name", _FORMAT_CONVERSIONS)
def test_cli_format_verb_has_binding_counterpart(cli_name, doc_method, from_name, load_name):
    """Every CLI format verb has its documented binding counterpart."""
    if doc_method is not None:
        assert hasattr(pyrs_yaml.YamlDocument, doc_method), f"`YamlDocument.{doc_method}` missing (CLI: {cli_name})"
    if from_name is not None:
        assert hasattr(pyrs_yaml, from_name), f"`pyrs_yaml.{from_name}` missing (CLI: {cli_name})"
        assert from_name in pyrs_yaml.__all__, f"`{from_name}` not re-exported in __all__"
    if load_name is not None:
        assert hasattr(pyrs_yaml, load_name), f"`pyrs_yaml.{load_name}` missing (CLI: {cli_name})"
        assert load_name in pyrs_yaml.__all__, f"`{load_name}` not re-exported in __all__"


def test_load_family_symmetry():
    """All four load_* siblings coexist — no partial coverage in the family."""
    for name in ("load_json", "load_jsonc", "load_json5", "load_toml"):
        assert hasattr(pyrs_yaml, name), f"{name} missing from module"
        assert name in pyrs_yaml.__all__, f"{name} missing from __all__"


# ── Editing / validate / compliance verbs ──────────────────────────────────


@pytest.mark.parametrize(
    ("cli_name", "binding_symbol"),
    [
        ("fmt", "safe_dump"),
        ("get", "Node"),
        ("set", "Node"),
        ("delete", "Node"),
        ("rename", "Node"),
        ("sort-keys", "Node"),
        ("move", "Node"),
        ("frontmatter", "read_markdown"),
        ("validate", "validate_against_schema"),
        ("compliance", "compliance_report"),
    ],
)
def test_cli_editing_verb_has_binding_counterpart(cli_name, binding_symbol):
    """Editing / validate / compliance CLI verbs route through live Python API."""
    assert hasattr(pyrs_yaml, binding_symbol), (
        f"CLI `{cli_name}` expects `pyrs_yaml.{binding_symbol}` on the binding side, but the symbol is missing"
    )


# ── Round-trip: format conversion works via binding alone ────────────────────


def test_binding_round_trip_covers_every_cli_format_pair():
    """Every format the CLI converts to/from is reachable through the binding
    without subprocess invocation — the substantive parity claim, executed."""
    src_yaml = "a: 1\nb: [1, 2, 3]\n"
    doc = pyrs_yaml.parse(src_yaml)
    # to-X emits text; from-X re-parses text back into YAML on the hub side.
    for meth in ("to_json", "to_jsonc", "to_json5", "to_toml"):
        text = getattr(doc, meth)()
        assert isinstance(text, str) and text
    # from-X returns YAML text that parse() re-accepts byte-for-byte.
    json_text = doc.to_json()
    assert pyrs_yaml.parse(pyrs_yaml.from_json(json_text)).to_json() == json_text
    toml_text = doc.to_toml()
    assert pyrs_yaml.parse(pyrs_yaml.from_toml(toml_text)).to_toml() == toml_text
    # load_* siblings produce the same Python values as `from_X -> safe_load`.
    assert pyrs_yaml.load_json(json_text) == pyrs_yaml.safe_load(pyrs_yaml.from_json(json_text))
