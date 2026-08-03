"""Duplicate key tests — YamlDuplicateKeyError and allow_duplicate_keys override."""

import pyrs_yaml
import pytest

from tests.data import yaml_samples as yaml


class TestDuplicateKeys:
    """Test duplicate key detection and override."""

    def test_duplicate_keys_default_false_raises_error(self):
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.YAML().parse(yaml.DUPLICATE_KEYS_OVERRIDE)

    def test_duplicate_keys_true_allows_override(self):
        doc = pyrs_yaml.YAML(allow_duplicate_keys=True).parse(yaml.DUPLICATE_KEYS_OVERRIDE)
        assert doc.get("key") == "second"

    def test_top_level_parse_allow_duplicate_keys(self):
        doc = pyrs_yaml.parse(yaml.DUPLICATE_KEYS_OVERRIDE, allow_duplicate_keys=True)
        assert doc.get("key") == "second"

    def test_top_level_parse_raises_by_default(self):
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.parse(yaml.DUPLICATE_KEYS_OVERRIDE)

    def test_safe_load_allow_duplicate_keys_last_wins(self):
        data = pyrs_yaml.safe_load(yaml.DUPLICATE_KEYS_OVERRIDE, allow_duplicate_keys=True)
        assert data["key"] == "second"

    def test_safe_load_raises_by_default(self):
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.safe_load(yaml.DUPLICATE_KEYS_OVERRIDE)

    def test_roundtrip_keeps_last_occurrence(self):
        doc = pyrs_yaml.YAML(typ="rt", allow_duplicate_keys=True).parse(yaml.DUPLICATE_KEYS_OVERRIDE)
        output = doc.to_yaml()
        assert "second" in output
        assert "first" not in output

    def test_nested_duplicate_keys(self):
        nested = "outer:\n  dup: a\n  dup: b"
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.parse(nested)
        doc = pyrs_yaml.parse(nested, allow_duplicate_keys=True)
        assert doc.get("outer")["dup"] == "b"

    def test_null_keys_are_not_duplicate(self):
        # yaml-test-suite 2JQS: duplicate empty/null keys are valid
        doc = pyrs_yaml.parse(": a\n: b")
        assert doc.get("~") == "b"
        doc = pyrs_yaml.parse("~: a\n~: b")
        assert doc.get("~") == "b"
        doc = pyrs_yaml.parse("null: a\nnull: b")
        assert doc.get("null") == "b"

    def test_null_key_then_real_duplicate_still_errors(self):
        yaml_text = "~: a\nkey: first\nkey: second"
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.parse(yaml_text)

    def test_parse_all_docs_duplicate_keys(self):
        multi = "---\nkey: first\nkey: second\n---\nother: value"
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.parse_all_docs(multi)
        docs = pyrs_yaml.parse_all_docs(multi, allow_duplicate_keys=True)
        assert len(docs) == 2
        assert docs[0].get("key") == "second"

    def test_safe_loads_duplicate_keys(self):
        multi = "---\nkey: first\nkey: second\n---\nother: value"
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.safe_loads(multi)
        docs = pyrs_yaml.safe_loads(multi, allow_duplicate_keys=True)
        assert docs[0]["key"] == "second"
