"""Python-surface tests for the TOML spoke (load_toml/from_toml/to_toml).

Mirrors the from_json family semantics: text in, text/dict out, with the
built-in !timestamp plugin active on the load path.
"""

import datetime

import pytest

import pyrs_yaml


@pytest.fixture(autouse=True)
def _ensure_builtin_plugins():
    # Built-in types share the global type registry with user plugins;
    # suites that call clear_type_handlers() (community-plugin tests)
    # may have wiped them before this file runs. Re-registration is
    # idempotent, making this file order-independent.
    from pyrs_yaml.plugins import _builtin

    _builtin._register_builtins()


SAMPLE = """
title = "config"
count = 3
ratio = 0.5
debug = true
created = 2026-01-02T03:04:05Z

[db]
host = "localhost"
port = 5432

[[servers]]
name = "a"

[[servers]]
name = "b"
"""


class TestLoadToml:
    def test_values(self):
        d = pyrs_yaml.load_toml(SAMPLE)
        assert d["title"] == "config"
        assert d["count"] == 3 and d["ratio"] == 0.5 and d["debug"] is True

    def test_nested_tables_and_aot(self):
        d = pyrs_yaml.load_toml(SAMPLE)
        assert d["db"] == {"host": "localhost", "port": 5432}
        assert d["servers"] == [{"name": "a"}, {"name": "b"}]

    def test_datetime_via_builtin_plugin(self):
        d = pyrs_yaml.load_toml(SAMPLE)
        assert isinstance(d["created"], datetime.datetime)
        assert d["created"].year == 2026

    def test_local_time_loads_not_crash(self):
        # toml-test regression: a bare local time once reached
        # datetime.fromisoformat and raised ValueError on valid TOML.
        d = pyrs_yaml.load_toml("t = 07:32:00")
        assert d["t"] == datetime.time(7, 32, 0)

    def test_local_time_without_seconds(self):
        d = pyrs_yaml.load_toml("t = 13:37")
        assert d["t"] == datetime.time(13, 37, 0)

    def test_local_date_is_date_not_midnight_datetime(self):
        d = pyrs_yaml.load_toml("bday = 1987-07-05")
        assert d["bday"] == datetime.date(1987, 7, 5)
        assert not isinstance(d["bday"], datetime.datetime)

    def test_lowercase_delimiter_offset_datetime(self):
        # TOML 1.0 permits a lowercase 't' delimiter and 'z' designator.
        d = pyrs_yaml.load_toml("dt = 1987-07-05t17:45:00z")
        assert d["dt"] == datetime.datetime(1987, 7, 5, 17, 45, tzinfo=datetime.timezone.utc)

    def test_fractional_seconds_any_precision(self):
        # toml-test regression: datetime.fromisoformat only accepts 3- or 6-digit
        # fractions before 3.11, but TOML allows any precision (e.g. a single
        # digit); the plugin normalizes to microseconds so it parses on 3.8+.
        d = pyrs_yaml.load_toml("ms = 1987-07-05T17:45:56.6+00:00")
        assert d["ms"] == datetime.datetime(1987, 7, 5, 17, 45, 56, 600000, tzinfo=datetime.timezone.utc)
        assert pyrs_yaml.load_toml("ms = 1987-07-05T17:45:56.555")["ms"].microsecond == 555000

    def test_fractional_seconds_in_local_time(self):
        d = pyrs_yaml.load_toml("t = 10:32:00.555")
        assert d["t"] == datetime.time(10, 32, 0, 555000)

    def test_temporal_roundtrip_to_toml(self):
        src = "d = 1987-07-05\nt = 07:32:00\ndt = 1987-07-05T17:45:00Z\n"
        out = pyrs_yaml.from_toml(src)
        assert pyrs_yaml.to_toml(out) is not None
        assert pyrs_yaml.load_toml(pyrs_yaml.to_toml(out)) == pyrs_yaml.load_toml(src)

    def test_string_true_stays_string(self):
        d = pyrs_yaml.load_toml('s = "true"\nn = "42"\n')
        assert d == {"s": "true", "n": "42"}

    def test_parse_error_is_typed(self):
        with pytest.raises(ValueError):
            pyrs_yaml.load_toml("broken = = 1\n")


class TestFromToml:
    def test_produces_loadable_yaml(self):
        y = pyrs_yaml.from_toml(SAMPLE)
        assert 'title: "config"' in y
        back = pyrs_yaml.safe_load(y)
        assert back["db"]["host"] == "localhost"

    def test_yaml_is_toml_stable(self):
        # toml -> yaml -> toml preserves values
        y = pyrs_yaml.from_toml(SAMPLE)
        data = pyrs_yaml.safe_load(y)
        t = pyrs_yaml.to_toml(y)
        assert pyrs_yaml.load_toml(t) == data


