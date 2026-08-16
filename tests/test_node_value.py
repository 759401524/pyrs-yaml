"""Tests for Node.value behavior across all node types."""

import pytest

import pyrs_yaml


class TestNodeValueScalar:
    def test_scalar_string_value(self):
        doc = pyrs_yaml.parse("key: hello\n")
        node = doc.node().find("$.key")
        assert node.root_type == "scalar"
        assert node.value == "hello"

    def test_scalar_int_value(self):
        doc = pyrs_yaml.parse("key: 42\n")
        node = doc.node().find("$.key")
        assert node.value == 42

    def test_scalar_float_value(self):
        doc = pyrs_yaml.parse("key: 3.14\n")
        node = doc.node().find("$.key")
        assert node.value == 3.14

    def test_scalar_bool_value(self):
        doc = pyrs_yaml.parse("key: true\n")
        node = doc.node().find("$.key")
        assert node.value is True

    def test_scalar_quoted_string_stays_string(self):
        """Quoted scalars are always strings (YAML 1.2); only plain scalars are resolved."""
        doc = pyrs_yaml.parse('key: "42"\n')
        node = doc.node().find("$.key")
        # Quoted scalars are never schema-resolved, so "42" stays a string.
        assert node.value == "42"


class TestNodeValueNull:
    def test_null_value(self):
        doc = pyrs_yaml.parse("key: null\n")
        node = doc.node().find("$.key")
        assert node.root_type == "null"
        assert node.value is None

    def test_empty_value_is_null(self):
        doc = pyrs_yaml.parse("key:\n")
        node = doc.node().find("$.key")
        assert node.root_type == "null"
        assert node.value is None


class TestNodeValueMapping:
    def test_mapping_value_is_none(self):
        doc = pyrs_yaml.parse("key: value\n")
        node = doc.node().find("$.key")
        assert node.root_type == "scalar"

        root = doc.node()
        assert root.root_type == "mapping"
        assert root.value is None

    def test_nested_mapping_value_is_none(self):
        doc = pyrs_yaml.parse("a:\n  b: 1\n")
        node = doc.node().find("$.a")
        assert node.root_type == "mapping"
        assert node.value is None

    def test_mapping_access_via_children(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        root = doc.node()
        children = root.children
        assert len(children) == 2
        assert children[0].value == 1
        assert children[1].value == 2

    def test_mapping_access_via_get(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        assert doc.get("a") == 1
        assert doc.get("b") == 2

    def test_mapping_to_yaml(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        root = doc.node()
        yaml_out = root.to_yaml()
        assert "a: 1" in yaml_out
        assert "b: 2" in yaml_out


class TestNodeValueSequence:
    def test_sequence_value_is_none(self):
        doc = pyrs_yaml.parse("- 1\n- 2\n")
        root = doc.node()
        assert root.root_type == "sequence"
        assert root.value is None

    def test_sequence_access_via_children(self):
        doc = pyrs_yaml.parse("- a\n- b\n- c\n")
        root = doc.node()
        children = root.children
        assert len(children) == 3
        assert children[0].value == "a"
        assert children[1].value == "b"
        assert children[2].value == "c"

    def test_sequence_access_via_index(self):
        doc = pyrs_yaml.parse("- a\n- b\n- c\n")
        root = doc.node()
        assert root.find("$[0]").value == "a"
        assert root.find("$[1]").value == "b"
        assert root.find("$[-1]").value == "c"

    def test_sequence_to_yaml(self):
        doc = pyrs_yaml.parse("- a\n- b\n")
        root = doc.node()
        yaml_out = root.to_yaml()
        assert "- a" in yaml_out
        assert "- b" in yaml_out


class TestNodeValueMixed:
    def test_nested_mixed_structure(self):
        doc = pyrs_yaml.parse("""\
servers:
  - host: localhost
    port: 8080
  - host: example.com
    port: 443
""")
        root = doc.node()
        assert root.value is None
        assert root.root_type == "mapping"

        servers = root.find("$.servers")
        assert servers.value is None
        assert servers.root_type == "sequence"

        first = servers.find("$[0]")
        assert first.value is None
        assert first.root_type == "mapping"

        host = first.find("$.host")
        assert host.value == "localhost"
        assert host.root_type == "scalar"


class TestNodeValueStaleness:
    def test_stale_node_raises_after_edit(self):
        doc = pyrs_yaml.parse("a: 1\n")
        node = doc.node().find("$.a")
        assert node.value == 1
        doc.set("$.b", 2)
        # Accessing stale node raises YamlDocumentError with RuntimeWarning
        with pytest.warns(RuntimeWarning, match="stale"), pytest.raises(pyrs_yaml.YamlDocumentError):
            _ = node.value

    def test_self_edit_stales_node(self):
        """Self-editing a node DOES stale it (the node itself triggered the revision)."""
        doc = pyrs_yaml.parse("a: 1\n")
        node = doc.node().find("$.a")
        node.set_value(2)
        # After self-edit, the node is stale because revision changed
        with pytest.warns(RuntimeWarning, match="stale"), pytest.raises(pyrs_yaml.YamlDocumentError):
            _ = node.value


class TestNodeCopy:
    def test_copy_scalar(self):
        doc = pyrs_yaml.parse("a: 42\n")
        assert doc.node().find("$.a").copy() == 42

    def test_copy_mapping(self):
        doc = pyrs_yaml.parse("config:\n  host: localhost\n  ports: [80, 443]\n")
        copied = doc.node().find("$.config").copy()
        assert copied == {"host": "localhost", "ports": [80, 443]}
        assert isinstance(copied, dict)

    def test_copy_is_detached(self):
        doc = pyrs_yaml.parse("config:\n  host: localhost\n")
        copied = doc.node().find("$.config").copy()
        copied["host"] = "mutated"
        assert doc.to_yaml() == "config:\n  host: localhost\n"

    def test_copy_paste_back(self):
        doc = pyrs_yaml.parse("src:\n  a: 1\ndst: {}\n")
        src = doc.node().find("$.src").copy()
        doc.node().find("$.dst").set_value(src)
        # dst was declared flow-style ({}); set_value preserves the container's
        # original flow_style, so the pasted value keeps flow formatting.
        assert doc.to_yaml() == "src:\n  a: 1\ndst: {a: 1}\n"

    def test_copy_sequence(self):
        doc = pyrs_yaml.parse("items: [1, 2, 3]\n")
        assert doc.node().find("$.items").copy() == [1, 2, 3]
