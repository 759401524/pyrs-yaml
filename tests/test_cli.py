"""Tests for the pyrs-yaml CLI (requires the optional cyclopts extra)."""

from __future__ import annotations

import builtins
import io
import json
import sys

import pytest

pytest.importorskip("cyclopts")

from pyrs_yaml.cli import main as cli_main
from pyrs_yaml.cli.app import app

VALIDATE_SCHEMA = """\
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
"""


def run(*tokens: str) -> None:
    """Invoke the app in-process like a console script.

    Cyclopts raises ``SystemExit(0)`` after every successful command;
    only non-zero exits are surfaced to the caller.
    """
    try:
        app(list(tokens))
    except SystemExit as exc:
        if exc.code:
            raise


def feed_stdin(monkeypatch, text):
    monkeypatch.setattr(sys, "stdin", io.StringIO(text))


def read(tmp_path, name="doc.yaml"):
    return (tmp_path / name).read_text(encoding="utf-8")


# ── fmt ──────────────────────────────────────────────────────────────────────


def test_fmt_stdin_preserves_comment(capsys, monkeypatch):
    feed_stdin(monkeypatch, "a: 1 # keep me\n")
    run("fmt")
    assert "# keep me" in capsys.readouterr().out


def test_fmt_file_round_trip(tmp_path):
    src = tmp_path / "doc.yaml"
    src.write_text("list:\n  - one # first\nanchor: &a value\nuse: *a\n", encoding="utf-8")
    run("fmt", str(src))
    assert src.read_text(encoding="utf-8") == "list:\n  - one # first\nanchor: &a value\nuse: *a\n"


def test_fmt_inplace(tmp_path):
    src = tmp_path / "doc.yaml"
    src.write_text("a:    1\n", encoding="utf-8")
    run("fmt", str(src), "--inplace")
    assert src.read_text(encoding="utf-8") == "a: 1\n"


def test_fmt_indent_option(capsys, monkeypatch):
    feed_stdin(monkeypatch, "outer:\n inner: 1\n")
    run("fmt", "--indent", "4")
    assert capsys.readouterr().out == "outer:\n    inner: 1\n"


def test_fmt_output_file(tmp_path, monkeypatch):
    feed_stdin(monkeypatch, "a: 1\n")
    dst = tmp_path / "out.yaml"
    run("fmt", "-o", str(dst))
    assert dst.read_text(encoding="utf-8") == "a: 1\n"


def test_fmt_inplace_rejected_on_stdin(monkeypatch):
    feed_stdin(monkeypatch, "a: 1\n")
    with pytest.raises(SystemExit) as exc:
        run("fmt", "--inplace")
    assert exc.value.code == 1


# ── get ──────────────────────────────────────────────────────────────────────


def test_get_scalar_yaml_default(capsys, monkeypatch):
    feed_stdin(monkeypatch, "b:\n  c: 42\n")
    run("get", "-", "$.b.c")
    assert capsys.readouterr().out == "42\n"


def test_get_text_format(capsys, monkeypatch):
    feed_stdin(monkeypatch, "names:\n  - x\n  - y\n")
    run("get", "-", "$.names[*]", "--format", "text")
    assert capsys.readouterr().out == "x\ny\n"


def test_get_json_format(capsys, monkeypatch):
    feed_stdin(monkeypatch, "b:\n  c: [1, 2]\n")
    run("get", "-", "$.b.c", "--format", "json")
    assert json.loads(capsys.readouterr().out) == [1, 2]


def test_get_deep_scan(capsys, monkeypatch):
    feed_stdin(monkeypatch, "a:\n  b:\n    name: deep\nname: top\n")
    run("get", "-", "$..name", "--format", "text")
    out = capsys.readouterr().out.splitlines()
    assert "deep" in out and "top" in out


def test_get_mapping_outputs_yaml(capsys, monkeypatch):
    feed_stdin(monkeypatch, "b:\n  c: 1\n  d: 2\n")
    run("get", "-", "$.b")
    out = capsys.readouterr().out
    assert "c: 1" in out and "d: 2" in out


def test_get_no_match_exits_1(capsys, monkeypatch):
    feed_stdin(monkeypatch, "a: 1\n")
    with pytest.raises(SystemExit) as exc:
        run("get", "-", "$.missing")
    assert exc.value.code == 1
    assert "no match" in capsys.readouterr().err


