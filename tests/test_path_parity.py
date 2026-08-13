"""Path-parsing parity tests: lock _parse_jsonpath segment structure.

The Rust parse_path_segments is deprecated (no active callers); the Python
side _parse_jsonpath is the sole active path parser. These tests lock its
segment structure as a contract.
"""

import pytest

from pyrs_yaml.node import _parse_jsonpath


def test_simple_key():
    assert _parse_jsonpath("$.a") == ["a"]


def test_nested_keys():
    assert _parse_jsonpath("$.a.b") == ["a", "b"]


def test_sequence_index():
    assert _parse_jsonpath("$.arr[0]") == ["arr", 0]


def test_negative_index():
    assert _parse_jsonpath("$.arr[-1]") == ["arr", -1]


def test_root_only():
    assert _parse_jsonpath("$") == []


def test_nested_index_and_key():
    assert _parse_jsonpath("$[0].key") == [0, "key"]


def test_deep_scan():
    assert _parse_jsonpath("$..key") == ["..key"]


def test_wildcard():
    assert _parse_jsonpath("$[*]") == ["*"]


def test_no_dollar_prefix_raises():
    with pytest.raises(ValueError, match="Path must start with \\$"):
        _parse_jsonpath("a.b")


def test_invalid_bracket_raises():
    with pytest.raises(ValueError, match="invalid index"):
        _parse_jsonpath("$[bad")
