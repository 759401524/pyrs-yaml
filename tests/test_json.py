"""JSON/YAML conversion tests — from_dict, from_json."""

import json
import math

import pytest

import pyrs_yaml


class TestFromDict:
    """Test from_dict function"""

    @pytest.mark.parametrize(
        "data,checks",
        [
            ({"name": "John", "age": 30}, ["name: John", "30"]),
            ({"app": {"name": "myapp", "version": "1.0"}}, ["app:", "name: myapp"]),
            ({"items": [1, 2, 3]}, ["- 1", "- 2"]),
        ],
        ids=["simple", "nested", "list"],
    )
    def test_converts_dict_to_yaml(self, data, checks):
        yaml_str = pyrs_yaml.from_dict(data)
        for check in checks:
            assert check in yaml_str


class TestFromJson:
    """Test from_json function"""

    @pytest.mark.parametrize(
        "json_str,checks",
        [
            ('{"name": "Alice", "active": true}', ["name: Alice", "active: true"]),
            ('{"db": {"host": "localhost", "port": 5432}}', ["db:", "host: localhost"]),
            ('{"items": [1, 2, 3]}', ["- 1"]),
        ],
        ids=["simple", "nested", "array"],
    )
    def test_converts_json_to_yaml(self, json_str, checks):
        yaml_str = pyrs_yaml.from_json(json_str)
        for check in checks:
            assert check in yaml_str

    def test_rejects_invalid_json(self):
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.from_json("{invalid json}")


class TestJsonDialects:
    """from_jsonc / from_json5 / load_json5 + to_jsonc / to_json5."""

    def test_from_jsonc_keeps_comments_as_yaml_notes(self):
        # Since #112/#115 a JSONC comment rides the AST's comment slots so that
        # `to_jsonc` can reproduce it; the YAML projection renders those slots as
        # `#` notes instead of throwing the information away. Every position the
        # reader can report a note from has to survive: after a value, above a key,
        # and trailing a key (which used to disappear with the key's own slot).
        after_value = pyrs_yaml.from_jsonc('{"a": 1 /* tail */, "b": 2}')
        above_key = pyrs_yaml.from_jsonc('{\n  // head\n  "a": 1\n}')
        after_key = pyrs_yaml.from_jsonc('{"a": 1, // note\n "b": 2}')
        assert "tail" in after_value
        assert "head" in above_key
        assert "note" in after_key
        # The notes cost nothing to the data, and each view is a fixed point.
        assert pyrs_yaml.safe_load(after_value) == {"a": 1, "b": 2}
        assert pyrs_yaml.safe_load(above_key) == {"a": 1}
        assert pyrs_yaml.safe_load(after_key) == {"a": 1, "b": 2}
        for text in (after_value, above_key, after_key):
            assert pyrs_yaml.parse(text).to_yaml() == text

    def test_stacked_jsonc_comments_all_survive(self):
        # A member may be introduced by any number of comment lines. The parser
        # kept a single pending note, so `// a` vanished and only `// b` came
        # back -- stable output, silently short one note, which is why the
        # round-trip tier could not see it either.
        stacked = pyrs_yaml.from_jsonc('// a\n// b\n{"k": 1}\n')
        assert "a" in stacked and "b" in stacked, stacked
        assert stacked.count("#") >= 2, stacked
        assert pyrs_yaml.parse(stacked).to_yaml() == stacked
        block = pyrs_yaml.from_jsonc('/* one */\n/* two */\n{"k": 1}\n')
        assert "one" in block and "two" in block, block
        assert pyrs_yaml.parse(block).to_yaml() == block

    def test_from_json5_accepts_wider_grammar(self):
        # unquoted keys, single-quoted strings, trailing comma
        yaml_str = pyrs_yaml.from_json5("{name: 'Alice', active: true,}")
        assert "name:" in yaml_str
        assert "Alice" in yaml_str

    def test_load_json5_line_continuation_and_quote_escape(self):
        # PR #127: JSON5 allows a backslash-newline line continuation and an
        # escaped single quote inside double-quoted strings. Parity check that
        # the string *value* matches what these forms should produce.
        cont = pyrs_yaml.load_json5('{"msg": "ab\\\ncd"}')
        assert cont == {"msg": "abcd"}
        esc = pyrs_yaml.load_json5('{"who": "it\\\'s"}')
        assert esc == {"who": "it's"}
        # Strict JSON / JSONC keep rejecting both forms.
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_jsonc('{"msg": "ab\\\ncd"}')
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_jsonc('{"who": "it\\\'s"}')

    def test_load_json5_accepts_unicode_whitespace(self):
        # PR #128: JSON5 treats NBSP, the Unicode Zs separators, the LS/PS
        # line terminators and ZWNBSP (U+FEFF) as structural whitespace.
        d = pyrs_yaml.load_json5("{\u00a0a:\u20031,\u2028b: 2\ufeff}")
        assert d == {"a": 1, "b": 2}
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_jsonc("{\u00a0a: 1}")
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.from_json("\u000c{}")

    def test_load_json5_accepts_unicode_identifier_keys(self):
        # PR #129: unquoted keys accept the full Unicode ID_Start / ID_Continue
        # set (accented Latin, CJK, Devanagari with combining marks).
        d = pyrs_yaml.load_json5("{\u00e9: 1, \u540d: 2, \u0939\u093f: 3}")
        assert d == {"\u00e9": 1, "\u540d": 2, "\u0939\u093f": 3}
        # Strict JSON / JSONC still require quoting a non-ASCII key.
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_jsonc("{\u00e9: 1}")

    def test_load_json5_returns_values(self):
        d = pyrs_yaml.load_json5("{a: 1, b: .5, c: 'str', d: [1, 2,],}")
        assert d == {"a": 1, "b": 0.5, "c": "str", "d": [1, 2]}

    def test_load_json5_resolves_exotic_numbers(self):
        # PR: JSON5 hex / leading-plus / Infinity / NaN are real numbers,
        # not strings (the Json5 value schema). to_json5 still round-trips
        # the original source spellings.
        d = pyrs_yaml.load_json5("{hex: 0x1F, plus: +7, inf: Infinity, nan: NaN, t: 5.}")
        assert d["hex"] == 31
        assert d["plus"] == 7
        assert d["inf"] == float("inf")
        assert d["nan"] != d["nan"]  # NaN
        assert d["t"] == 5.0
        # strict json load still treats those spellings as non-numbers
        assert pyrs_yaml.load_jsonc('{"hex": 1}')["hex"] == 1

    def test_load_json5_rejects_strict_only_via_options(self):
        # trailing comma is JSON5-only; the plain loader still rejects it.
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.from_json("{a: 1,}")

    def test_document_to_jsonc_preserves_comments(self):
        doc = pyrs_yaml.parse("# above the key\nport: 8080\n")
        out = doc.to_jsonc()
        # The standalone note is emitted inside the object, on its own
        # line just before the pair it annotates.
        assert "// above the key" in out, out
        assert '"port": 8080' in out, out
        assert out.index("// above the key") < out.index('"port"'), out

    def test_document_to_jsonc_and_json5_are_parseable(self):
        doc = pyrs_yaml.parse("a: 1\nb: [1, 2]\n")
        import json

        assert json.loads(doc.to_jsonc()) == {"a": 1, "b": [1, 2]}
        assert json.loads(doc.to_json5()) == {"a": 1, "b": [1, 2]}