class TestToToml:
    def test_scalars_and_nesting(self):
        t = pyrs_yaml.to_toml("name: app\ncount: 3\nnested:\n  a: 1\n  b: two\n")
        assert 'name = "app"' in t
        assert "count = 3" in t
        assert 'b = "two"' in t

    def test_null_rejected(self):
        with pytest.raises(ValueError, match="null"):
            pyrs_yaml.to_toml("k: null\n")

    def test_root_scalar_rejected(self):
        with pytest.raises(ValueError):
            pyrs_yaml.to_toml("42\n")

    def test_quoted_number_stays_string(self):
        t = pyrs_yaml.to_toml('n: "42"\n')
        assert 'n = "42"' in t
        assert pyrs_yaml.load_toml(t) == {"n": "42"}


class TestDocumentToToml:
    """`YamlDocument.to_toml()` writes the AST directly (no to_yaml round-trip)."""

    def test_matches_module_to_toml(self):
        # The document method must be byte-identical to the text round-trip.
        yaml_src = "name: app\ncount: 3\nnested:\n  a: 1\n"
        doc = pyrs_yaml.parse(yaml_src)
        assert doc.to_toml() == pyrs_yaml.to_toml(doc.to_yaml())

    def test_roundtrips_from_parsed_toml(self):
        toml_in = '[server]\nhost = "0.0.0.0"\nport = 8080\n\n[[items]]\nname = "a"\nvalue = 1\n'
        doc = pyrs_yaml.parse(pyrs_yaml.from_toml(toml_in))
        # from_toml gives YAML text; parse it to a document; to_toml writes it
        # back, and reloading the result equals reloading the original TOML.
        assert pyrs_yaml.load_toml(doc.to_toml()) == pyrs_yaml.load_toml(toml_in)

    def test_root_scalar_rejected(self):
        doc = pyrs_yaml.parse("42\n")
        with pytest.raises(ValueError):
            doc.to_toml()

    def test_null_rejected(self):
        doc = pyrs_yaml.parse("k: null\n")
        with pytest.raises(ValueError, match="null"):
            doc.to_toml()


class TestCommentFidelity:
    """Document-level comment survival across the TOML -> hub -> TOML round trip.

    PR #131: to_toml reads the root mapping's leading_comment so the very
    first standalone note is no longer dropped (parity with the JSON writer's
    emit_root_leading).
    """

    def test_roundtrip_preserves_root_standalone_comment(self):
        toml_in = '# top note\nkey = "v"\n'
        out = pyrs_yaml.to_toml(pyrs_yaml.from_toml(toml_in))
        assert "# top note" in out, out
        assert out.index("# top note") < out.index("key"), out

    def test_root_comment_not_duplicated(self):
        out = pyrs_yaml.to_toml(pyrs_yaml.from_toml("# solo\nfirst = 1\n"))
        assert out.count("solo") == 1, out

    def test_root_and_trailing_comments_both_survive(self):
        out = pyrs_yaml.to_toml(pyrs_yaml.from_toml('# doc\nkey = "v"  # tail\n'))
        assert "# doc" in out and "# tail" in out, out

    def test_clean_doc_has_no_stray_comment_line(self):
        assert pyrs_yaml.to_toml(pyrs_yaml.from_toml("a = 1\nb = 2\n")) == "a = 1\nb = 2\n"


class TestMultilineStrings:
    """PR #132: multi-line TOML strings keep their shape through the hub.

    A triple-quoted basic or literal value is projected as a YAML literal
    block (so the style survives the text hub) and re-emitted by ``to_toml``
    as a triple-quoted block, while a single-line escaped string stays
    single-line.
    """

    def _roundtrip(self, src):
        return pyrs_yaml.to_toml(pyrs_yaml.from_toml(src))

    def test_multiline_basic_with_trailing_newline(self):
        src = 'x = """\nline1\nline2\n"""\n'
        out = self._roundtrip(src)
        assert '"""' in out, out
        assert pyrs_yaml.load_toml(out) == {"x": "line1\nline2\n"}

    def test_multiline_without_trailing_newline(self):
        src = 'x = """line1\nline2"""\n'
        out = self._roundtrip(src)
        assert '"""' in out, out
        assert pyrs_yaml.load_toml(out) == {"x": "line1\nline2"}

    def test_multiline_literal_value_and_shape_preserved(self):
        src = "x = '''\nraw \\path\nline'''\n"
        out = self._roundtrip(src)
        assert ('"""' in out) or ("'''" in out), out
        assert pyrs_yaml.load_toml(out) == {"x": "raw \\path\nline"}

    def test_multiline_quotes_and_backslash_preserved(self):
        src = 'x = """she said \\"hi\\"\nbye"""\n'
        out = self._roundtrip(src)
        assert pyrs_yaml.load_toml(out) == {"x": 'she said "hi"\nbye'}

    def test_roundtrip_is_idempotent(self):
        once = self._roundtrip('x = """a\nb\nc"""\n')
        twice = self._roundtrip(once)
        assert once == twice, (once, twice)

    def test_single_line_stays_single_line(self):
        src = 'x = "a\\nb"\n'
        out = self._roundtrip(src)
        assert '"""' not in out, out
        assert pyrs_yaml.load_toml(out) == {"x": "a\nb"}


