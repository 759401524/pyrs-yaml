"""Serialization snapshot tests — guard against format regressions.

These tests lock in the exact serialized output for representative inputs so
that refactoring the serializer (e.g. sharing formatting logic with
`DirectWriter`) cannot silently change output format.
"""

import pytest

import pyrs_yaml

# (input, expected_exact_output) — each entry is a contract for round-trip fidelity.
SERIALIZE_SNAPSHOTS = [
    ("key: value", "key: value\n"),
    ("a: 1\nb: 2", "a: 1\nb: 2\n"),
    ("- a\n- b", "- a\n- b\n"),
    ("nested:\n  key: value", "nested:\n  key: value\n"),
    ("list:\n  - a\n  - b", "list:\n  - a\n  - b\n"),
    ("flow: {a: 1, b: 2}", "flow: {a: 1, b: 2}\n"),
    ("seq: [1, 2, 3]", "seq: [1, 2, 3]\n"),
    ("multi:\n  - a: 1\n    b: 2\n", "multi:\n  - a: 1\n    b: 2\n"),
    ("# comment\nkey: value", "# comment\nkey: value\n"),
    ("anchor: &a value\nref: *a", "anchor: &a value\nref: *a\n"),
]


@pytest.mark.parametrize("yaml_input,expected", SERIALIZE_SNAPSHOTS, ids=lambda v: str(v)[:20])
def test_serialize_exact_snapshot(yaml_input, expected):
    """Serialize output must match the locked snapshot exactly."""
    doc = pyrs_yaml.parse(yaml_input)
    assert doc.to_yaml() == expected


INDENT_SNAPSHOTS = [
    # (yaml_input, expected_output) — serializer uses fixed 2-space indent
    ("a:\n  b: 1", "a:\n  b: 1\n"),
    ("a:\n  b:\n    c: 2", "a:\n  b:\n    c: 2\n"),
]


@pytest.mark.parametrize("yaml_input,expected", INDENT_SNAPSHOTS, ids=lambda v: str(v)[:20])
def test_serialize_indent_snapshot(yaml_input, expected):
    """Serialize output must preserve the original nesting structure with 2-space indent."""
    doc = pyrs_yaml.parse(yaml_input)
    result = doc.to_yaml()
    assert result == expected
