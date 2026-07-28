import pyrs_yaml


def test_special_chars_colon():
    yaml_str = 'key: "value:with:colons"'
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.get("key") == "value:with:colons"


def test_special_chars_hash():
    yaml_str = 'key: "value#with#hash"'
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.get("key") == "value#with#hash"


def test_special_chars_dash():
    yaml_str = 'key: "-value"'
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.get("key") == "-value"


def test_special_chars_brackets():
    yaml_str = 'key: "value[0]"'
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.get("key") == "value[0]"


def test_special_chars_curly():
    yaml_str = 'key: "value{a: 1}"'
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.get("key") == "value{a: 1}"


def test_literal_block_scalar():
    yaml_str = "key: |\n  line1\n  line2"
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.root_type() == "mapping"


def test_folded_block_scalar():
    yaml_str = "key: >\n  this is\n  folded"
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.root_type() == "mapping"


def test_anchor():
    yaml_str = "anchor: &a value\nalias: *a"
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.root_type() == "mapping"


def test_alias():
    yaml_str = "defaults: &defaults\n  timeout: 30\ndevelopment:\n  <<: *defaults"
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.root_type() == "mapping"


def test_nested_sequence():
    yaml_str = "- - nested1\n  - nested2\n- top"
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.root_type() == "sequence"


def test_empty_mapping():
    yaml_str = "{}"
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.root_type() == "mapping"


def test_empty_sequence():
    yaml_str = "[]"
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.root_type() == "sequence"


def test_single_quoted_scalar():
    yaml_str = "key: 'single'"
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.get("key") == "single"


def test_double_quoted_scalar():
    yaml_str = 'key: "double"'
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.get("key") == "double"


def test_plain_scalar_no_quotes():
    yaml_str = "key: plain"
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.get("key") == "plain"


def test_multiple_documents():
    yaml_str = "---\nfirst: 1\n---\nsecond: 2"
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.root_type() == "mapping"


def test_complex_nested():
    yaml_str = """
server:
  host: localhost
  port: 8080
database:
  driver: postgresql
  host: db.example.com
  port: 5432
"""
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.get("server") is not None
    assert doc.get("database") is not None
