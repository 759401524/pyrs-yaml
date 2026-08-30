"""Tests for the unified TagHandler API (Phase 1 — additive).

Covers:
- TagHandler base class (resolve, from_yaml, to_yaml, container hooks)
- register_tag_handler / get_tag_handler / remove_tag_handler
- list_tag_handlers
- Lifecycle hooks (on_parse_start/end, on_serialize_start/end)
- PluginInfo + conflict detection
- Backward compatibility with register_tag / register_type
"""

from __future__ import annotations

from collections import OrderedDict
from typing import Any

import pytest

import pyrs_yaml
from pyrs_yaml import (
    ParseContext,
    PluginInfo,
    SerializeContext,
    TagHandler,
    YamlTagSkip,
    get_tag_handler,
    parse,
    parse_all_docs,
    register_tag_handler,
    remove_tag_handler,
    serialize,
)
from pyrs_yaml._tag_handler import (
    all_stored_handlers,
    clear_stored_handlers,
)
from pyrs_yaml.plugins import list_plugin_info

# ── Helpers ──────────────────────────────────────────────────────────────────


class _Tracker:
    """Collect lifecycle hook calls for assertions."""

    def __init__(self) -> None:
        self.calls: list[str] = []

    def reset(self) -> None:
        self.calls.clear()


# ── TagHandler base class ────────────────────────────────────────────────────


class TestTagHandlerBase:
    def test_default_methods_are_noop(self) -> None:
        h = TagHandler()
        assert h.can_parse("anything") is True
        assert h.from_yaml("value") == "value"
        assert h.to_yaml("obj") == "obj"
        assert h.validate("obj") is True
        assert h.python_type is None

    def test_resolve_raises_skip_by_default(self) -> None:
        h = TagHandler()
        with pytest.raises(YamlTagSkip):
            h.resolve("!tag value", "!tag")

    def test_container_defaults_raise_skip(self) -> None:
        h = TagHandler()
        with pytest.raises(YamlTagSkip):
            h.from_yaml_mapping("!m", {"k": "v"})
        with pytest.raises(YamlTagSkip):
            h.from_yaml_sequence("!s", [1, 2])
        assert h.to_yaml_mapping("obj") is None
        assert h.to_yaml_sequence("obj") is None

    def test_lifecycle_defaults_are_identity(self) -> None:
        h = TagHandler()
        ctx = ParseContext()
        sctx = SerializeContext()
        assert h.on_parse_start("yaml", ctx) == "yaml"
        h.on_parse_end(None, ctx)
        h.on_serialize_start(None, sctx)
        assert h.on_serialize_end("yaml", sctx) == "yaml"


# ── register_tag_handler / get / remove / list ───────────────────────────────


class TestRegisterTagHandler:
    def teardown_method(self) -> None:
        pyrs_yaml.clear_tag_handlers()
        clear_stored_handlers()

    def test_register_imperative(self) -> None:
        h = TagHandler()

        register_tag_handler("!imperative", h)
        assert get_tag_handler("!imperative") is h

    def test_register_decorator(self) -> None:
        @register_tag_handler("!decorated")
        class MyHandler(TagHandler):
            pass

        assert get_tag_handler("!decorated") is not None
        assert isinstance(get_tag_handler("!decorated"), MyHandler)

    def test_register_decorator_instance(self) -> None:
        instance = TagHandler()
        register_tag_handler("!inst2", instance)
        assert get_tag_handler("!inst2") is instance

    def test_remove_tag_handler(self) -> None:
        h = TagHandler()
        register_tag_handler("!rm", h)
        assert get_tag_handler("!rm") is h
        remove_tag_handler("!rm")
        assert get_tag_handler("!rm") is None

    def test_clear_tag_handlers_clears_both_registries(self) -> None:
        h = TagHandler()
        register_tag_handler("!clear_me", h)
        assert get_tag_handler("!clear_me") is h
        pyrs_yaml.clear_tag_handlers()
        assert get_tag_handler("!clear_me") is None
        assert all_stored_handlers() == {}

    def test_list_tag_handlers(self) -> None:
        h = TagHandler()
        register_tag_handler("!list1", h)
        tags = pyrs_yaml.list_tag_handlers()
        tag_names = [t for t, _ in tags]
        assert "!list1" in tag_names

    def test_register_invalid_type_raises(self) -> None:
        with pytest.raises(TypeError, match="TagHandler instance"):
            register_tag_handler("!bad", "not a handler")  # type: ignore[arg-type]