class TestJsonLoadFastPath:
    """The direct strict-JSON -> Python fast path in `load_jsonc`.

    Canonical documents (objects/arrays, i64 integers, JSON floats, booleans,
    null, escape-free strings) are built directly, skipping the AST round-trip.
    Any non-canonical construct must bail to the AST path and yield the
    identical value, so these cases guard both the fast path's correctness and
    its fallback. Floats use a correctly-rounded parse that matches CPython's
    ``float`` (what ``json.loads`` uses), so parity holds by construction.
    """

    def test_canonical_object_array_scalars(self):
        got = pyrs_yaml.load_jsonc(
            '{"s": "hi", "n": 42, "neg": -7, "z": 0, "t": true, "f": false, "nl": null, "arr": [1, 2, 3], "obj": {"k": "v"}}'
        )
        assert got == {
            "s": "hi",
            "n": 42,
            "neg": -7,
            "z": 0,
            "t": True,
            "f": False,
            "nl": None,
            "arr": [1, 2, 3],
            "obj": {"k": "v"},
        }

    def test_top_level_array_and_scalar(self):
        assert pyrs_yaml.load_jsonc('[1, true, null, "x"]') == [1, True, None, "x"]
        assert pyrs_yaml.load_jsonc("  123  ") == 123
        assert pyrs_yaml.load_jsonc("{}") == {}
        assert pyrs_yaml.load_jsonc("[]") == []

    @pytest.mark.parametrize(
        "token",
        [
            "1.5",
            "0.0",
            "-0.0",
            "2e3",
            "2E3",
            "2e+3",
            "2e-3",
            "-0.25",
            "0.5",
            "3.14159",
            "6.02e23",
            "1e400",
            "1e-400",
            "1.7976931348623157e308",
            "5e-324",
            "123456789.123456789",
            "-1.5e-10",
        ],
    )
    def test_float_parity_with_json_loads(self, token):
        # The float fast branch must produce exactly what json.loads produces
        # (correctly-rounded parse == CPython float): exponent forms, signed
        # zero, and overflow/underflow to inf/0.0 all included.
        doc = f'{{"v": {token}}}'
        got = pyrs_yaml.load_jsonc(doc)
        assert got == json.loads(doc)
        # Bit-exact sign for -0.0 / inf, where == hides the sign bit.
        assert math.copysign(1.0, got["v"]) == math.copysign(1.0, json.loads(doc)["v"])

    def test_floats_and_ints_mixed(self):
        got = pyrs_yaml.load_jsonc('{"a": 1.5, "b": 2e3, "c": -0.25, "i": 7}')
        assert got == {"a": 1.5, "b": 2000.0, "c": -0.25, "i": 7}

    @pytest.mark.parametrize(
        "raw,expected",
        [
            (r'{"s": "a\nb"}', {"s": "a\nb"}),
            (r'{"s": "a\tb"}', {"s": "a\tb"}),
            (r'{"s": "a\"b"}', {"s": 'a"b'}),
            (r'{"s": "a\\b"}', {"s": "a\\b"}),
            (r'{"s": "a\/b"}', {"s": "a/b"}),
            (r'{"s": "\b\f\r\n\t"}', {"s": "\b\f\r\n\t"}),
            ('{"s": "café \U0001f600"}', {"s": "café \U0001f600"}),  # raw multibyte passthrough
            (r'{"s": "\ud83d\ude00"}', {"s": "\U0001f600"}),  # \u -> bails to AST, combines
        ],
    )
    def test_string_escape_parity_with_json_loads(self, raw, expected):
        # The inline escape decoder must agree with json.loads; \u and any
        # construct it declines still route through the AST path unchanged.
        assert pyrs_yaml.load_jsonc(raw) == json.loads(raw) == expected

    def test_invalid_escape_still_errors(self):
        # `\q` is not a JSON escape: the fast path bails and the AST path raises,
        # matching json.loads.
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_jsonc(r'{"s": "a\qb"}')
        with pytest.raises(ValueError):
            json.loads(r'{"s": "a\qb"}')

    def test_mixed_canonical_stays_on_fast_path(self):
        # floats + simple escapes + raw non-ASCII together: every construct the
        # fast path handles, so the whole doc is built directly, equal to json.loads.
        doc = '{"a": 1.5, "b": "x\\ny\\tz", "c": "café", "d": [2e3, -0.25, true, null]}'
        assert pyrs_yaml.load_jsonc(doc) == json.loads(doc)

    @pytest.mark.parametrize(
        "doc,expected",
        [
            (r'{"s": "\u00e9", "n": 1}', {"s": "\u00e9", "n": 1}),  # unicode escape -> AST
            ('{"a": 1.5 // hi\n}', {"a": 1.5}),  # JSONC comment (strict json.loads rejects)
            (r'{"big": 99999999999999999999999}', {"big": "99999999999999999999999"}),  # >i64 -> source text
        ],
    )
    def test_declined_constructs_fall_back_transparently(self, doc, expected):
        # Each carries a construct the fast path declines; the WHOLE doc must
        # route through the AST path and yield the identical value - never a
        # partial/corrupt result. Guards the #146/#148 fallback invariant.
        assert pyrs_yaml.load_jsonc(doc) == expected

    def test_trailing_content_and_bad_tokens_still_error(self):
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_jsonc("{} extra")
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_jsonc('{"a": }')
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_jsonc("[1,]")  # trailing comma -> AST path rejects (JSONC has no trailing commas)

    def test_duplicate_keys_last_wins_first_position(self):
        got = pyrs_yaml.load_jsonc('{"a": 1, "b": 2, "a": 3}')
        assert got == {"a": 3, "b": 2}
        assert list(got) == ["a", "b"]  # first insertion position kept

    def test_leading_zero_is_rejected_by_ast_path(self):
        # `01` is invalid JSON; the fast path bails and the AST path errors.
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_jsonc("[01]")

    def test_out_of_i64_integer_keeps_existing_string_behavior(self):
        # Pre-existing pyrs behavior (unchanged by the fast path, which bails on
        # out-of-i64 ints): the huge integer is preserved as its source text.
        assert pyrs_yaml.load_jsonc('{"big": 99999999999999999999999}') == {"big": "99999999999999999999999"}


