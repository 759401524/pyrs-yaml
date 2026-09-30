"""Syntax-boundary edge tests for TOML / JSON / JSONC / JSON5 (pillar 2.1).

`test_edge_cases.py` covers the YAML spine; this pins the dialect formats'
grammar edges - empty/whitespace input, extreme nesting (and the depth limit),
oversized numbers, illegal escapes, unterminated strings, and special characters
- documenting pyrs's actual, spec-consistent behavior for each so a regression in
any direction is caught. Each assertion reflects verified current behavior.
"""

import pytest

import pyrs_yaml

# ── empty / whitespace-only input ────────────────────────────────────────────


# An empty TOML document is a valid empty table.
@pytest.mark.parametrize("src", ["", "   ", "\n", "  \n\t"])
def test_toml_empty_is_empty_table(src):
    assert pyrs_yaml.load_toml(src) == {}


# JSON / JSONC / JSON5 have no "empty document" value, so they must reject it.
@pytest.mark.parametrize("loader", [pyrs_yaml.load_jsonc, pyrs_yaml.load_json5])
@pytest.mark.parametrize("src", ["", "   ", "\n"])
def test_json_family_empty_is_error(loader, src):
    with pytest.raises(pyrs_yaml.YamlParseError):
        loader(src)


# ── extreme nesting / depth limit ────────────────────────────────────────────


@pytest.mark.parametrize("depth", [1, 32, 200])
def test_deeply_nested_within_limit_loads(depth):
    doc = "[" * depth + "]" * depth
    result = pyrs_yaml.load_jsonc(doc)
    assert isinstance(result, list)


def test_deeply_nested_over_limit_errors():
    doc = "[" * 1500 + "]" * 1500
    with pytest.raises(pyrs_yaml.YamlParseError, match="depth"):
        pyrs_yaml.load_jsonc(doc)


# ── oversized numbers ─────────────────────────────────────────────────────────


def test_json_int_beyond_i64_kept_as_source_text():
    # pyrs's documented behavior: an integer literal past i64 is preserved as its
    # source string rather than silently losing precision as a float.
    assert pyrs_yaml.load_jsonc('{"n": 99999999999999999999999}') == {"n": "99999999999999999999999"}


def test_json_exponent_overflow_is_inf():
    result = pyrs_yaml.load_jsonc('{"n": 1e400}')
    assert result["n"] == float("inf")


def test_toml_int_beyond_range_is_error():
    # TOML integers are i64-bounded by spec; out-of-range must be rejected, not
    # coerced to a float or truncated.
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_toml("n = 99999999999999999999999")


# ── illegal escapes ───────────────────────────────────────────────────────────


@pytest.mark.parametrize(
    "loader,src",
    [
        (pyrs_yaml.load_jsonc, r'{"s": "a\q"}'),
        (pyrs_yaml.load_json5, r"{s: 'a\q'}"),
        (pyrs_yaml.load_toml, 's = "a\\q"'),
    ],
)
def test_illegal_escape_rejected(loader, src):
    with pytest.raises(pyrs_yaml.YamlParseError):
        loader(src)


# ── unterminated strings / structures ─────────────────────────────────────────


@pytest.mark.parametrize(
    "loader,src",
    [
        (pyrs_yaml.load_jsonc, '{"s": "abc'),
        (pyrs_yaml.load_jsonc, '{"a": 1'),
        (pyrs_yaml.load_json5, "{s: 'abc"),
        (pyrs_yaml.load_toml, 's = "abc'),
    ],
)
def test_unterminated_input_rejected(loader, src):
    with pytest.raises(pyrs_yaml.YamlParseError):
        loader(src)


# ── special characters preserved in strings/keys ─────────────────────────────


def test_json_unicode_escape_in_key_and_value():
    assert pyrs_yaml.load_jsonc('{"a b\\u00a7": "caf\\u00e9"}') == {"a b\u00a7": "caf\u00e9"}


def test_json5_raw_non_ascii_preserved():
    src = "{k: 'caf\u00e9 \u65e5\u672c'}"
    assert pyrs_yaml.load_json5(src) == {"k": "caf\u00e9 \u65e5\u672c"}


def test_toml_multiline_non_ascii_preserved():
    src = 's = """caf\u00e9\u00a0x"""'
    assert pyrs_yaml.load_toml(src) == {"s": "caf\u00e9\u00a0x"}
