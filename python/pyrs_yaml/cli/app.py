"""Cyclopts application wiring all ``pyrs-yaml`` CLI commands."""

from __future__ import annotations

import json as _json
import sys
from pathlib import Path
from typing import Annotated, Any

from cyclopts import App, Parameter

import pyrs_yaml

from ._io import emit, fail, is_stdio, load_document, read_text

app = App(
    name="pyrs-yaml",
    help="High-performance YAML toolkit with perfect round-trip support.",
    version=pyrs_yaml.__version__,
)

STDIN_FILE = "-"
"""Default input source; the bare "-" token means stdin/stdout."""


def _serialize(doc: Any, indent: int) -> str:
    from pyrs_yaml import YamlSerializeError

    try:
        if indent == 2:
            return doc.to_yaml()
        return doc.to_yaml_with_options(indent_size=indent)
    except YamlSerializeError as exc:
        fail(str(exc))


def _finish_edit(doc: Any, file: str, inplace: bool, output: str | None) -> None:
    if inplace:
        if is_stdio(file):
            fail("cannot use --inplace with stdin input; use -o/--output instead")
        Path(file).write_text(_serialize(doc, 2), encoding="utf-8")
    else:
        emit(_serialize(doc, 2), output)


@app.command(name="fmt")
def fmt(
    file: Annotated[str, Parameter(allow_leading_hyphen=True)] = STDIN_FILE,
    *,
    output: Annotated[str | None, Parameter(name=["--output", "-o"], allow_leading_hyphen=True)] = None,
    inplace: Annotated[bool, Parameter(name=["--inplace", "-i"])] = False,
    indent: Annotated[int, Parameter(name=["--indent"])] = 2,
) -> None:
    """Reformat YAML with round-trip fidelity (comments/anchors/order preserved).

    Parameters
    ----------
    file:
        YAML file to format; ``-`` or omitted reads stdin.
    """
    doc = load_document(file)
    if inplace:
        if is_stdio(file):
            fail("cannot use --inplace with stdin input; use -o/--output instead")
        Path(file).write_text(_serialize(doc, indent), encoding="utf-8")
    else:
        emit(_serialize(doc, indent), output)


@app.command
def get(
    file: Annotated[str, Parameter(allow_leading_hyphen=True)] = STDIN_FILE,
    path: str = "$",
    *,
    format: Annotated[str, Parameter(name=["--format", "-f"])] = "yaml",
) -> None:
    """Query values by JSONPath (e.g. ``$.servers[0].host``, ``$..name``).

    Parameters
    ----------
    file:
        YAML file to query; ``-`` or omitted reads stdin.
    path:
        JSONPath expression starting with ``$``.
    """
    if format not in ("yaml", "json", "text"):
        fail(f"invalid --format {format!r}; expected yaml, json or text")

    doc = load_document(file)
    try:
        result = doc.find(path)
    except ValueError as exc:
        fail(str(exc))

    nodes = result if isinstance(result, list) else [result]
    chunks: list[str] = []
    for node in nodes:
        try:
            root_type = node.root_type
        except (KeyError, IndexError, TypeError):
            continue
        if format == "json":
            chunks.append(_json.dumps(node.copy(), indent=2))
        elif format == "text" and root_type in ("scalar", "null"):
            value = node.value
            chunks.append("null" if value is None else str(value))
        else:
            chunks.append(node.to_yaml().rstrip("\n"))
    if not chunks:
        fail(f"no match for path {path!r}")
    sys.stdout.write("\n".join(chunks) + "\n")


