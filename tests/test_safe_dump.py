"""Smoke tests for safe_dump and from_dict (identical implementations)."""

import pyrs_yaml


class TestSafeDump:
    def test_dump_simple_dict(self):
        result = pyrs_yaml.safe_dump({"key": "value"})
        assert "key: value" in result

    def test_dump_nested_dict(self):
        result = pyrs_yaml.safe_dump({"a": {"b": 1}})
        assert "a:" in result
        assert "b: 1" in result

    def test_dump_list(self):
        result = pyrs_yaml.safe_dump([1, 2, 3])
        assert "- 1" in result
        assert "- 2" in result
        assert "- 3" in result

    def test_dump_mixed(self):
        result = pyrs_yaml.safe_dump({"items": [1, "two", True]})
        assert "items:" in result
        assert "- 1" in result
        assert "- two" in result
        assert "- true" in result

    def test_empty_containers_are_inlined_under_a_dash(self):
        # `{}` and `[]` have no block spelling: written on a line of their own below a
        # `- ` they re-read as flow nodes, so the next dump inlined them and the text
        # kept moving between rounds. The dash line is where it stops.
        assert pyrs_yaml.safe_dump([{}]) == "- {}\n"
        assert pyrs_yaml.safe_dump([[]]) == "- []\n"
        assert pyrs_yaml.safe_dump([{}, []]) == "- {}\n- []\n"
        for value in ([{}], [[]], [{"a": [{}]}], [[[]]], [{"x": {}}]):
            one = pyrs_yaml.safe_dump(value)
            assert one == pyrs_yaml.safe_dump(pyrs_yaml.safe_load(one)), one

    def test_dump_empty_dict(self):
        result = pyrs_yaml.safe_dump({})
        # Empty dict emits the explicit flow form so it re-parses as {} (not null).
        assert result.strip() == "{}"

    def test_dump_empty_list(self):
        result = pyrs_yaml.safe_dump([])
        # Empty list emits the explicit flow form so it re-parses as [] (not null).
        assert result.strip() == "[]"

    def test_dump_string_with_special_chars(self):
        result = pyrs_yaml.safe_dump({"key": "hello: world"})
        assert "key:" in result

    def test_dump_none_value(self):
        result = pyrs_yaml.safe_dump({"key": None})
        assert "key: null" in result

    def test_dump_bool_values(self):
        result = pyrs_yaml.safe_dump({"a": True, "b": False})
        assert "a: true" in result
        assert "b: false" in result

    def test_dump_numeric_values(self):
        result = pyrs_yaml.safe_dump({"i": 42, "f": 3.14})
        assert "i: 42" in result
        assert "f: 3.14" in result


class TestFromDict:
    def test_from_dict_same_as_safe_dump(self):
        data = {"key": "value", "num": 42}
        assert pyrs_yaml.from_dict(data) == pyrs_yaml.safe_dump(data)

    def test_from_dict_nested(self):
        data = {"a": {"b": [1, 2]}}
        result = pyrs_yaml.from_dict(data)
        assert "a:" in result
        assert "b:" in result
        assert "- 1" in result
        assert "- 2" in result

    def test_from_dict_empty(self):
        assert pyrs_yaml.from_dict({}) == pyrs_yaml.safe_dump({})
        assert pyrs_yaml.from_dict([]) == pyrs_yaml.safe_dump([])


class TestRoundTrip:
    def test_dump_then_load(self):
        data = {"name": "test", "values": [1, 2, 3]}
        yaml_str = pyrs_yaml.safe_dump(data)
        loaded = pyrs_yaml.safe_load(yaml_str)
        assert loaded == data

    def test_from_dict_then_load(self):
        data = {"a": 1, "b": 2}
        yaml_str = pyrs_yaml.from_dict(data)
        loaded = pyrs_yaml.safe_load(yaml_str)
        assert loaded == data


class TestSequenceModes:
    """direct_dump write_sequence branch coverage (audit §15)."""

    def test_compact_mapping_in_sequence(self):
        # write_sequence is_compact_mapping=true (direct_dump path via safe_dump).
        out = pyrs_yaml.safe_dump([{"a": 1}])
        assert out.strip() == "- a: 1"
        assert pyrs_yaml.safe_load(out) == [{"a": 1}]

    def test_block_mapping_in_sequence(self):
        out = pyrs_yaml.safe_dump([{"a": 1, "b": 2}])
        assert pyrs_yaml.safe_load(out) == [{"a": 1, "b": 2}]

    def test_nested_list_in_sequence(self):
        # write_sequence nested-list branch.
        out = pyrs_yaml.safe_dump([[1, 2], [3, 4]])
        assert pyrs_yaml.safe_load(out) == [[1, 2], [3, 4]]


