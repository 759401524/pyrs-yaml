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
