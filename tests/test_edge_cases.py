import pytest

import pyrs_yaml


@pytest.mark.parametrize(
    "yaml,expected",
    [
        ('key: "value:with:colons"', "value:with:colons"),
        ('key: "value#with#hash"', "value#with#hash"),
        ('key: "-value"', "-value"),
        ('key: "value[0]"', "value[0]"),
        ('key: "value{a: 1}"', "value{a: 1}"),
    ],
    ids=["colon", "hash", "dash", "brackets", "curly"],
)
def test_special_chars(yaml, expected):
    assert pyrs_yaml.parse(yaml).get("key") == expected


@pytest.mark.parametrize(
    "yaml,expected",
    [
        ("key: 'single'", "single"),
        ('key: "double"', "double"),
        ("key: plain", "plain"),
    ],
    ids=["single-quoted", "double-quoted", "plain"],
)
def test_quoted_scalars(yaml, expected):
    assert pyrs_yaml.parse(yaml).get("key") == expected


@pytest.mark.parametrize(
    "yaml,expected_type",
    [
        ("key: |\n  line1\n  line2", "mapping"),
        ("key: >\n  this is\n  folded", "mapping"),
    ],
    ids=["literal-block", "folded-block"],
)
def test_block_scalars(yaml, expected_type):
    assert pyrs_yaml.parse(yaml).root_type() == expected_type


@pytest.mark.parametrize(
    "yaml,expected_type",
    [
        ("{}", "mapping"),
        ("[]", "sequence"),
    ],
    ids=["empty-mapping", "empty-sequence"],
)
def test_empty_collections(yaml, expected_type):
    assert pyrs_yaml.parse(yaml).root_type() == expected_type


@pytest.mark.parametrize(
    "yaml,expected_type",
    [
        ("anchor: &a value\nalias: *a", "mapping"),
        ("defaults: &defaults\n  timeout: 30\ndevelopment:\n  <<: *defaults", "mapping"),
    ],
    ids=["anchor-alias", "merge-keys"],
)
def test_anchor_and_alias(yaml, expected_type):
    assert pyrs_yaml.parse(yaml).root_type() == expected_type


def test_nested_sequence():
    yaml_str = "- - nested1\n  - nested2\n- top"
    doc = pyrs_yaml.parse(yaml_str)
    assert doc.root_type() == "sequence"


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
