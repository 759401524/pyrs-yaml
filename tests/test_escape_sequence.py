"""
Test escape sequences in double-quoted scalars
"""

import pyyaml_rs


def test_tab_escape():
    """Test \\t escape sequence"""
    yaml_str = "\"1 leading\\n    \\ttab\""
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "leading" in result
    assert "tab" in result


def test_backslash_escape():
    """Test \\\\ escape sequence"""
    yaml_str = "\"hello\\\\world\""
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "hello" in result
    assert "world" in result


def test_newline_escape():
    """Test \\n escape sequence"""
    yaml_str = "\"hello\\nworld\""
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "hello" in result
    assert "world" in result


if __name__ == '__main__':
    test_tab_escape()
    test_backslash_escape()
    test_newline_escape()
