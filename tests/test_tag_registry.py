import pytest

import pyrs_yaml
from pyrs_yaml import TagHandler, register_tag_handler
from tests.data import yaml_samples as yaml


@pytest.fixture(autouse=True)
def _clear_tag_handlers():
    pyrs_yaml.clear_tag_handlers()
    yield


def _resolve_handler(fn):
    """Wrap a scalar string-transform callable as a TagHandler."""

    class _ResolveHandler(TagHandler):
        def resolve(self, value: str, tag: str) -> str:
            return fn(value)

    return _ResolveHandler()


class TestTagRegistry:
    """Test tag handler registry (Phase 3 of v2.0 — unified TagHandler API)."""

    def test_register_tag_decorator(self):
        @register_tag_handler("!custom")
        class CustomHandler(TagHandler):
            def resolve(self, value, tag):
                return f"custom:{value}"

        doc = pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
        assert doc.get("name") == "custom:value"

    def test_register_tag_imperative(self):
        register_tag_handler("!custom", _resolve_handler(lambda node: node.upper()))
        doc = pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
        assert doc.get("name") == "VALUE"

    def test_tag_handler_error_propagation(self):
        @register_tag_handler("!custom")
        class BrokenHandler(TagHandler):
            def resolve(self, value, tag):
                raise ValueError("handler failed")

        with pytest.raises(pyrs_yaml.YamlTagError):
            pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)

    def test_chain_first_skip_second_handles(self):
        @register_tag_handler("!custom", priority=0)
        class SkipHandler(TagHandler):
            def resolve(self, value, tag):
                raise pyrs_yaml.YamlTagSkip()

        @register_tag_handler("!custom", priority=1)
        class RealHandler(TagHandler):
            def resolve(self, value, tag):
                return f"handled:{value}"

        doc = pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
        assert doc.get("name") == "handled:value"

    def test_chain_all_skip_fallback(self):
        @register_tag_handler("!custom", priority=0)
        class Skip1(TagHandler):
            def resolve(self, value, tag):
                raise pyrs_yaml.YamlTagSkip()

        @register_tag_handler("!custom", priority=1)
        class Skip2(TagHandler):
            def resolve(self, value, tag):
                raise pyrs_yaml.YamlTagSkip()

        doc = pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
        assert doc.get("name") == "value"

    def test_priority_ordering(self):
        @register_tag_handler("!custom", priority=10)
        class LowPriority(TagHandler):
            def resolve(self, value, tag):
                return "low"

        @register_tag_handler("!custom", priority=1)
        class HighPriority(TagHandler):
            def resolve(self, value, tag):
                return "high"

        doc = pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
        assert doc.get("name") == "high"

    def test_unregistered_tag_returns_original(self):
        doc = pyrs_yaml.YAML().parse("key: !unknown value")
        assert doc.get("key") == "value"

    def test_handler_returns_non_string_raises(self):
        @register_tag_handler("!custom")
        class NonStringHandler(TagHandler):
            def resolve(self, value, tag):
                return 123

        with pytest.raises(pyrs_yaml.YamlTagError, match="must return a string"):
            pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)

    def test_nested_container_tag_resolved(self):
        @register_tag_handler("!custom")
        class CustomHandler(TagHandler):
            def resolve(self, value, tag):
                return f"custom:{value}"

        doc = pyrs_yaml.YAML().parse(yaml.NESTED_TAGGED)
        assert doc.get("outer")["inner"] == "custom:value"

    def test_clear_tag_handlers_effective(self):
        @register_tag_handler("!custom")
        class CustomHandler(TagHandler):
            def resolve(self, value, tag):
                return f"custom:{value}"

        pyrs_yaml.clear_tag_handlers()
        doc = pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
        assert doc.get("name") == "value"

    def test_remove_tag_effective(self):
        @register_tag_handler("!custom")
        class CustomHandler(TagHandler):
            def resolve(self, value, tag):
                return f"custom:{value}"

        pyrs_yaml.remove_tag("!custom")
        doc = pyrs_yaml.YAML().parse(yaml.TAG_CUSTOM)
        assert doc.get("name") == "value"

    def test_duplicate_register_first_wins(self):
        @register_tag_handler("!dup")
        class FirstHandler(TagHandler):
            def resolve(self, value, tag):
                return f"first:{value}"

        @register_tag_handler("!dup")
        class SecondHandler(TagHandler):
            def resolve(self, value, tag):
                return f"second:{value}"

        doc = pyrs_yaml.YAML().parse("x: !dup v\n")
        assert doc.get("x") == "first:v"
