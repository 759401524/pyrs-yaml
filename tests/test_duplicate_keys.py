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
