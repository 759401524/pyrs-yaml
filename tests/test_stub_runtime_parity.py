"""The stub inside the wheel must describe the object the wheel ships, parameter name by parameter name.

Measured on the way to this file: `pyrs_yaml.read_markdown(path=...)` raised `TypeError` while the committed
`.pyi` declared the parameter `path`, because `python/pyrs_yaml/__init__.py` re-wraps the native function under a
wrapper that renamed the parameter to `content`. The stub is generated from the **extension**, so every wrapper
that shifts a name silently moves the typed contract away from the callable: mypy and pyright then approve a call
the runtime refuses, and reject the one that works. `validate_against_schema` had the same drift in the other
direction (`schema_yaml` in the stub, `schema` on the object).

This is the systemic pin. The two names are read from the two artefacts and compared for every module-level
function the stub declares; the count is asserted, so a future wrapper that renames something fails here rather
than in a user's editor.
"""

from __future__ import annotations

import ast
import inspect
import pathlib

import pytest

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
STUB = REPO_ROOT / "python" / "pyrs_yaml" / "pyrs_yaml.pyi"


def declared_parameters(node: ast.FunctionDef | ast.AsyncFunctionDef) -> list[str]:
    """The parameter names the stub writes, `self` excluded, `*args` / `**kwargs` marked."""
    args = node.args
    names = [a.arg for a in [*args.posonlyargs, *args.args] if a.arg != "self"]
    if args.vararg:
        names.append(f"*{args.vararg.arg}")
    names += [a.arg for a in args.kwonlyargs]
    if args.kwarg:
        names.append(f"**{args.kwarg.arg}")
    return names


@pytest.fixture(scope="module")
def stub_functions() -> dict[str, list[str]]:
    tree = ast.parse(STUB.read_text(encoding="utf-8"))
    return {
        node.name: declared_parameters(node)
        for node in tree.body
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
    }


def test_the_stub_covers_every_module_function_the_package_exports(stub_functions):
    """An empty inventory would make the parity assertion below vacuous, so it is measured separately."""
    assert len(stub_functions) >= 40, len(stub_functions)


def test_stub_parameter_names_are_the_names_the_object_accepts(stub_functions):
    import pyrs_yaml

    drift = []
    for name, declared in sorted(stub_functions.items()):
        function = getattr(pyrs_yaml, name, None)
        if function is None:
            drift.append((name, declared, "<not exported>"))
            continue
        try:
            actual = [p.name for p in inspect.signature(function).parameters.values()]
        except (TypeError, ValueError):  # pragma: no cover - every shipped function answers today
            pytest.fail(f"{name} stopped reporting a signature; the parity rule would silently skip it")
        if actual != declared:
            drift.append((name, declared, actual))

    assert not drift, "the stub describes a callable the package does not offer:\n" + "\n".join(
        f"  {name}: stub={stub} runtime={runtime}" for name, stub, runtime in drift
    )