class TestSectionHeaderCommentBoundary:
    """Characterization test for a KNOWN hub boundary (deliberately not root-fixed).

    A comment placed on a TOML table-header line (`[sec] # note`) has no
    faithful representation in the YAML hub: the shared YAML engine does not
    capture a comment sitting on a container's key line (verified: pure YAML
    `sec: # note` loses the note on parse too), so `to_toml` re-emits the header
    without it. Root-fixing this means changing the locked granit comment-capture
    model — a high-blast-radius core-engine change explicitly declined. These
    cases PIN the current behavior (values stay lossless; standalone/leading
    comments survive; the inline header note is dropped) so it cannot silently
    drift. See ROADMAP.md "Known engine boundaries".
    """

    def test_header_inline_comment_dropped_but_value_lossless(self):
        src = "a = 1\n\n[sec] # note\nx = 1\n"
        out = pyrs_yaml.to_toml(pyrs_yaml.from_toml(src))
        # Value round-trips through the hub unchanged (the real guarantee).
        assert pyrs_yaml.load_toml(out) == pyrs_yaml.load_toml(src)
        # The header-line inline comment is the documented boundary: dropped.
        assert "# note" not in out, out
        assert "[sec]" in out, out

    def test_standalone_section_comment_still_survives(self):
        # Only the *inline* header comment is lost; a standalone note on the
        # line above the header is preserved (via the #131 leading-comment path).
        src = "# above\n[sec]\nx = 1\n"
        out = pyrs_yaml.to_toml(pyrs_yaml.from_toml(src))
        assert "# above" in out, out
        assert "[sec]" in out, out

    def test_leaf_value_inline_comment_still_survives(self):
        # A trailing comment on a leaf key = value line is captured and kept.
        src = 'key = "v"  # tail\n'
        out = pyrs_yaml.to_toml(pyrs_yaml.from_toml(src))
        assert "# tail" in out, out


class TestNonAsciiStrings:
    """Multi-byte characters inside TOML strings.

    toml-test surfaced a char-boundary panic when the basic / multi-line string
    loops advanced byte-wise and sliced mid-character on non-ASCII content
    (U+00A0, U+0251, U+00A7, ...). Written with unicode escapes so the source has
    no ambiguous-literal warnings but the runtime string carries the same bytes.
    """

    def test_basic_string_non_ascii(self):
        src = 'v = "caf\u00e9\u00a0\u0251\u00a7"\n'
        assert pyrs_yaml.load_toml(src) == {"v": "caf\u00e9\u00a0\u0251\u00a7"}

    def test_multiline_basic_string_non_ascii(self):
        src = 'v = """caf\u00e9\u00a0\n\u0251\u00a7"""\n'
        assert pyrs_yaml.load_toml(src) == {"v": "caf\u00e9\u00a0\n\u0251\u00a7"}

    def test_non_ascii_survives_toml_round_trip(self):
        src = 'table = { name = "\u65e5\u672c\u8a9e\u00a0x" }\n'
        out = pyrs_yaml.to_toml(pyrs_yaml.from_toml(src))
        assert pyrs_yaml.load_toml(out) == pyrs_yaml.load_toml(src)


class TestControlCharacterRejection:
    """toml-test strictness: raw C0 control codes and DEL are forbidden in strings.

    Surfaced by the toml-test ``invalid/control`` corpus: NUL, FF, DLE (0x10),
    US (0x1F) and DEL (0x7F) must be rejected inside basic / literal and
    single / multi-line strings rather than silently accepted. Sources are built
    from ``chr()`` so no literal control byte lives in this file.
    """

    @pytest.mark.parametrize("byte", [0x00, 0x0C, 0x10, 0x1F, 0x7F])
    def test_rejected_in_basic_string(self, byte):
        src = 'v = "abc' + chr(byte) + 'def"\n'
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_toml(src)

    @pytest.mark.parametrize("byte", [0x00, 0x0C, 0x10, 0x1F, 0x7F])
    def test_rejected_in_literal_string(self, byte):
        src = "v = 'abc" + chr(byte) + "def'\n"
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_toml(src)

    @pytest.mark.parametrize("byte", [0x00, 0x0C, 0x10, 0x1F, 0x7F])
    def test_rejected_in_multiline_basic_string(self, byte):
        src = 'v = """abc' + chr(byte) + 'def"""\n'
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_toml(src)

    def test_tab_stays_legal(self):
        # Tab (0x09) is explicitly allowed by TOML inside strings.
        assert pyrs_yaml.load_toml('v = "a\tb"') == {"v": "a\tb"}
