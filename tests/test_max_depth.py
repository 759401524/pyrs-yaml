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


def test_rejects_exceeded_max_depth():
    deep_yaml = _deep_nested_yaml(200)
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        pyrs_yaml.parse(deep_yaml, max_depth=100)


def test_accepts_custom_max_depth():
    deep_yaml = _deep_nested_yaml(50)
    assert pyrs_yaml.parse(deep_yaml, max_depth=100) is not None


def test_accepts_default_max_depth():
    deep_yaml = _deep_nested_yaml(100)
    assert pyrs_yaml.parse(deep_yaml) is not None


def test_max_depth_exception_is_value_error():
    assert issubclass(pyrs_yaml.YamlMaxDepthError, ValueError)


@pytest.mark.parametrize(
    "func_name,args",
    [
        ("safe_load", {}),
        ("safe_loads", {}),
        ("parse_all_docs", {}),
    ],
    ids=["safe_load", "safe_loads", "parse_all_docs"],
)
def test_rejects_depth_limit_in_parse_funcs(func_name, args):
    deep_yaml = _deep_nested_yaml(200)
    func = getattr(pyrs_yaml, func_name)
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        func(deep_yaml, max_depth=100, **args)


def test_rejects_depth_limit_in_serialize():
    deep_yaml = _deep_nested_yaml(200)
    doc = pyrs_yaml.parse(deep_yaml, max_depth=1000)
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        doc.to_yaml_with_options(max_depth=50)


def test_serializes_normal_depth_succeeds():
    doc = pyrs_yaml.parse("key: value\nnested:\n  a: 1\n  b: 2")
    result = doc.to_yaml()
    assert result is not None
    assert "key: value" in result


def test_rejects_depth_limit_in_parse_file(tmp_path):
    deep_yaml = _deep_nested_yaml(200)
    f = tmp_path / "deep.yaml"
    f.write_text(deep_yaml)
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        pyrs_yaml.parse_file(str(f), max_depth=100)
