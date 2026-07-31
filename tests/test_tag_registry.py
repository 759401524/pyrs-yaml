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

    def test_chain_first_skip_second_handles(self):
        @pyrs_yaml.register_tag("!custom", priority=0)
        def skip_handler(node):
            raise pyrs_yaml.YamlTagSkip()

        @pyrs_yaml.register_tag("!custom", priority=1)
        def real_handler(node):
            return f"handled:{node}"

        doc = pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
        assert doc.get("name") == "handled:value"

    def test_chain_all_skip_fallback(self):
        @pyrs_yaml.register_tag("!custom", priority=0)
        def skip1(node):
            raise pyrs_yaml.YamlTagSkip()

        @pyrs_yaml.register_tag("!custom", priority=1)
        def skip2(node):
            raise pyrs_yaml.YamlTagSkip()

        doc = pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
        assert doc.get("name") == "value"

    def test_priority_ordering(self):
        @pyrs_yaml.register_tag("!custom", priority=10)
        def low_priority(node):
            return "low"

        @pyrs_yaml.register_tag("!custom", priority=1)
        def high_priority(node):
            return "high"

        doc = pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
        assert doc.get("name") == "high"
