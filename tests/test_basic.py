import pyyaml_rs


def test_parse_simple_scalar():
    doc = pyyaml_rs.parse("hello")
    assert doc.to_yaml() == "hello\n"
    assert doc.root_type() == "scalar"


def test_parse_mapping():
    yaml_str = "key: value"
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.root_type() == "mapping"
    assert doc.to_yaml() == "key: value\n"


def test_parse_sequence():
    yaml_str = "- item1\n- item2"
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.root_type() == "sequence"


def test_roundtrip_preserves_quotes():
    yaml_str = "key: 'single quoted'\n"
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "single quoted" in result


def test_roundtrip_preserves_double_quotes():
    yaml_str = 'key: "double quoted"\n'
    doc = pyyaml_rs.parse(yaml_str)
    result = doc.to_yaml()
    assert "double quoted" in result


def test_get_mapping_value():
    yaml_str = "name: Alice\nage: 30"
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("name") == "Alice"
    assert doc.get("age") == 30


def test_get_nonexistent_key():
    yaml_str = "name: Alice"
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("missing") is None


def test_parse_nested_mapping():
    yaml_str = "outer:\n  inner: value"
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.root_type() == "mapping"


def test_parse_null_value():
    yaml_str = "key: null"
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("key") is None


def test_parse_boolean_values():
    yaml_str = "flag: true"
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("flag") is True


def test_parse_integer_value():
    yaml_str = "count: 42"
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("count") == 42


def test_parse_float_value():
    yaml_str = "pi: 3.14"
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("pi") == 3.14


def test_parse_empty_value():
    yaml_str = "key:"
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("key") is None