def test_get_bad_path_exits_1(capsys, monkeypatch):
    feed_stdin(monkeypatch, "a: 1\n")
    with pytest.raises(SystemExit) as exc:
        run("get", "a.b")
    assert exc.value.code == 1


# ── set / delete / rename ────────────────────────────────────────────────────


def test_set_parses_value_as_yaml(capsys, monkeypatch):
    feed_stdin(monkeypatch, "a: 1\n")
    run("set", "-", "$.a", "42")
    assert capsys.readouterr().out == "a: 42\n"


def test_set_string_flag_keeps_literal(capsys, monkeypatch):
    feed_stdin(monkeypatch, "a: 1\n")
    run("set", "-", "$.a", "42", "--string")
    assert capsys.readouterr().out == 'a: "42"\n'


def test_set_create_missing(capsys, monkeypatch):
    feed_stdin(monkeypatch, "a: 1\n")
    run("set", "-", "$.x.y", "new", "--create-missing")
    out = capsys.readouterr().out
    assert "x:" in out and "y: new" in out


def test_set_inplace_and_comment_preserved(tmp_path):
    src = tmp_path / "doc.yaml"
    src.write_text("port: 80 # the port\n", encoding="utf-8")
    run("set", str(src), "$.port", "8080", "--inplace")
    text = src.read_text(encoding="utf-8")
    assert "port: 8080" in text and "# the port" in text


def test_set_negative_value_positional(capsys, monkeypatch):
    feed_stdin(monkeypatch, "t: 0\n")
    run("set", "-", "$.t", "-5")
    assert capsys.readouterr().out == "t: -5\n"


def test_delete_removes_node(capsys, monkeypatch):
    feed_stdin(monkeypatch, "a: 1\nb: 2\n")
    run("delete", "-", "$.b")
    assert capsys.readouterr().out == "a: 1\n"


def test_rename_key_preserves_value_and_comment(tmp_path):
    src = tmp_path / "doc.yaml"
    src.write_text("old: v # noted\n", encoding="utf-8")
    run("rename", str(src), "$.old", "fresh", "--inplace")
    text = src.read_text(encoding="utf-8")
    assert "fresh: v" in text and "# noted" in text


# ── validate ─────────────────────────────────────────────────────────────────


def test_validate_schema_file_ok_is_silent(tmp_path, capsys, monkeypatch):
    schema = tmp_path / "schema.yaml"
    schema.write_text(VALIDATE_SCHEMA, encoding="utf-8")
    feed_stdin(monkeypatch, "port: 8080\n")
    run("validate", "--schema-file", str(schema))
    assert capsys.readouterr().out == ""


def test_validate_failure_exits_1(tmp_path, capsys, monkeypatch):
    schema = tmp_path / "schema.yaml"
    schema.write_text(VALIDATE_SCHEMA, encoding="utf-8")
    feed_stdin(monkeypatch, "port: abc\n")
    with pytest.raises(SystemExit) as exc:
        run("validate", "--schema-file", str(schema))
    assert exc.value.code == 1
    assert "expected int" in capsys.readouterr().err


# ── conversions ──────────────────────────────────────────────────────────────


def test_to_json(capsys, monkeypatch):
    feed_stdin(monkeypatch, "b: {c: 2}\n")
    run("to-json")
    assert json.loads(capsys.readouterr().out) == {"b": {"c": 2}}


def test_from_json_round_trip_with_to_json(capsys, monkeypatch):
    feed_stdin(monkeypatch, '{"a": [1, {"b": null}]}\n')
    run("from-json")
    yaml_out = capsys.readouterr().out
    feed_stdin(monkeypatch, yaml_out)
    run("to-json")
    assert json.loads(capsys.readouterr().out) == {"a": [1, {"b": None}]}


# ── errors & app surface ─────────────────────────────────────────────────────


def test_missing_input_file_exits_1(capsys):
    with pytest.raises(SystemExit) as exc:
        run("fmt", "definitely-missing.yaml")
    assert exc.value.code == 1
    assert "cannot read" in capsys.readouterr().err


def test_help_lists_all_commands(capsys):
    with pytest.raises(SystemExit) as exc:
        app(["--help"])
    assert exc.value.code == 0
    out = capsys.readouterr().out
    for cmd in (
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
        "compliance",
    ):
        assert cmd in out


