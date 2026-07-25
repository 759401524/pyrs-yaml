"""
Test JSON comparison issues
"""

import pyyaml_rs


def test_sequence():
    """Test sequence parsing"""
    yaml_str = "- name: John\n- name: Alice"
    doc = pyyaml_rs.parse(yaml_str)
    actual = doc.to_dict()
    expected = [{"name": "John"}, {"name": "Alice"}]
    assert actual == expected


def test_mapping():
    """Test mapping parsing"""
    yaml_str = "name: John\nage: 30"
    doc = pyyaml_rs.parse(yaml_str)
    actual = doc.to_dict()
    expected = {"name": "John", "age": 30}
    assert actual == expected


def test_nested():
    """Test nested parsing"""
    yaml_str = "a:\n  b: 1\n  c: 2"
    doc = pyyaml_rs.parse(yaml_str)
    actual = doc.to_dict()
    expected = {"a": {"b": 1, "c": 2}}
    assert actual == expected


def test_anchor_alias():
    """Test anchor/alias parsing"""
    yaml_str = "defaults: &d\n  timeout: 30\nprod:\n  <<: *d\n  host: prod.com"
    doc = pyyaml_rs.parse(yaml_str)
    actual = doc.to_dict()
    expected = {"defaults": {"timeout": 30}, "prod": {"timeout": 30, "host": "prod.com"}}
    assert actual == expected


if __name__ == '__main__':
    test_sequence()
    test_mapping()
    test_nested()
    test_anchor_alias()
