"""
Test whitespace handling
"""

import pyamlium_custom


def test_trailing_spaces():
    """Test trailing spaces in mappings"""
    yaml_str = '"top1" :   \n  "key1" : scalar1'
    print(f'YAML: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'Output: {doc.to_yaml()}')
    except Exception as e:
        print(f'Error: {e}')


def test_tabs():
    """Test tab characters"""
    yaml_str = "key: value\t"
    print(f'YAML: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'Output: {repr(doc.to_yaml())}')
    except Exception as e:
        print(f'Error: {e}')


def test_empty_lines():
    """Test empty lines between mappings"""
    yaml_str = "a: 1\n\nb: 2"
    print(f'YAML: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'Output: {repr(doc.to_yaml())}')
    except Exception as e:
        print(f'Error: {e}')


if __name__ == '__main__':
    test_trailing_spaces()
    print()
    test_tabs()
    print()
    test_empty_lines()
