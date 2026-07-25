"""
Test tab handling in quoted scalars
"""

import pyyaml_rs


def test_tab_in_double_quoted():
    """Test tab character in double-quoted scalar"""
    yaml_str = 'key: "hello\tworld"'
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "hello" in result
    assert "world" in result


def test_backslash_tab():
    """Test backslash followed by tab"""
    yaml_str = 'key: "hello\\\\\tworld"'
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "hello" in result
    assert "world" in result


def test_multiline_quoted():
    """Test multiline quoted scalar"""
    yaml_str = '"line1\nline2"'
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "line1" in result
    assert "line2" in result


if __name__ == '__main__':
    test_tab_in_double_quoted()
    test_backslash_tab()
    test_multiline_quoted()
