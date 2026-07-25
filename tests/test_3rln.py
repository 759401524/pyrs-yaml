"""
Test the specific 3RLN case
"""

import pyamlium_custom


def test_3rln():
    """Test 3RLN case"""
    # Original YAML with special chars
    yaml_input = '"2 leading\n    \\———\ttab"'

    # Convert special chars
    yaml_converted = yaml_input.replace('\u2016\u2016\u2016\u00bb', '\t')

    print(f'Original: {repr(yaml_input)}')
    print(f'Converted: {repr(yaml_converted)}')
    print()

    try:
        doc = pyamlium_custom.parse(yaml_converted)
        print(f'Output: {repr(doc.to_yaml())}')
    except Exception as e:
        print(f'Error: {e}')

    # Try without the backslash
    yaml_simple = '"2 leading\n    tab"'
    print(f'\nSimple: {repr(yaml_simple)}')
    try:
        doc = pyamlium_custom.parse(yaml_simple)
        print(f'Output: {repr(doc.to_yaml())}')
    except Exception as e:
        print(f'Error: {e}')


if __name__ == '__main__':
    test_3rln()