def test_version_flag(capsys):
    with pytest.raises(SystemExit) as exc:
        app(["--version"])
    assert exc.value.code == 0
    assert any(ch.isdigit() for ch in capsys.readouterr().out)


def test_entry_point_without_cyclopts(monkeypatch, capsys):
    monkeypatch.delitem(sys.modules, "pyrs_yaml.cli.app", raising=False)
    real_import = builtins.__import__

    def fake_import(name, *args, **kwargs):
        if name == "cyclopts" or name.startswith("cyclopts."):
            raise ImportError(f"No module named {name!r}", name="cyclopts")
        return real_import(name, *args, **kwargs)

    monkeypatch.setattr(builtins, "__import__", fake_import)
    with pytest.raises(SystemExit) as exc:
        cli_main()
    assert exc.value.code == 1
    err = capsys.readouterr().err
    assert "pip install pyrs-yaml[cli]" in err


# ── validate: explicit --schema / --schema-file (#98) ───────────────────────


def test_validate_registered_schema_name(tmp_path, capsys, monkeypatch):
    import pyrs_yaml

    schema_file = tmp_path / "s.yaml"
    schema_file.write_text(VALIDATE_SCHEMA, encoding="utf-8")
    pyrs_yaml.register_schema("cli_test_schema", VALIDATE_SCHEMA)
    feed_stdin(monkeypatch, "port: 8080\n")
    run("validate", "--schema", "cli_test_schema")
    assert capsys.readouterr().out == ""


def test_validate_both_flags_rejected(capsys):
    with pytest.raises(SystemExit) as exc:
        run("validate", "--schema", "x", "--schema-file", "y.yaml")
    assert exc.value.code == 1
    assert "mutually exclusive" in capsys.readouterr().err


def test_validate_no_schema_flag_rejected(capsys):
    with pytest.raises(SystemExit) as exc:
        run("validate")
    assert exc.value.code == 1
    assert "--schema or --schema-file" in capsys.readouterr().err


# ── multi-document streams: -A/--all-docs (#96) ─────────────────────────────

MULTI_DOC = "a: 1 # one\n---\nb: 2 # two\n"


def test_fmt_all_docs_round_trip(capsys, monkeypatch):
    feed_stdin(monkeypatch, MULTI_DOC)
    run("fmt", "-A")
    assert capsys.readouterr().out == "a: 1  # one\n---\nb: 2  # two\n"


def test_fmt_all_docs_inplace(tmp_path):
    src = tmp_path / "stream.yaml"
    src.write_text("a:    1\n---\nb:    2\n", encoding="utf-8")
    run("fmt", str(src), "-A", "--inplace")
    assert src.read_text(encoding="utf-8") == "a: 1\n---\nb: 2\n"


def test_get_all_docs_across_documents(capsys, monkeypatch):
    feed_stdin(monkeypatch, "name: first\n---\nnested:\n  name: second\n")
    run("get", "-", "$..name", "--format", "text", "-A")
    assert capsys.readouterr().out == "first\nsecond\n"


def test_to_json_all_docs_array(capsys, monkeypatch):
    feed_stdin(monkeypatch, "a: 1\n---\nb: [2]\n")
    run("to-json", "-A")
    assert json.loads(capsys.readouterr().out) == [{"a": 1}, {"b": [2]}]


def test_validate_all_docs_reports_document_index(tmp_path, capsys, monkeypatch):
    schema = tmp_path / "schema.yaml"
    schema.write_text(VALIDATE_SCHEMA, encoding="utf-8")
    feed_stdin(monkeypatch, "port: 80\n---\nport: abc\n")
    with pytest.raises(SystemExit) as exc:
        run("validate", "-", "--schema-file", str(schema), "-A")
    assert exc.value.code == 1
    err = capsys.readouterr().err
    assert "document 1" in err and "expected int" in err


def test_set_all_docs_applies_to_every_document(capsys, monkeypatch):
    feed_stdin(monkeypatch, "x: 0\n---\ny: 1\n")
    run("set", "-", "$.x", "5", "-A")
    assert capsys.readouterr().out == "x: 5\n---\ny: 1\nx: 5\n"


def test_set_all_docs_unresolvable_everywhere_exits_1(capsys, monkeypatch):
    feed_stdin(monkeypatch, "x: 0\n---\ny: 1\n")
    with pytest.raises(SystemExit) as exc:
        run("set", "-", "$.a.b.c", "5", "-A")
    assert exc.value.code == 1
    assert "no document" in capsys.readouterr().err


