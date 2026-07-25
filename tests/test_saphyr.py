"""
Test saphyr-parser vs yaml-rust2
"""

import saphyr_parser
import yaml_rust2


def test_basic_parse():
    """Test basic parsing"""
    yaml_str = "name: John\nage: 30"

    # saphyr-parser
    print("=== saphyr-parser ===")
    try:
        docs = saphyr_parser.load(yaml_str)
        print(f"Success: {docs}")
    except Exception as e:
        print(f"Error: {e}")

    # yaml-rust2
    print("\n=== yaml-rust2 ===")
    try:
        docs = yaml_rust2.load(yaml_str)
        print(f"Success: {docs}")
    except Exception as e:
        print(f"Error: {e}")


if __name__ == '__main__':
    test_basic_parse()
