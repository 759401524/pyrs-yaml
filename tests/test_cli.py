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
    run("validate", "--schema", str(schema))
    assert capsys.readouterr().out == ""


def test_validate_failure_exits_1(tmp_path, capsys, monkeypatch):
    schema = tmp_path / "schema.yaml"
    schema.write_text(VALIDATE_SCHEMA, encoding="utf-8")
    feed_stdin(monkeypatch, "port: abc\n")
    with pytest.raises(SystemExit) as exc:
        run("validate", "--schema", str(schema))
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
    for cmd in ("fmt", "get", "set", "delete", "rename", "validate", "to-json", "from-json"):
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
