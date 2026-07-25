"""
Test JSON comparison issues
"""

import yaml
import json
import pyamlium_custom


def test_sequence():
    """Test sequence parsing"""
    yaml_str = "- name: John\n- name: Alice"
    doc = pyamlium_custom.parse(yaml_str)
    actual = doc.to_dict()
    expected = [{"name": "John"}, {"name": "Alice"}]
    print(f"Actual: {actual}")
    print(f"Expected: {expected}")
    print(f"Match: {actual == expected}")


def test_mapping():
    """Test mapping parsing"""
    yaml_str = "name: John\nage: 30"
    doc = pyamlium_custom.parse(yaml_str)
    actual = doc.to_dict()
    expected = {"name": "John", "age": 30}
    print(f"Actual: {actual}")
    print(f"Expected: {expected}")
    print(f"Match: {actual == expected}")


def test_nested():
    """Test nested parsing"""
    yaml_str = "a:\n  b: 1\n  c: 2"
    doc = pyamlium_custom.parse(yaml_str)
    actual = doc.to_dict()
    expected = {"a": {"b": 1, "c": 2}}
    print(f"Actual: {actual}")
    print(f"Expected: {expected}")
    print(f"Match: {actual == expected}")


def test_anchor_alias():
    """Test anchor/alias parsing"""
    yaml_str = "defaults: &d\n  timeout: 30\nprod:\n  <<: *d\n  host: prod.com"
    doc = pyamlium_custom.parse(yaml_str)
    actual = doc.to_dict()
    expected = {"defaults": {"timeout": 30}, "prod": {"timeout": 30, "host": "prod.com"}}
    print(f"Actual: {actual}")
    print(f"Expected: {expected}")
    print(f"Match: {actual == expected}")


if __name__ == '__main__':
    test_sequence()
    print()
    test_mapping()
    print()
    test_nested()
    print()
    test_anchor_alias()
