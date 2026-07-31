import pyrs_yaml
import pytest

from tests.data import yaml_samples as yaml


@pytest.fixture(autouse=True)
def _clear_tag_handlers():
    pyrs_yaml.clear_tag_handlers()
    yield


class TestTagRegistry:
    """Test tag handler registry (Phase 3 of v0.9.0)."""

    def test_register_tag_decorator(self):
        @pyrs_yaml.register_tag("!custom")
        def custom_handler(node):
            return f"custom:{node}"

        doc = pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
        assert doc.get("name") == "custom:value"

    def test_register_tag_imperative(self):
        def uppercase_handler(node):
            return node.upper()

        pyrs_yaml.register_tag("!custom", uppercase_handler)
        doc = pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
        assert doc.get("name") == "VALUE"

    def test_tag_handler_error_propagation(self):
        @pyrs_yaml.register_tag("!custom")
        def broken_handler(node):
            raise ValueError("handler failed")

        with pytest.raises(pyrs_yaml.YamlTagError):
            pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
