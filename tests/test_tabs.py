"""Test tab handling in quoted scalars"""

import pytest

import pyrs_yaml


@pytest.mark.parametrize(
    "yaml_str,expected",
    [
        ('key: "hello\tworld"', "hello"),
        ('key: "hello\\\\\tworld"', "hello"),
        ('"line1\nline2"', "line1"),
        ('"1 leading\n    \\\\ttab"', "leading"),
        ('"hello\tworld"', "hello"),
    ],
    ids=["tab-in-double-quoted", "backslash-tab", "multiline-quoted", "backslash-tab-leading", "direct-tab-standalone"],
)
def test_parses_tab_in_quoted_scalar(yaml_str, expected):
    assert expected in pyrs_yaml.parse(yaml_str).to_yaml()