def test_delete_all_docs_skips_documents_without_match(capsys, monkeypatch):
    feed_stdin(monkeypatch, "drop: me\nkeep: 1\n---\nkeep: 2\n")
    run("delete", "-", "$.drop", "-A")
    assert capsys.readouterr().out == "keep: 1\n---\nkeep: 2\n"


def test_delete_and_rename_all_docs(tmp_path):
    src = tmp_path / "stream.yaml"
    src.write_text("drop: me\nkeep: 1\n---\ndrop: me too\nkeep: 2\n", encoding="utf-8")
    run("delete", str(src), "$.drop", "-A", "--inplace")
    expected = "keep: 1\n---\nkeep: 2\n"
    assert src.read_text(encoding="utf-8") == expected

    src.write_text("old: v1\n---\nold: v2\n", encoding="utf-8")
    run("rename", str(src), "$.old", "fresh", "-A", "--inplace")
    assert src.read_text(encoding="utf-8") == "fresh: v1\n---\nfresh: v2\n"


# ── sort-keys / move / frontmatter (#97) ────────────────────────────────────


def test_sort_keys_root_only(capsys, monkeypatch):
    feed_stdin(monkeypatch, "b: 2\na: 1\nouter:\n  z: 26\n  m: 13\n")
    run("sort-keys", "-")
    out = capsys.readouterr().out
    assert out.index("a:") < out.index("b:")
    assert out.index("z: 26") < out.index("m: 13")


def test_sort_keys_at_subpath(capsys, monkeypatch):
    feed_stdin(monkeypatch, "meta:\n  z: 1\n  a: 2\norder: kept\n")
    run("sort-keys", "-", "$.meta")
    out = capsys.readouterr().out
    assert out.index("a: 2") < out.index("z: 1")
    assert "order: kept" in out


def test_move_subtree_to_existing_destination(capsys, monkeypatch):
    feed_stdin(monkeypatch, "src:\n  v: 1 # keep\nnested: {}\n")
    run("move", "-", "$.src", "$.nested")
    out = capsys.readouterr().out
    assert "nested:" in out and "v: 1" in out and "# keep" in out
    assert "src:" not in out


def test_move_missing_destination_exits_1(capsys, monkeypatch):
    feed_stdin(monkeypatch, "src: {v: 1}\n")
    with pytest.raises(SystemExit) as exc:
        run("move", "-", "$.src", "$.missing.parent")
    assert exc.value.code == 1
    assert "cannot move" in capsys.readouterr().err


def test_move_wildcard_source_rejected(capsys, monkeypatch):
    feed_stdin(monkeypatch, "items: [{v: 1}]\n")
    with pytest.raises(SystemExit) as exc:
        run("move", "-", "$.items[*]", "$.dest")
    assert exc.value.code == 1
    assert "wildcards" in capsys.readouterr().err


MARKDOWN = "---\ntitle: Hello\nrating: 5\n---\n\nBody text.\n"


def test_frontmatter_extraction(capsys, monkeypatch):
    feed_stdin(monkeypatch, MARKDOWN)
    run("frontmatter", "-")
    out = capsys.readouterr().out
    assert "title: Hello" in out and "Body" not in out


def test_frontmatter_body_out(tmp_path, capsys, monkeypatch):
    body_path = tmp_path / "body.md"
    feed_stdin(monkeypatch, MARKDOWN)
    run("frontmatter", "-", "--body-out", str(body_path))
    assert body_path.read_text(encoding="utf-8").strip() == "Body text."


def test_frontmatter_absent_exits_1(capsys, monkeypatch):
    feed_stdin(monkeypatch, "Just plain markdown.\n")
    with pytest.raises(SystemExit) as exc:
        run("frontmatter", "-")
    assert exc.value.code == 1
    assert "no front matter" in capsys.readouterr().err


# ── compliance (#99) ────────────────────────────────────────────────────────


def test_compliance_json_report():
    from pathlib import Path

    if not Path("Reference/yaml-test-suite").exists():
        pytest.skip("YAML Test Suite not found")
    with pytest.raises(SystemExit) as exc:
        app(["compliance", "--json"])
    assert exc.value.code == 0
