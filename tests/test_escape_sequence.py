"""
Test escape sequences in double-quoted scalars
"""

import pyamlium_custom


def test_tab_escape():
    """Test \\t escape sequence"""
    # This is the YAML: "1 leading\n    \ttab"
    # In Python string: "\"1 leading\\n    \\ttab\""
    yaml_str = "\"1 leading\\n    \\ttab\""
    print(f'Input: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'Output: {repr(doc.to_yaml())}')
    except Exception as e:
        print(f'Error: {e}')


def test_backslash_escape():
    """Test \\\\ escape sequence"""
    yaml_str = "\"hello\\\\world\""
    print(f'Input: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'Output: {repr(doc.to_yaml())}')
    except Exception as e:
        print(f'Error: {e}')


def test_newline_escape():
    """Test \\n escape sequence"""
    yaml_str = "\"hello\\nworld\""
    print(f'Input: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'Output: {repr(doc.to_yaml())}')
    except Exception as e:
        print(f'Error: {e}')


if __name__ == '__main__':
    test_tab_escape()
    print()
    test_backslash_escape()
    print()
    test_newline_escape()
