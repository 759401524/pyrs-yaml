"""Gate: no PyO3-facing path may reach for the infallible YAML writer.

`pyrs_yaml_core::serializer::to_yaml` answers with `.expect("serialization failed")`, which is fine
inside a Rust test and fatal at the Python boundary: measured on 2026-11-06, `from_toml` of a
1000-level dotted-key chain - a document its own reader accepts - surfaced
`pyo3_runtime.PanicException: serialization failed: MaxDepthExceeded(DepthError(1000))`. A panic is
not an `Exception`, so it walks straight through every `except Exception` in the language. The four
conversions (`from_json`, `from_jsonc`, `from_json5`, `from_toml`) now share `yaml_of_node`, which
maps the writer's refusal onto the typed exceptions the binding already owns.

This test is what keeps a fifth call site from appearing. It reads the binding's own sources rather
than a list beside them, so a new conversion is covered the day it lands; each file is cut at its
first `#[cfg(test)]`, because a Rust test may legitimately use the infallible form; and the scan
refuses when it finds nothing to look at, so a moved directory is a refusal rather than a pass.
"""

from __future__ import annotations

import pathlib

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
BINDING_SRC = REPO_ROOT / "crates" / "pyrs-yaml" / "src"
FORBIDDEN = "serializer::to_yaml("
HELPER = "yaml_of_node("


def _production_text(path: pathlib.Path) -> str:
    """The file's text up to its test module, if it has one."""
    text = path.read_text(encoding="utf-8")
    cut = text.find("#[cfg(test)]")
    return text if cut < 0 else text[:cut]


def test_the_binding_sources_are_where_this_gate_thinks_they_are():
    files = sorted(BINDING_SRC.rglob("*.rs"))
    assert len(files) >= 10, f"{BINDING_SRC} holds {len(files)} Rust file(s): this gate measures nothing"


def test_no_production_binding_path_calls_the_infallible_writer():
    offenders = []
    for path in sorted(BINDING_SRC.rglob("*.rs")):
        body = _production_text(path)
        for number, line in enumerate(body.split("\n"), start=1):
            if FORBIDDEN in line:
                offenders.append(f"{path.relative_to(REPO_ROOT)}:{number}: {line.strip()[:80]}")
    assert not offenders, (
        "a binding call site uses `serializer::to_yaml`, whose `.expect()` becomes a Rust panic in "
        "Python - route it through `yaml_of_node` so the writer's refusal is typed:\n  " + "\n  ".join(offenders)
    )


def test_the_fallible_helper_is_the_one_in_use():
    """A positive anchor: the rule above could also pass because the helper went unused."""
    uses = sum(_production_text(p).count(HELPER) for p in sorted(BINDING_SRC.rglob("*.rs")))
    # Five is the helper's own `fn` line plus the four conversions that call it, so a fifth site
    # drifting back to the infallible writer is a red run rather than a lower count.
    assert uses >= 5, f"expected the helper plus four conversions, found {uses} uses of {HELPER}"
