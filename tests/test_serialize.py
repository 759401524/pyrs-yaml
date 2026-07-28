"""
YAML serialization and PyYAML-compatible API tests.
"""

import pyrs_yaml

# ============================================================================
# Serialization
# ============================================================================


class TestSerialization:
    """Test YAML serialization"""

    def test_serialize_scalar(self):
        doc = pyrs_yaml.parse("key: value")
        output = doc.to_yaml()
        assert "key: value" in output

    def test_serialize_mapping(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2")
        output = doc.to_yaml()
        assert "a: 1" in output
        assert "b: 2" in output

    def test_serialize_sequence(self):
        doc = pyrs_yaml.parse("- a\n- b")
        output = doc.to_yaml()
        assert "- a" in output
        assert "- b" in output


# ============================================================================
# PyYAML Compatible API
# ============================================================================


class TestPyyamlCompatible:
    """Test pyyaml-compatible API"""

    def test_safe_load(self):
        data = pyrs_yaml.safe_load("key: value")
        assert data == {"key": "value"}

    def test_safe_load_types(self):
        data = pyrs_yaml.safe_load("n: 42\nf: 3.14\nb: true\ns: hello")
        assert data["n"] == 42
        assert abs(data["f"] - 3.14) < 1e-10
        assert data["b"] is True
        assert data["s"] == "hello"

    def test_safe_loads(self):
        docs = pyrs_yaml.safe_loads("a: 1\n---\nb: 2")
        assert len(docs) == 2

    def test_safe_dump(self):
        data = {"key": "value", "num": 42}
        output = pyrs_yaml.safe_dump(data)
        assert "key: value" in output
        assert "42" in output

    def test_safe_dumps(self):
        data = {"key": "value"}
        output = pyrs_yaml.safe_dump(data)
        assert "key: value" in output
