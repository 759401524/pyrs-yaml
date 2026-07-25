import pyamlium_custom


def test_roundtrip_with_comments():
    """Test that comments are preserved in round-trip"""
    yaml_str = """# Top comment
key: value  # Inline comment
"""
    doc = pyamlium_custom.parse(yaml_str)
    result = doc.to_yaml()
    print("=== Input ===")
    print(yaml_str)
    print("=== Output ===")
    print(result)
    print("=== Match ===")
    print(yaml_str == result)
    # Note: comments may not be perfectly preserved yet


def test_roundtrip_with_anchors():
    """Test that anchors and aliases work"""
    yaml_str = """defaults: &defaults
  timeout: 30
  retries: 3

production:
  <<: *defaults
  host: prod.example.com
"""
    doc = pyamlium_custom.parse(yaml_str)
    result = doc.to_yaml()
    print("=== Anchors Input ===")
    print(yaml_str)
    print("=== Anchors Output ===")
    print(result)


def test_roundtrip_multiline():
    """Test multi-line strings"""
    yaml_str = """literal: |
  line 1
  line 2
  line 3

folded: >
  this is
  folded text
"""
    doc = pyamlium_custom.parse(yaml_str)
    result = doc.to_yaml()
    print("=== Multi-line Input ===")
    print(yaml_str)
    print("=== Multi-line Output ===")
    print(result)


def test_roundtrip_complex():
    """Test complex nested structure"""
    yaml_str = """server:
  name: web-server
  port: 8080

database:
  host: localhost
  port: 5432
"""
    doc = pyamlium_custom.parse(yaml_str)
    result = doc.to_yaml()
    print("=== Complex Input ===")
    print(yaml_str)
    print("=== Complex Output ===")
    print(result)
    print("=== Values preserved ===")
    print(f"server.name: {doc.get('server')}")
    print(f"database.host: {doc.get('database')}")


def test_from_file():
    """Test parsing from file"""
    try:
        doc = pyamlium_custom.parse_file("tests/roundtrip.yaml")
        result = doc.to_yaml()
        print("=== File Parse Success ===")
        print(f"Root type: {doc.root_type()}")
        print(f"App name: {doc.get('app')}")
        print(f"Database: {doc.get('database')}")
        print(f"First 500 chars of output:")
        print(result[:500])
    except Exception as e:
        print(f"Error: {e}")


if __name__ == "__main__":
    test_roundtrip_with_comments()
    print("\n" + "="*60 + "\n")
    test_roundtrip_with_anchors()
    print("\n" + "="*60 + "\n")
    test_roundtrip_multiline()
    print("\n" + "="*60 + "\n")
    test_roundtrip_complex()
    print("\n" + "="*60 + "\n")
    test_from_file()
