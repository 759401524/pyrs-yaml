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
