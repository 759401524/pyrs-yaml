"""The stub inside the wheel must describe the object the wheel ships, parameter name by parameter name.

Measured on the way to this file: `pyrs_yaml.read_markdown(path=...)` raised `TypeError` while the committed
`.pyi` declared the parameter `path`, because `python/pyrs_yaml/__init__.py` re-wraps the native function under a
wrapper that renamed the parameter to `content`. The stub is generated from the **extension**, so every wrapper
that shifts a name silently moves the typed contract away from the callable: mypy and pyright then approve a call
the runtime refuses, and reject the one that works.

This is the systemic pin. The two names are read from the two artefacts and compared for every module-level
function the stub declares; the count is asserted, so a future wrapper that renames something fails here rather
than in a user's editor.

The same question has a second, larger answer for *classes*, measured while closing ROADMAP row 1: the stub is
generated from the extension, so anything `python/pyrs_yaml/__init__.py` puts onto a native class after import
is invisible to the generator. `pyrs_yaml.parse()` returned `Any` (the wrapper erased the binding's
`-> YamlDocument`) and `YamlDocument` declared none of the twelve members the package attaches - `node`, `find`,
`walk`, `scalars`, `merged`, `set`, `set_many`, `insert`, `append`, `delete`, `rename`, `sort_keys` - so an
editor saw one class and the runtime offered another. Both halves are pinned here: every member a class owns is
declared, and declared members keep their parameter names.
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
    assert len(stub_functions) >= 38, len(stub_functions)


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


@pytest.fixture(scope="module")
def stub_methods() -> dict[str, list[str]]:
    """The members the stub declares, keyed `Class.member`, with `self` excluded the same way."""
    tree = ast.parse(STUB.read_text(encoding="utf-8"))
    methods = {}
    for node in tree.body:
        if not isinstance(node, ast.ClassDef):
            continue
        for member in node.body:
            if isinstance(member, (ast.FunctionDef, ast.AsyncFunctionDef)):
                methods[f"{node.name}.{member.name}"] = declared_parameters(member)
    return methods


def owned_members(cls: type) -> list[str]:
    """The public callables the class object carries itself.

    `vars()` and not `dir()`: `add_note` and `with_traceback` belong to `BaseException`, which no stub declares
    on anybody's behalf, and a rule that walks the MRO would ask the artifact to name them.
    """
    return sorted(name for name, value in vars(cls).items() if not name.startswith("_") and callable(value))


def test_the_stub_declares_every_public_member_of_a_class_it_names(stub_methods):
    """A class the artifact describes has to offer what the runtime class offers, member for member.

    This is the ROADMAP row 1 pin: `python/pyrs_yaml/__init__.py` attaches `node`, `merged`, `set` and the rest
    onto the extension's class after the generator has run, and until the route declared them no checker could
    see them - `document.node()` was an attribute of a class the wheel's own typing contract never wrote.
    """
    import pyrs_yaml

    classes = sorted({key.split(".")[0] for key in stub_methods})
    assert len(classes) >= 4, classes

    missing = []
    for cls_name in classes:
        cls = getattr(pyrs_yaml, cls_name, None)
        if not isinstance(cls, type):
            missing.append(f"{cls_name} (the stub describes a class the package does not export)")
            continue
        missing.extend(
            f"{cls_name}.{member}" for member in owned_members(cls) if f"{cls_name}.{member}" not in stub_methods
        )
    assert not missing, "the runtime offers a member the shipped stub does not declare:\n" + "\n".join(
        f"  {name}" for name in missing
    )


def test_no_public_wrapper_hides_the_type_the_binding_returns():
    """`parse() -> Any` was the first domino: with the document typed away, no member question is answerable.

    Measured while closing ROADMAP row 1 - mypy reported `Any` for `pyrs_yaml.parse("a: 1")` even though the
    binding and its stub both say `YamlDocument`, because the package re-exports the function through a wrapper
    that declared no return type. The `__getattr__` shims are the deliberate exception: a dispatcher answers
    attributes nobody enumerated, and a stub cannot name what the runtime chooses at call time.
    """
    tree = ast.parse((REPO_ROOT / "python" / "pyrs_yaml" / "__init__.py").read_text(encoding="utf-8"))
    # An `@overload` pair's implementation is not what a checker reads - the declarations above it are - so the
    # untyped `def register_tag(name, handler=None, priority=0)` body is not an erased return.
    overloaded = {
        node.name
        for node in tree.body
        if isinstance(node, ast.FunctionDef)
        and any(isinstance(d, ast.Name) and d.id == "overload" for d in node.decorator_list)
    }
    erased = [
        node.name
        for node in tree.body
        if isinstance(node, ast.FunctionDef)
        and not node.name.startswith("_")
        and node.name not in overloaded
        and (node.returns is None or (isinstance(node.returns, ast.Name) and node.returns.id == "Any"))
    ]
    assert not erased, "a public wrapper states no type where the binding does: " + ", ".join(erased)


def test_stub_member_parameters_are_the_names_the_class_accepts(stub_methods):
    """The attached members are compared parameter by parameter, exactly as the module functions are."""
    import pyrs_yaml

    drift = []
    for key, declared in sorted(stub_methods.items()):
        cls_name, _, member = key.partition(".")
        cls = getattr(pyrs_yaml, cls_name, None)
        if not isinstance(cls, type) or member not in vars(cls):
            continue  # inherited members are the base's contract, and this file does not speak for it
        function = getattr(cls, member)
        try:
            actual = [p.name for p in inspect.signature(function).parameters.values() if p.name != "self"]
        except (TypeError, ValueError):
            pytest.fail(f"{key} stopped reporting a signature; the parity rule would silently skip it")
        if actual != declared:
            drift.append((key, declared, actual))

    assert not drift, "the stub describes a member the class does not accept:\n" + "\n".join(
        f"  {key}: stub={stub} runtime={runtime}" for key, stub, runtime in drift
    )


def annotation_text(source: str, node) -> str:
    """The annotation exactly as the file spells it, with a forward-reference's quotes removed.

    `ast.get_source_segment` rather than `ast.unparse`: the Python 3.8 legs run this file and `unparse` is
    3.9+, so the rendering the repository can ask for on the floor interpreter is the source text itself.
    """
    if node is None:
        return ""
    return (ast.get_source_segment(source, node) or "").strip().strip("'\"")


def test_the_three_parse_entries_state_the_document_the_binding_returns():
    """The named claim, spelled out: `parse`, `parse_file` and `parse_all_docs` type the document twice over.

    A wrapper that says `list[Any]` where the binding says `list[YamlDocument]` is invisible to the general
    predicate above - `Any` inside a container is honest for a mapping of anything - so the three entry points
    users reach first are pinned by name, in the wrapper and in the artifact the wheel ships.
    """
    stub_source = STUB.read_text(encoding="utf-8")
    declared = {
        node.name: annotation_text(stub_source, node.returns)
        for node in ast.parse(stub_source).body
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
    }
    package_source = (REPO_ROOT / "python" / "pyrs_yaml" / "__init__.py").read_text(encoding="utf-8")
    package = ast.parse(package_source)
    for name, expected in (
        ("parse", "YamlDocument"),
        ("parse_file", "YamlDocument"),
        ("parse_all_docs", "list[YamlDocument]"),
    ):
        assert declared[name] == expected, (name, declared[name])
        stated = next(
            annotation_text(package_source, node.returns)
            for node in package.body
            if isinstance(node, ast.FunctionDef) and node.name == name
        )
        assert stated == expected, (name, stated)
