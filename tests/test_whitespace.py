"""
Test whitespace handling
"""

import pyyaml_rs


def test_trailing_spaces():
    """Test trailing spaces in mappings"""
    yaml_str = '"top1" :   \n  "key1" : scalar1'
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "top1" in result
    assert "key1" in result


def test_tabs():
    """Test tab characters"""
    yaml_str = "key: value\t"
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "key" in result
    assert "value" in result


def test_empty_lines():
    """Test empty lines between mappings"""
    yaml_str = "a: 1\n\nb: 2"
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "a" in result
    assert "b" in result
    assert "1" in result
    assert "2" in result


if __name__ == '__main__':
    test_trailing_spaces()
    test_tabs()
    test_empty_lines()
