import pyamlium_custom


def test_yaml12_booleans():
    """Test YAML 1.2 boolean types"""
    yaml_str = '''true_val: true
True_val: True
TRUE_val: TRUE
false_val: false
False_val: False
FALSE_val: FALSE
'''
    doc = pyamlium_custom.parse(yaml_str)
    assert doc.get("true_val") is True
    assert doc.get("True_val") is True
    assert doc.get("TRUE_val") is True
    assert doc.get("false_val") is False
    assert doc.get("False_val") is False
    assert doc.get("FALSE_val") is False


def test_yaml12_null():
    """Test YAML 1.2 null types"""
    yaml_str = '''null_val: null
Null_val: Null
NULL_val: NULL
tilde_val: ~
empty_val:
'''
    doc = pyamlium_custom.parse(yaml_str)
    assert doc.get("null_val") is None
    assert doc.get("Null_val") is None
    assert doc.get("NULL_val") is None
    assert doc.get("tilde_val") is None
    assert doc.get("empty_val") is None


def test_yaml12_integers():
    """Test YAML 1.2 integer types"""
    yaml_str = '''decimal: 42
negative: -17
octal: 0o14
hex: 0x0C
'''
    doc = pyamlium_custom.parse(yaml_str)
    assert doc.get("decimal") == 42
    assert doc.get("negative") == -17
    assert doc.get("octal") == 12  # 0o14 = 12
    assert doc.get("hex") == 12  # 0x0C = 12


def test_yaml12_floats():
    """Test YAML 1.2 float types"""
    yaml_str = '''basic: 3.14
negative: -0.5
scientific: 6.022e23
negative_sci: -1.0e-3
'''
    doc = pyamlium_custom.parse(yaml_str)
    assert abs(doc.get("basic") - 3.14) < 1e-10
    assert abs(doc.get("negative") - (-0.5)) < 1e-10
    assert abs(doc.get("scientific") - 6.022e23) < 1e10
    assert abs(doc.get("negative_sci") - (-1.0e-3)) < 1e-10


def test_yaml12_infinity():
    """Test YAML 1.2 infinity"""
    yaml_str = '''inf: .inf
neg_inf: -.inf
'''
    doc = pyamlium_custom.parse(yaml_str)
    import math
    assert math.isinf(doc.get("inf")) and doc.get("inf") > 0
    assert math.isinf(doc.get("neg_inf")) and doc.get("neg_inf") < 0


def test_yaml12_nan():
    """Test YAML 1.2 NaN"""
    yaml_str = '''nan: .nan
'''
    doc = pyamlium_custom.parse(yaml_str)
    import math
    assert math.isnan(doc.get("nan"))


def test_yaml12_roundtrip():
    """Test round-trip for YAML 1.2 types"""
    yaml_str = '''boolean: true
integer: 42
float: 3.14
null_val: null
'''
    doc = pyamlium_custom.parse(yaml_str)
    output = doc.to_yaml()
    # Verify the types are preserved in output
    assert "true" in output
    assert "42" in output
    assert "3.14" in output
    assert "null" in output
