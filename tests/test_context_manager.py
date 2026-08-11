"""YamlDocument `with` transaction semantics."""

import pytest

import pyrs_yaml


def test_with_clean_exit_keeps_edits():
    doc = pyrs_yaml.parse("a: 1\n")
    with doc:
        doc.set("$.b", 2)
    assert doc.to_yaml() == "a: 1\nb: 2\n"


def test_with_exception_rolls_back():
    doc = pyrs_yaml.parse("a: 1\n")
    with pytest.raises(ValueError), doc:
        doc.set("$.b", 2)
        raise ValueError("boom")
    assert doc.to_yaml() == "a: 1\n"


def test_with_returns_self():
    doc = pyrs_yaml.parse("a: 1\n")
    with doc as d:
        assert d is doc


def test_with_nested_rollback_outer_snapshot():
    doc = pyrs_yaml.parse("a: 1\n")
    with pytest.raises(RuntimeError), doc:
        doc.set("$.b", 2)
        with doc:
            doc.set("$.c", 3)
            raise RuntimeError("inner")
    assert doc.to_yaml() == "a: 1\n"


def test_with_after_previous_edits_keeps_them():
    doc = pyrs_yaml.parse("a: 1\n")
    doc.set("$.b", 2)
    with pytest.raises(ValueError), doc:
        doc.set("$.c", 3)
        raise ValueError("boom")
    assert doc.to_yaml() == "a: 1\nb: 2\n"
