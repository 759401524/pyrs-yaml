"""
Test various tab scenarios
"""

import pyyaml_rs


def test_tab_only():
    """Test tab character only"""
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


def test_escaped_tab():
    """Test escaped tab sequence"""
    yaml_str = 'key: "hello\\tworld"'
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "hello" in result
    assert "world" in result


if __name__ == '__main__':
    test_tab_only()
    test_backslash_tab()
    test_escaped_tab()