# ── Resolve (scalar tag handler) ─────────────────────────────────────────────


class TestResolve:
    def teardown_method(self) -> None:
        pyrs_yaml.clear_tag_handlers()
        clear_stored_handlers()

    def test_resolve_transforms_value(self) -> None:
        class Upper(TagHandler):
            def resolve(self, value: str, tag: str) -> str:
                return value.upper()

        register_tag_handler("!upper", Upper())
        doc = parse("!upper hello")
        assert doc.to_dict() == "HELLO"

    def test_resolve_chain_skip_fallback(self) -> None:
        class SkipAlways(TagHandler):
            def resolve(self, value: str, tag: str) -> str:
                raise YamlTagSkip

        class TakeIt(TagHandler):
            def resolve(self, value: str, tag: str) -> str:
                return f"taken:{value}"

        register_tag_handler("!chain", SkipAlways(), priority=0)
        register_tag_handler("!chain", TakeIt(), priority=1)
        doc = parse("!chain val")
        assert doc.to_dict() == "taken:val"


# ── from_yaml / to_yaml (scalar type handlers) ──────────────────────────────


class TestScalarTypeHandler:
    def teardown_method(self) -> None:
        pyrs_yaml.clear_tag_handlers()
        clear_stored_handlers()

    def test_from_yaml_called_on_get(self) -> None:
        class UpperStr(TagHandler):
            python_type = str

            def from_yaml(self, value: Any) -> Any:
                if isinstance(value, str):
                    return value.upper()
                return value

        register_tag_handler("!upperstr", UpperStr())
        doc = parse("!upperstr hello")
        assert doc.to_dict() == "HELLO"

    def test_to_yaml_called_on_roundtrip(self) -> None:
        class MyInt(TagHandler):
            python_type = int

            def from_yaml(self, value: Any) -> Any:
                return int(value) * 10

            def to_yaml(self, obj: Any) -> str:
                return str(obj // 10)

        register_tag_handler("!myint", MyInt())
        doc = parse("!myint 5")
        # to_dict triggers from_yaml
        assert doc.to_dict() == 50
        # to_yaml preserves the tag and calls to_yaml
        output = doc.to_yaml()
        assert "!myint" in output


# ── Container handlers ───────────────────────────────────────────────────────


class TestContainerHandlers:
    def teardown_method(self) -> None:
        pyrs_yaml.clear_tag_handlers()
        clear_stored_handlers()

    def test_from_yaml_mapping(self) -> None:
        class OrderedDictHandler(TagHandler):
            def from_yaml_mapping(self, tag: str, data: dict) -> Any:
                from collections import OrderedDict

                return OrderedDict(data)

        register_tag_handler("!ordered", OrderedDictHandler())
        doc = parse("!ordered\n  b: 1\n  a: 2")
        val = doc.to_dict()
        assert isinstance(val, OrderedDict)
        assert list(val.keys()) == ["b", "a"]

    def test_from_yaml_sequence(self) -> None:
        class TupleHandler(TagHandler):
            def from_yaml_sequence(self, tag: str, data: list) -> Any:
                return tuple(data)

        register_tag_handler("!tuple", TupleHandler())
        doc = parse("!tuple [1, 2, 3]")
        val = doc.to_dict()
        assert val == (1, 2, 3)

    def test_to_yaml_mapping_roundtrip(self) -> None:
        from collections import OrderedDict

        class OrderedTag(TagHandler):
            python_type = OrderedDict

            def to_yaml_mapping(self, obj: Any) -> dict | None:
                if isinstance(obj, OrderedDict):
                    return dict(obj)
                return None

        register_tag_handler("!ordered2", OrderedTag())
        od = OrderedDict([("z", 1), ("a", 2)])
        doc = parse("key: value")
        doc.set("$", od)
        output = doc.to_yaml()
        assert "!ordered2" in output


# ── Lifecycle hooks ──────────────────────────────────────────────────────────


class TestLifecycleHooks:
    def teardown_method(self) -> None:
        pyrs_yaml.clear_tag_handlers()
        clear_stored_handlers()

    def test_on_parse_start_end_called(self) -> None:
        tracker = _Tracker()

        class HookHandler(TagHandler):
            def on_parse_start(self, yaml_str: str, context: ParseContext) -> str:
                tracker.calls.append("parse_start")
                return yaml_str

            def on_parse_end(self, document: Any, context: ParseContext) -> None:
                tracker.calls.append("parse_end")

        register_tag_handler("!hook", HookHandler())
        parse("!hook test")
        assert "parse_start" in tracker.calls
        assert "parse_end" in tracker.calls

    def test_on_serialize_start_end_called(self) -> None:
        tracker = _Tracker()

        class HookHandler(TagHandler):
            def on_serialize_start(self, document: Any, context: SerializeContext) -> None:
                tracker.calls.append("ser_start")

            def on_serialize_end(self, yaml_str: str, context: SerializeContext) -> str:
                tracker.calls.append("ser_end")
                return yaml_str

        register_tag_handler("!hook2", HookHandler())
        doc = parse("key: value")
        serialize(doc)
        assert "ser_start" in tracker.calls
        assert "ser_end" in tracker.calls

    def test_parse_all_docs_calls_hooks_per_doc(self) -> None:
        tracker = _Tracker()

        class CountHandler(TagHandler):
            def on_parse_end(self, document: Any, context: ParseContext) -> None:
                tracker.calls.append("doc")

        register_tag_handler("!count", CountHandler())
        parse_all_docs("---\na: 1\n---\nb: 2")
        assert tracker.calls.count("doc") == 2

    def test_hook_exception_does_not_propagate(self) -> None:
        class BadHook(TagHandler):
            def on_parse_start(self, yaml_str: str, context: ParseContext) -> str:
                raise RuntimeError("boom")

        register_tag_handler("!badhook", BadHook())
        # Should not raise — hooks are best-effort
        doc = parse("!badhook value")
        assert doc.to_dict() == "value"


# ── Context objects ──────────────────────────────────────────────────────────


class TestContextObjects:
    def test_parse_context_defaults(self) -> None:
        ctx = ParseContext()
        assert ctx.schema == "core"
        assert ctx.max_depth == 1000
        assert ctx.resolve_merges is True
        assert ctx.source is None

    def test_serialize_context_defaults(self) -> None:
        ctx = SerializeContext()
        assert ctx.indent == 2
        assert ctx.sort_keys is False

    def test_parse_context_frozen(self) -> None:
        ctx = ParseContext()
        with pytest.raises(AttributeError):
            ctx.schema = "json"  # type: ignore[misc]


# ── PluginInfo + conflict detection ──────────────────────────────────────────


class TestPluginInfo:
    def teardown_method(self) -> None:
        pyrs_yaml.clear_tag_handlers()
        clear_stored_handlers()

    def test_plugin_info_dataclass(self) -> None:
        info = PluginInfo(
            name="test-plugin",
            version="1.0.0",
            tags=["!test"],
            conflicts=["!other"],
        )
        assert info.name == "test-plugin"
        assert info.version == "1.0.0"
        assert "!test" in info.tags
        assert "!other" in info.conflicts

    def test_list_plugin_info_empty(self) -> None:
        result = list_plugin_info()
        assert isinstance(result, list)


# ── Backward compatibility ──────────────────────────────────────────────────


class TestBackwardCompat:
    def teardown_method(self) -> None:
        pyrs_yaml.clear_tag_handlers()
        clear_stored_handlers()

    def test_register_tag_still_works(self) -> None:
        """register_tag (old API) should still function."""
        pyrs_yaml.register_tag("!oldapi", lambda v: f"wrapped:{v}")
        doc = parse("!oldapi test")
        assert doc.to_dict() == "wrapped:test"

    def test_register_type_still_works(self) -> None:
        """register_type (old API) should still function."""
        from pyrs_yaml import CustomType, register_type

        class MyCustom(CustomType):
            python_type = float

            def from_yaml(self, value: Any) -> Any:
                return float(value) * 2

        register_type("!doubled", MyCustom())
        doc = parse("!doubled 21")
        assert doc.to_dict() == 42.0
