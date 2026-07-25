"""
Test backslash followed by tab
"""

import pyamlium_custom


def test_backslash_tab():
    """Test backslash followed by tab escape"""
    # In the test file, this is: "1 leading\n    \\ttab"
    # Which means: literal backslash followed by tab
    yaml_str = '"1 leading\n    \\\\ttab"'
    print(f'Input: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'Output: {repr(doc.to_yaml())}')
    except Exception as e:
        print(f'Error: {e}')


def test_direct_tab():
    """Test direct tab character"""
    yaml_str = '"hello\tworld"'
    print(f'Input: {repr(yaml_str)}')
    try:
        doc = pyamlium_custom.parse(yaml_str)
        print(f'Output: {repr(doc.to_yaml())}')
    except Exception as e:
        print(f'Error: {e}')


if __name__ == '__main__':
    test_backslash_tab()
    print()
    test_direct_tab()
