"""
Test tab handling in quoted scalars
"""

import pyamlium_custom


def test_tab_in_double_quoted():
    """Test tab character in double-quoted scalar"""
    yaml_str = 'key: "hello\tworld"'
    print(f'Input: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'Output: {repr(doc.to_yaml())}')
    except Exception as e:
        print(f'Error: {e}')


def test_backslash_tab():
    """Test backslash followed by tab"""
    yaml_str = 'key: "hello\\\\\tworld"'
    print(f'Input: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'Output: {repr(doc.to_yaml())}')
    except Exception as e:
        print(f'Error: {e}')


def test_multiline_quoted():
    """Test multiline quoted scalar"""
    yaml_str = '"line1\nline2"'
    print(f'Input: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'Output: {repr(doc.to_yaml())}')
    except Exception as e:
        print(f'Error: {e}')


if __name__ == '__main__':
    test_tab_in_double_quoted()
    print()
    test_backslash_tab()
    print()
    test_multiline_quoted()
