"""
Test various tab scenarios
"""

import pyamlium_custom


def test_tab_only():
    """Test tab character only"""
    yaml_str = 'key: "hello\tworld"'
    print(f'Input: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'  OK: {repr(doc.to_yaml()[:50])}')
    except Exception as e:
        print(f'  Error: {str(e)[:80]}')


def test_backslash_tab():
    """Test backslash followed by tab"""
    yaml_str = 'key: "hello\\\\\tworld"'
    print(f'Input: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'  OK: {repr(doc.to_yaml()[:50])}')
    except Exception as e:
        print(f'  Error: {str(e)[:80]}')


def test_escaped_tab():
    """Test escaped tab sequence"""
    yaml_str = 'key: "hello\\tworld"'
    print(f'Input: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'  OK: {repr(doc.to_yaml()[:50])}')
    except Exception as e:
        print(f'  Error: {str(e)[:80]}')


if __name__ == '__main__':
    test_tab_only()
    print()
    test_backslash_tab()
    print()
    test_escaped_tab()
