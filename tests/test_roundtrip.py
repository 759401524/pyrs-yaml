import pyyaml_rs


def test_roundtrip_with_comments():
    """Test that comments are preserved in round-trip"""
    yaml_str = """# Top comment
key: value  # Inline comment
"""
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "# Top comment" in result
    assert "key: value" in result


def test_roundtrip_with_anchors():
    """Test that anchors and aliases work"""
    yaml_str = """defaults: &defaults
  timeout: 30
  retries: 3

production:
  <<: *defaults
  host: prod.example.com
"""
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "defaults" in result
    assert "timeout" in result
    assert "retries" in result
    assert "prod.example.com" in result


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
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "literal" in result
    assert "line 1" in result
    assert "folded" in result


def test_roundtrip_complex():
    """Test complex nested structure"""
    yaml_str = """server:
  name: web-server
  port: 8080

database:
  host: localhost
  port: 5432
"""
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "server" in result
    assert "web-server" in result
    assert "database" in result
    assert "localhost" in result
    server = doc.get("server")
    database = doc.get("database")
    assert server is not None
    assert database is not None


def test_from_file():
    """Test parsing from file"""
    doc = pyyaml_rs.parse_file("tests/roundtrip.yaml")
    result = doc.to_yaml()
    assert result is not None
    assert len(result) > 0
    assert doc.root_type() is not None


if __name__ == "__main__":
    test_roundtrip_with_comments()
    test_roundtrip_with_anchors()
    test_roundtrip_multiline()
    test_roundtrip_complex()
    test_from_file()
