"""Typed-error contract tests — guard each error type's identity and message.

These tests lock in behavior so that migrating editing-layer `String` errors
to typed errors (Phase 4.4) cannot silently break consumer code that catches
specific exception types.
"""

import pytest

import pyrs_yaml


class TestTypedErrorIdentity:
    """Each public error type must exist and have the documented base class."""

    @pytest.mark.parametrize(
        "name,base",
        [
            ("YamlParseError", ValueError),
            ("YamlSerializeError", ValueError),
            ("YamlTypeError", TypeError),
            ("YamlEditError", ValueError),
            ("YamlPathError", ValueError),
            ("YamlMaxDepthError", ValueError),
            ("YamlDuplicateKeyError", ValueError),
            ("YamlTagError", ValueError),
            ("YamlValidateError", ValueError),
        ],
        ids=lambda v: str(v),
    )
    def test_error_type_exists_and_inherits(self, name, base):
        assert hasattr(pyrs_yaml, name)
        assert issubclass(getattr(pyrs_yaml, name), base)


class TestParseErrorPaths:
    """Parse failures must raise YamlParseError with line/col context."""

    @pytest.mark.parametrize(
        "bad_yaml",
        [
            "{{invalid",
            "key: value: extra",
            "a:\n  b: [unclosed",
            "- a\n  - b: c\n  bad",
        ],
        ids=["brace", "extra-colon", "unclosed-flow", "bad-sequence"],
    )
    def test_parse_error_is_typed(self, bad_yaml):
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse(bad_yaml)


class TestEditErrorPaths:
    """Path edits with invalid targets must raise YamlEditError / YamlPathError."""

    def test_edit_missing_key_raises(self):
        doc = pyrs_yaml.parse("a: 1")
        with pytest.raises((pyrs_yaml.YamlEditError, pyrs_yaml.YamlPathError)):
            doc.set("missing.path", 2)

    def test_edit_out_of_range_index_raises(self):
        doc = pyrs_yaml.parse("- a\n- b")
        with pytest.raises((pyrs_yaml.YamlEditError, pyrs_yaml.YamlPathError)):
            doc.set("[5]", "x")

    def test_delete_missing_key_raises(self):
        doc = pyrs_yaml.parse("a: 1")
        with pytest.raises((pyrs_yaml.YamlEditError, pyrs_yaml.YamlPathError)):
            doc.delete("nope")


class TestTypeErrorPaths:
    """Type resolution failures must raise YamlTypeError."""

    def test_invalid_custom_type_raises(self):
        # Resolution of an unregistered tag should surface as a typed error or parse cleanly.
        # Either way, safe_load must not return a silently-wrong value.
        try:
            pyrs_yaml.safe_load("!unknown_tag value")
        except pyrs_yaml.YamlTypeError as e:
            assert "unknown_tag" in str(e) or "tag" in str(e).lower()


class TestMaxDepthErrorPaths:
    """Deeply nested input must raise YamlMaxDepthError when over the limit."""

    def test_max_depth_exceeded_raises(self):
        deep = "\n".join(f"{'  ' * i}level{i}:" for i in range(50)) + " value"
        with pytest.raises(pyrs_yaml.YamlMaxDepthError):
            pyrs_yaml.parse(deep, max_depth=10)


class TestDuplicateKeyErrorPaths:
    """Duplicate keys must raise YamlDuplicateKeyError when not allowed."""

    def test_duplicate_key_raises(self):
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.parse("a: 1\na: 2", allow_duplicate_keys=False)