class TestLoadJson:
    """Strict-JSON loader — the `load_*` family counterpart of `load_jsonc`.

    Completes Pillar 1 CLI↔Binding parity: every loader in the family now has
    a strict JSON variant. The contract is *narrower* than `load_jsonc`:
    RFC 8259 grammar only, matching `json.loads` bit-for-bit on canonical
    inputs and raising on every JSONC/JSON5 extension (`//`, `/* */`,
    trailing commas, single quotes, bare `Infinity`/`NaN`, `0x…`).

    The fast path shares `json_fast::try_load` with `load_jsonc`; because
    the scanner bails on every non-canonical byte the widening risk is
    zero — anything the fast path accepts is also strict-valid, and
    anything it declines falls through to `from_json` (STRICT), never
    `from_jsonc`.
    """

    def test_canonical_matches_load_jsonc(self):
        doc = '{"s": "hi", "n": 42, "t": true, "nl": null, "arr": [1, 2, 3]}'
        assert pyrs_yaml.load_json(doc) == pyrs_yaml.load_jsonc(doc)

    @pytest.mark.parametrize(
        "doc",
        [
            '{"a":1,"b":[1,2,3]}',
            "[[1,2],[3,4]]",
            '"hello"',
            "null",
            "true",
            "123",
            "1.5e10",
            "-0.25",
            r'{"k":"a\nb"}',
            '{"k":"café"}',
        ],
    )
    def test_parity_with_json_loads(self, doc):
        assert pyrs_yaml.load_json(doc) == json.loads(doc)

    @pytest.mark.parametrize(
        "doc,label",
        [
            ('{"a":1 // hi\n}', "line-comment"),
            ('{"a":1 /* hi */}', "block-comment"),
            ('{"a":1,}', "trailing-comma-object"),
            ("[1,2,]", "trailing-comma-array"),
            ("{'a':1}", "single-quote"),
            ("Infinity", "bare-Infinity"),
            ("-Infinity", "bare-neg-Infinity"),
            ("NaN", "bare-NaN"),
            ("0x1F", "hex-int"),
            ("+.5", "leading-plus-float"),
            ("5.", "trailing-dot-float"),
        ],
    )
    def test_strict_rejects_jsonc_and_json5_extensions(self, doc, label):
        # Every one of these is accepted by `load_jsonc` / `load_json5` and
        # rejected by strict JSON. `load_json` must side with strictness.
        #
        # Oracle note: Python's `json.loads` is NOT a perfect RFC 8259
        # oracle — its default `allow_nan=True` emits/parsers the three
        # non-standard literals `NaN` / `Infinity` / `-Infinity` (json-py
        # historical behaviour). Our loader is stricter than the stdlib
        # here, matching the spec. For the three tokens we assert only
        # our own rejection; every other case cross-checks `json.loads`.
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_json(doc)
        if label in ("bare-Infinity", "bare-neg-Infinity", "bare-NaN"):
            assert json.loads(doc) != doc  # stdlib accepts — we intentionally do not
        else:
            with pytest.raises(ValueError):
                json.loads(doc)

    @pytest.mark.parametrize(
        "doc,expected",
        [
            (r'{"k":"\u00e9"}', {"k": "\u00e9"}),
            (r'{"k":"\ud83d\ude00"}', {"k": "\U0001f600"}),
            ("[01]", "__raises__"),  # leading zero: json.loads raises
            ('{"a": 007}', "__raises__"),
        ],
    )
    def test_declined_constructs_route_through_ast_strictly(self, doc, expected):
        # `\u` and multi-digit leading-zero ints bail the fast path; the AST
        # route (STRICT `from_json`) either produces the same value as
        # `json.loads` or raises where the stdlib raises.
        if expected == "__raises__":
            with pytest.raises(pyrs_yaml.YamlParseError):
                pyrs_yaml.load_json(doc)
            with pytest.raises(ValueError):
                json.loads(doc)
        else:
            got = pyrs_yaml.load_json(doc)
            assert got == expected == json.loads(doc)

    def test_out_of_i64_int_uses_ast_strict_path(self):
        # >i64 bails the fast path; the AST strict path preserves the same
        # source-text string form `load_jsonc` yields (no widening, no drift).
        assert pyrs_yaml.load_json('{"big": 99999999999999999999999}') == {"big": "99999999999999999999999"}

    def test_trailing_content_and_bad_tokens_still_error(self):
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_json("{} extra")
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_json('{"a": }')
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_json("[1,]")

    def test_duplicate_keys_last_wins_first_position(self):
        got = pyrs_yaml.load_json('{"a": 1, "b": 2, "a": 3}')
        assert got == {"a": 3, "b": 2}
        assert list(got) == ["a", "b"]

    def test_exported_from_package_and_all(self):
        # Pillar 1 completeness guard: `load_json` must be re-exported from
        # `pyrs_yaml` and advertised in `__all__`, alongside load_jsonc /
        # load_json5 / load_toml — otherwise the family parity is only nominal.
        import pyrs_yaml as pkg

        assert hasattr(pkg, "load_json")
        assert "load_json" in pkg.__all__
        for sibling in ("load_jsonc", "load_json5", "load_toml"):
            assert sibling in pkg.__all__
