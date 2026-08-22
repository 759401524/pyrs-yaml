"""Cyclopts application wiring all ``pyrs-yaml`` CLI commands."""

from __future__ import annotations

import json as _json
import sys
from pathlib import Path
from typing import Annotated, Any

from cyclopts import App, Parameter

import pyrs_yaml

from ._io import emit, fail, is_stdio, join_documents, load_document, load_documents, read_text

app = App(
    name="pyrs-yaml",
    help="High-performance YAML toolkit with perfect round-trip support.",
    version=pyrs_yaml.__version__,
)

STDIN_FILE = "-"
"""Default input source; "-" means stdin."""

ALL_DOCS = Annotated[bool, Parameter(name=["--all-docs", "-A"])]
INPLACE = Annotated[bool, Parameter(name=["--inplace", "-i"])]
OUTPUT = Annotated[str | None, Parameter(name=["--output", "-o"], allow_leading_hyphen=True)]
FILE_ARG = Annotated[str, Parameter(allow_leading_hyphen=True)]
"""File-path parameter accepting the bare "-" token (stdin/stdout)."""


def _serialize(doc: Any, indent: int) -> str:
    from pyrs_yaml import YamlSerializeError

    try:
        if indent == 2:
            return doc.to_yaml()
        return doc.to_yaml_with_options(indent_size=indent)
    except YamlSerializeError as exc:
        fail(str(exc))


def _finish_edits(
    docs: list[Any],
    target_file: str,
    inplace: bool,
    output: str | None,
) -> None:
    """Write edited documents back out.

    Single-document edits re-serialize as one stream; multi-document edits are
    joined with standard ``---`` separators.
    """
    texts = [_serialize(doc, 2) for doc in docs]
    text = texts[0] if len(texts) == 1 else join_documents(texts)
    if inplace:
        if is_stdio(target_file):
            fail("cannot use --inplace with stdin input; use -o/--output instead")
        Path(target_file).write_text(text, encoding="utf-8")
    else:
        emit(text, output)


@app.command(name="fmt")
def fmt(
    file: FILE_ARG = STDIN_FILE,
    *,
    output: OUTPUT = None,
    inplace: INPLACE = False,
    indent: Annotated[int, Parameter(name=["--indent"])] = 2,
    all_docs: ALL_DOCS = False,
) -> None:
    """Reformat YAML with round-trip fidelity (comments/anchors/order preserved).

    Parameters
    ----------
    file:
        YAML file to format; ``-`` or omitted reads stdin.
    all_docs:
        Treat the input as a stream of documents separated by ``---``.
    """
    docs = load_documents(file) if all_docs else [load_document(file)]
    texts = [_serialize(doc, indent) for doc in docs]
    text = texts[0] if len(texts) == 1 else join_documents(texts)
    if inplace:
        if is_stdio(file):
            fail("cannot use --inplace with stdin input; use -o/--output instead")
        Path(file).write_text(text, encoding="utf-8")
    else:
        emit(text, output)


def _query_chunks(doc: Any, path: str, format: str) -> list[str]:
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
    return chunks


@app.command
def get(
    file: FILE_ARG = STDIN_FILE,
    path: str = "$",
    *,
    format: Annotated[str, Parameter(name=["--format", "-f"])] = "yaml",
    all_docs: ALL_DOCS = False,
) -> None:
    """Query values by JSONPath (e.g. ``$.servers[0].host``, ``$..name``).

    Parameters
    ----------
    file:
        YAML file to query; ``-`` or omitted reads stdin.
    path:
        JSONPath expression starting with ``$``.
    all_docs:
        Query every document in the input stream; matches print in order.
    """
    if format not in ("yaml", "json", "text"):
        fail(f"invalid --format {format!r}; expected yaml, json or text")

    docs = load_documents(file) if all_docs else [load_document(file)]
    chunks: list[str] = []
    for doc in docs:
        chunks.extend(_query_chunks(doc, path, format))
    if not chunks:
        fail(f"no match for path {path!r}")
    sys.stdout.write("\n".join(chunks) + "\n")


def _parse_value(value: str, string: bool) -> Any:
    from pyrs_yaml import YamlParseError, safe_load

    if string:
        return value
    try:
        return safe_load(value)
    except YamlParseError:
        return value


