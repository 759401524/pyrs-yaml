"""
Test backslash followed by tab
"""

import pyyaml_rs


def test_backslash_tab():
    """Test backslash followed by tab escape"""
    yaml_str = '"1 leading\n    \\\\ttab"'
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert result is not None
    assert "leading" in result
    assert "tab" in result


def test_direct_tab():
    """Test direct tab character"""
    yaml_str = '"hello\tworld"'
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "hello" in result
    assert "world" in result


if __name__ == '__main__':
    test_backslash_tab()
    test_direct_tab()
