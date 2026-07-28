"""
Test tab handling in quoted scalars
"""

import pyrs_yaml


def test_tab_in_double_quoted():
    """Test tab character in double-quoted scalar"""
    yaml_str = 'key: "hello\tworld"'
    doc = pyrs_yaml.parse(yaml_str)
    result = doc.to_yaml()
    assert "hello" in result
    assert "world" in result


def test_backslash_tab():
    """Test backslash followed by tab"""
    yaml_str = 'key: "hello\\\\\tworld"'
    doc = pyrs_yaml.parse(yaml_str)
    result = doc.to_yaml()
    assert "hello" in result
    assert "world" in result


def test_multiline_quoted():
    """Test multiline quoted scalar"""
    yaml_str = '"line1\nline2"'
    doc = pyrs_yaml.parse(yaml_str)
    result = doc.to_yaml()
    assert "line1" in result
    assert "line2" in result


def test_backslash_tab_leading():
    """Test backslash followed by tab in leading context"""
    yaml_str = '"1 leading\n    \\\\ttab"'
    doc = pyrs_yaml.parse(yaml_str)
    result = doc.to_yaml()
    assert result is not None
    assert "leading" in result
    assert "tab" in result


def test_direct_tab_standalone():
    """Test direct tab character in standalone scalar"""
    yaml_str = '"hello\tworld"'
    doc = pyrs_yaml.parse(yaml_str)
    result = doc.to_yaml()
    assert "hello" in result
    assert "world" in result