@app.command(name="set")
def set_value(
    file: FILE_ARG,
    path: str,
    value: Annotated[str, Parameter(allow_leading_hyphen=True)],
    *,
    inplace: INPLACE = False,
    string: Annotated[bool, Parameter(name=["--string", "-s"])] = False,
    create_missing: Annotated[bool, Parameter(name=["--create-missing"])] = False,
    all_docs: ALL_DOCS = False,
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
    all_docs:
        Apply to every document where the path resolves; fails only when no
        document matches.
    """
    from pyrs_yaml import YamlEditError, YamlPathError

    parsed = _parse_value(value, string)
    docs = load_documents(file) if all_docs else [load_document(file)]
    applied = 0
    for doc in docs:
        try:
            doc.set(path, parsed, create_missing=create_missing)
            applied += 1
        except (YamlEditError, YamlPathError, KeyError, IndexError):
            if not all_docs:
                fail(f"cannot set {path!r}: path does not resolve")
    if applied == 0:
        fail(f"cannot set {path!r}: no document resolves the path")
    _finish_edits(docs, file, inplace, None)


@app.command(name="delete")
def delete_value(
    file: FILE_ARG,
    path: str,
    *,
    inplace: INPLACE = False,
    all_docs: ALL_DOCS = False,
) -> None:
    """Delete the node at a JSONPath.

    Parameters
    ----------
    file:
        YAML file to edit.
    path:
        JSONPath of the node to delete (wildcards not allowed).
    all_docs:
        Delete from every document where the path resolves; fails only when
        no document matches.
    """
    from pyrs_yaml import YamlEditError, YamlPathError

    docs = load_documents(file) if all_docs else [load_document(file)]
    applied = 0
    for doc in docs:
        try:
            doc.delete(path)
            applied += 1
        except (YamlEditError, YamlPathError, KeyError, IndexError):
            if not all_docs:
                fail(f"cannot delete {path!r}: path does not resolve")
    if applied == 0:
        fail(f"cannot delete {path!r}: no document resolves the path")
    _finish_edits(docs, file, inplace, None)


@app.command(name="rename")
def rename_value(
    file: FILE_ARG,
    path: str,
    new_key: str,
    *,
    inplace: INPLACE = False,
    all_docs: ALL_DOCS = False,
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
    all_docs:
        Rename in every document where the path resolves; fails only when no
        document matches.
    """
    from pyrs_yaml import YamlEditError, YamlPathError

    docs = load_documents(file) if all_docs else [load_document(file)]
    applied = 0
    for doc in docs:
        try:
            doc.rename(path, new_key)
            applied += 1
        except (YamlEditError, YamlPathError, KeyError, IndexError):
            if not all_docs:
                fail(f"cannot rename {path!r}: path does not resolve")
    if applied == 0:
        fail(f"cannot rename {path!r}: no document resolves the path")
    _finish_edits(docs, file, inplace, None)


@app.command(name="sort-keys")
def sort_keys(
    file: FILE_ARG,
    path: str = "$",
    *,
    inplace: INPLACE = False,
    all_docs: ALL_DOCS = False,
) -> None:
    """Sort mapping keys (alphabetically) at a JSONPath.

    Parameters
    ----------
    file:
        YAML file to edit.
    path:
        Mapping node whose keys to sort (default: the root).
    all_docs:
        Sort in every document where the path resolves; fails only when no
        document matches.
    """
    from pyrs_yaml import YamlEditError, YamlPathError

    docs = load_documents(file) if all_docs else [load_document(file)]
    applied = 0
    for doc in docs:
        try:
            doc.sort_keys(path)
            applied += 1
        except (YamlEditError, YamlPathError, KeyError, IndexError):
            if not all_docs:
                fail(f"cannot sort keys at {path!r}: path does not resolve")
    if applied == 0:
        fail(f"cannot sort keys at {path!r}: no document resolves the path")
    _finish_edits(docs, file, inplace, None)


@app.command(name="move")
def move_value(
    file: FILE_ARG,
    source: str,
    destination: str,
    *,
    inplace: INPLACE = False,
) -> None:
    """Move a subtree to another path within the same document.

    Parameters
    ----------
    file:
        YAML file to edit.
    source:
        JSONPath of the subtree to move (wildcards not allowed).
    destination:
        Absolute JSONPath the subtree is moved to. The destination node must
        already exist and its value is replaced (e.g. ``$.nested`` below).
    """
    from pyrs_yaml import Node, YamlEditError, YamlPathError

    for label, p in (("source", source), ("destination", destination)):
        if "*" in p or ".." in p:
            fail(f"{label} path must not contain wildcards or deep scans")
    doc = load_document(file)
    try:
        found = Node(doc).find(source)
        if isinstance(found, list):
            fail(f"source path {source!r} matched multiple nodes")
        found.move(destination)
    except (YamlEditError, YamlPathError, KeyError, IndexError, ValueError) as exc:
        fail(f"cannot move {source!r} to {destination!r}: {exc}")
    _finish_edits([doc], file, inplace, None)


@app.command(name="frontmatter")
def frontmatter(
    file: FILE_ARG = STDIN_FILE,
    *,
    body_out: Annotated[str | None, Parameter(name=["--body-out"], allow_leading_hyphen=True)] = None,
) -> None:
    """Extract Markdown front matter as YAML.

    Parameters
    ----------
    file:
        Markdown file to read; ``-`` or omitted reads stdin.
    body_out:
        Optionally write the Markdown body (without front matter) to this path.
    """
    from pyrs_yaml import from_dict, read_markdown_str

    text = read_text(file)
    meta, body = read_markdown_str(text)
    if meta is None:
        fail(f"no front matter found in '{file}'")
    emit(from_dict(meta))
    if body_out is not None:
        emit(body, body_out)


@app.command
def validate(
    file: FILE_ARG = STDIN_FILE,
    *,
    schema: Annotated[str | None, Parameter(name=["--schema"])] = None,
    schema_file: Annotated[str | None, Parameter(name=["--schema-file"])] = None,
    all_docs: ALL_DOCS = False,
) -> None:
    """Validate YAML against a schema (registered name or schema-definition file).

    Parameters
    ----------
    file:
        YAML file to validate; ``-`` or omitted reads stdin.
    schema:
        Registered schema name (mutually exclusive with --schema-file).
    schema_file:
        Path to a schema definition file (mutually exclusive with --schema).
    all_docs:
        Validate every document in the input stream.
    """
    from pyrs_yaml import YamlValidateError, validate_against_schema

    if schema and schema_file:
        fail("--schema and --schema-file are mutually exclusive")
    if not schema and not schema_file:
        fail("one of --schema or --schema-file is required")

    schema_src: str = read_text(schema_file) if schema_file else (schema or "")
    docs = load_documents(file) if all_docs else [load_document(file)]
    for i, doc in enumerate(docs):
        prefix = f"document {i}: " if len(docs) > 1 else ""
        try:
            validate_against_schema(doc.to_yaml(), schema_src)
        except YamlValidateError as exc:
            fail(prefix + str(exc))


@app.command(name="to-json")
def to_json_cmd(
    file: FILE_ARG = STDIN_FILE,
    *,
    output: OUTPUT = None,
    indent: Annotated[int, Parameter(name=["--indent"])] = 2,
    all_docs: ALL_DOCS = False,
) -> None:
    """Convert YAML to JSON.

    Parameters
    ----------
    file:
        YAML file to convert; ``-`` or omitted reads stdin.
    all_docs:
        Convert the whole stream into a JSON array of documents.
    """
    docs = load_documents(file) if all_docs else [load_document(file)]
    text = _json.dumps([doc.to_dict() for doc in docs], indent=indent) if all_docs else docs[0].to_json(indent=indent)
    emit(text if text.endswith("\n") else text + "\n", output)


@app.command(name="from-json")
def from_json_cmd(
    file: FILE_ARG = STDIN_FILE,
    *,
    output: OUTPUT = None,
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


@app.command
def compliance(
    suite_dir: Annotated[str | None, Parameter(allow_leading_hyphen=True)] = None,
    *,
    json_output: Annotated[bool, Parameter(name=["--json"])] = False,
) -> None:
    """Report YAML Test Suite compliance for the bundled parser.

    Parameters
    ----------
    suite_dir:
        Path to a yaml-test-suite checkout; defaults to ./Reference/yaml-test-suite.
    json_output:
        Emit machine-readable JSON instead of the formatted report.
    """
    from pyrs_yaml.compliance import compliance_report

    report = compliance_report(suite_dir)
    if json_output:
        print(_json.dumps(report, indent=2))
    else:
        print(report)