class TestFloatSpecialValues:
    """safe_dump of float('inf') / float('nan') (non-numpy path)."""

    def test_inf(self):
        out = pyrs_yaml.safe_dump(float("inf"))
        assert pyrs_yaml.safe_load(out) == float("inf")

    def test_neg_inf(self):
        out = pyrs_yaml.safe_dump(float("-inf"))
        assert pyrs_yaml.safe_load(out) == float("-inf")

    def test_nan(self):
        out = pyrs_yaml.safe_dump(float("nan"))
        # NaN differs from itself; just confirm it round-trips as a float.
        assert isinstance(pyrs_yaml.safe_load(out), float)


class TestFuzzRegressions:
    """Minimal repros from scripts/fuzz_panics.py (v0.14.0 release cycle).

    Each case produced invalid YAML or a panic before the fix; safe_dump output
    must always re-parse and round-trip.
    """

    def _rt(self, value):
        return pyrs_yaml.safe_load(pyrs_yaml.safe_dump(value))

    def test_control_char_plus_backslash_key(self):
        # U+001F (control) + backslash: single-quoted emission is invalid YAML
        # because single quotes cannot escape control characters. Must use
        # double-quoting.
        v = {"\x1f\\": ""}
        assert self._rt(v) == v

    def test_noncharacter_key_bmp(self):
        # U+FFFE / U+FFFF are non-characters granit rejects as plain scalars.
        for cp in (0xFFFE, 0xFFFF):
            v = {chr(cp): None}
            assert self._rt(v) == v

    def test_noncharacter_key_supplementary(self):
        # Plane-end non-characters above U+FFFF (e.g. U+1FFFE, U+DFFFE).
        for cp in (0x1FFFE, 0x1FFFF, 0xDFFFE, 0x10FFFE, 0x10FFFF):
            v = {chr(cp): None}
            assert self._rt(v) == v

    def test_bom_key(self):
        # U+FEFF (BOM) as a plain key is dropped by granit; must be quoted.
        v = {"\ufeff": None}
        assert self._rt(v) == v

    def test_wrap_continuation_in_nested_sequence(self):
        # A long plain scalar inside a nested sequence item must wrap with a
        # continuation indent greater than the parent block indent, otherwise
        # granit reports "simple key expected ':'".
        v = [["0000000 " + "\u00a1" * 18 + "\u0800" * 3 + "\U00010000" * 7]]
        assert self._rt(v) == v

    def test_wrap_continuation_multibyte_no_panic(self):
        # 4-byte char straddling the wrap boundary must not panic (char split).
        v = {"k": "x " + "y" * 75 + chr(0x10A09B)}
        assert self._rt(v) == v


class TestIntegersWiderThanI64:
    """A Python int outside i64 keeps its decimal digits on the way out.

    `extract::<f64>()` succeeds for *every* `PyLong`, so the fallback meant for
    protocol-convertible types captured these numbers first and spelled them as floats:
    `123456789012345678901234567890` left the writer as `1.2345678901234568e+29` and
    re-read as a value the caller never passed. YAML puts no width limit on integers, and the
    JSON family writers already emit hub digits verbatim, so the loss belonged to the dumper.

    The hub's *reader* still caps integers at i64 (a documented boundary pinned by
    `test_big_digit_string_spelling_is_verbatim`), so these tests assert the emitted text and
    what an external reader makes of it - not that `safe_load` hands back an `int`.
    """

    HUGE = 123456789012345678901234567890

    def test_dump_writes_digits_not_a_float_spelling(self):
        assert pyrs_yaml.safe_dump({"k": self.HUGE}) == f"k: {self.HUGE}\n"

    def test_dump_writes_a_negative_below_i64_min(self):
        low = -(2**63) - 1
        assert pyrs_yaml.safe_dump({"k": low}) == f"k: {low}\n"

    def test_the_boundary_is_width_not_decimal_length(self):
        # `2**63 - 1` fits and `2**63` does not; both must come out as digits, so the branch
        # keys on the i64 range rather than on "a number with many digits".
        text = pyrs_yaml.safe_dump({"fits": 2**63 - 1, "over": 2**63})
        assert text == "fits: 9223372036854775807\nover: 9223372036854775808\n"

    def test_the_json_family_writers_keep_the_value(self):
        import json

        doc = pyrs_yaml.parse(pyrs_yaml.safe_dump({"k": self.HUGE}))
        for text in (doc.to_json(), doc.to_jsonc(), doc.to_json5()):
            assert json.loads(text) == {"k": self.HUGE}, text

    def test_a_big_int_as_a_key_keeps_its_digits(self):
        # Keys go through the same extractor, and a key silently re-typed to float would
        # collide with a different key on re-read.
        assert pyrs_yaml.safe_dump({self.HUGE: 1}) == f"{self.HUGE}: 1\n"
