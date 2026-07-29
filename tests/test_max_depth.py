"""Tests for max_depth parameter in pyrs-yaml Python API."""

import pyrs_yaml
import pytest


def _deep_nested_yaml(depth: int) -> str:
    """Create a deeply nested flow-style YAML mapping.

    Produces: {a: {a: {a: ... {a: 1}...}}}
    """
    nested = "1"
    for _ in range(depth):
        nested = "{a: " + nested + "}"
    return nested


def test_max_depth_exceeded():
    """Deeply nested YAML should fail with max_depth error."""
    deep_yaml = _deep_nested_yaml(200)
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        pyrs_yaml.parse(deep_yaml, max_depth=100)


def test_max_depth_custom_limit():
    """Custom max_depth should work for reasonable depths."""
    deep_yaml = _deep_nested_yaml(50)
    doc = pyrs_yaml.parse(deep_yaml, max_depth=100)
    assert doc is not None


def test_max_depth_default():
    """Default max_depth should be 1000 — allows reasonable nesting."""
    deep_yaml = _deep_nested_yaml(100)
    doc = pyrs_yaml.parse(deep_yaml)
    assert doc is not None


def test_max_depth_exception_importable():
    """YamlMaxDepthError should be importable and match exception hierarchy."""
    assert issubclass(pyrs_yaml.YamlMaxDepthError, ValueError)


def test_max_depth_safe_load():
    """safe_load should respect max_depth."""
    deep_yaml = _deep_nested_yaml(200)
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        pyrs_yaml.safe_load(deep_yaml, max_depth=100)


def test_max_depth_safe_loads():
    """safe_loads should respect max_depth."""
    deep_yaml = _deep_nested_yaml(200)
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        pyrs_yaml.safe_loads(deep_yaml, max_depth=100)


def test_max_depth_parse_all_docs():
    """parse_all_docs should respect max_depth."""
    deep_yaml = _deep_nested_yaml(200)
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        pyrs_yaml.parse_all_docs(deep_yaml, max_depth=100)


def test_max_depth_parse_file(tmp_path):
    """parse_file should respect max_depth."""
    deep_yaml = _deep_nested_yaml(200)
    f = tmp_path / "deep.yaml"
    f.write_text(deep_yaml)
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        pyrs_yaml.parse_file(str(f), max_depth=100)
