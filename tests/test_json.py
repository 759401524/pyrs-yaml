"""JSON/YAML conversion tests — from_dict, from_json."""

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

    def test_from_jsonc_strips_comments(self):
        yaml_str = pyrs_yaml.from_jsonc('{"a": 1, // note\n "b": 2}')
        assert "a: 1" in yaml_str
        assert "note" not in yaml_str  # comments dropped for the YAML view

    def test_from_json5_accepts_wider_grammar(self):
        # unquoted keys, single-quoted strings, trailing comma
        yaml_str = pyrs_yaml.from_json5("{name: 'Alice', active: true,}")
        assert "name:" in yaml_str
        assert "Alice" in yaml_str

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