@app.command(name="set")
def set_value(
    file: Annotated[str, Parameter(allow_leading_hyphen=True)],
    path: str,
    value: Annotated[str, Parameter(allow_leading_hyphen=True)],
    *,
    inplace: Annotated[bool, Parameter(name=["--inplace", "-i"])] = False,
    string: Annotated[bool, Parameter(name=["--string", "-s"])] = False,
    create_missing: Annotated[bool, Parameter(name=["--create-missing"])] = False,
) -> None:
    """Set the value at a JSONPath.

    VALUE is parsed as YAML unless --string is given.

    Parameters
    ----------
    file:
        YAML file to edit.
    path:
        JSONPath of the node to set (wildcards not allowed).
    value:
        New value; parsed as YAML (numbers, bools, nested structures).
    """
    from pyrs_yaml import YamlParseError, YamlPathError, safe_load

    if string:
        parsed: Any = value
    else:
        try:
            parsed = safe_load(value)
        except YamlParseError:
            parsed = value

    doc = load_document(file)
    try:
        doc.set(path, parsed, create_missing=create_missing)
    except (YamlPathError, KeyError, IndexError) as exc:
        fail(f"cannot set {path!r}: {exc}")
    _finish_edit(doc, file, inplace, None)


@app.command(name="delete")
def delete_value(
    file: Annotated[str, Parameter(allow_leading_hyphen=True)],
    path: str,
    *,
    inplace: Annotated[bool, Parameter(name=["--inplace", "-i"])] = False,
) -> None:
    """Delete the node at a JSONPath.

    Parameters
    ----------
    file:
        YAML file to edit.
    path:
        JSONPath of the node to delete (wildcards not allowed).
    """
    from pyrs_yaml import YamlPathError

    doc = load_document(file)
    try:
        doc.delete(path)
    except (YamlPathError, KeyError, IndexError) as exc:
        fail(f"cannot delete {path!r}: {exc}")
    _finish_edit(doc, file, inplace, None)


@app.command(name="rename")
def rename_value(
    file: Annotated[str, Parameter(allow_leading_hyphen=True)],
    path: str,
    new_key: str,
    *,
    inplace: Annotated[bool, Parameter(name=["--inplace", "-i"])] = False,
) -> None:
    """Rename a mapping key.

    Parameters
    ----------
    file:
        YAML file to edit.
    path:
        JSONPath of the key to rename (wildcards not allowed).
    new_key:
        The new mapping key.
    """
    from pyrs_yaml import YamlPathError

    doc = load_document(file)
    try:
        doc.rename(path, new_key)
    except (YamlPathError, KeyError, IndexError) as exc:
        fail(f"cannot rename {path!r}: {exc}")
    _finish_edit(doc, file, inplace, None)


@app.command
def validate(
    file: Annotated[str, Parameter(allow_leading_hyphen=True)] = STDIN_FILE,
    *,
    schema: Annotated[str, Parameter(name=["--schema"], required=True)],
) -> None:
    """Validate YAML against a schema (registered name or schema-definition file).

    Parameters
    ----------
    file:
        YAML file to validate; ``-`` or omitted reads stdin.
    schema:
        Registered schema name, or path to a schema definition file.
    """
    from pyrs_yaml import YamlValidateError, validate_against_schema

    data = read_text(file)
    schema_src = read_text(schema) if Path(schema).exists() else schema
    try:
        validate_against_schema(data, schema_src)
    except YamlValidateError as exc:
        fail(str(exc))


@app.command(name="to-json")
def to_json_cmd(
    file: Annotated[str, Parameter(allow_leading_hyphen=True)] = STDIN_FILE,
    *,
    output: Annotated[str | None, Parameter(name=["--output", "-o"], allow_leading_hyphen=True)] = None,
    indent: Annotated[int, Parameter(name=["--indent"])] = 2,
) -> None:
    """Convert YAML to JSON.

    Parameters
    ----------
    file:
        YAML file to convert; ``-`` or omitted reads stdin.
    """
    doc = load_document(file)
    text = doc.to_json(indent=indent)
    emit(text if text.endswith("\n") else text + "\n", output)


@app.command(name="from-json")
def from_json_cmd(
    file: Annotated[str, Parameter(allow_leading_hyphen=True)] = STDIN_FILE,
    *,
    output: Annotated[str | None, Parameter(name=["--output", "-o"], allow_leading_hyphen=True)] = None,
) -> None:
    """Convert JSON to YAML.

    Parameters
    ----------
    file:
        JSON file to convert; ``-`` or omitted reads stdin.
    """
    from pyrs_yaml import YamlParseError, from_json

    text = read_text(file)
    try:
        emit(from_json(text), output)
    except YamlParseError as exc:
        fail(str(exc))
