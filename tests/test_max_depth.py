"""Tests for max_depth parameter in pyrs-yaml Python API."""

import pytest

import pyrs_yaml


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
        ("parse_stream", {}),
    ],
    ids=["safe_load", "safe_loads", "parse_all_docs", "parse_stream"],
)
def test_rejects_depth_limit_in_parse_funcs(func_name, args):
    deep_yaml = _deep_nested_yaml(200)
    func = getattr(pyrs_yaml, func_name)
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        func(deep_yaml, max_depth=100, **args)


def test_rejects_depth_limit_in_parse_stream_iterator():
    deep_yaml = _deep_nested_yaml(200)
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        list(pyrs_yaml.parse_stream(deep_yaml, max_depth=100))


def test_rejects_depth_limit_in_parse_stream_callback():
    deep_yaml = _deep_nested_yaml(200)
    calls = []
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        pyrs_yaml.parse_stream(deep_yaml, on_event=lambda e: calls.append(e) or True, max_depth=100)


def test_rejects_depth_limit_in_read_markdown_str():
    deep_yaml = _deep_nested_yaml(200)
    md = f"---\n{deep_yaml}\n---\nbody"
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        pyrs_yaml.read_markdown_str(md, max_depth=100)


def test_rejects_depth_limit_in_read_markdown_file(tmp_path):
    deep_yaml = _deep_nested_yaml(200)
    f = tmp_path / "deep.md"
    f.write_text(f"---\n{deep_yaml}\n---\nbody")
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        pyrs_yaml.read_markdown(str(f), max_depth=100)


def test_read_markdown_str_default_depth_ok():
    deep_yaml = _deep_nested_yaml(100)
    md = f"---\n{deep_yaml}\n---\nbody"
    frontmatter, content = pyrs_yaml.read_markdown_str(md)
    assert frontmatter is not None
    assert content == "body"


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


def test_resolve_tags_deep_nesting_does_not_recursively_crash():
    # Deeply nested block mapping with a tagged scalar at the bottom:
    # resolve_tags recurses over the AST when a tag handler is registered.
    inner = "leaf: !mytag 42\n"
    for i in range(60):
        inner = f"k{i}:\n  {inner}"
    doc = pyrs_yaml.parse(inner, max_depth=1000)
    data = doc.to_dict()
    assert data is not None


def test_resolve_tags_with_custom_handler_deep_tree():
    # Verify invoke-through-handler path at nesting depth (no recursion crash).
    @pyrs_yaml.register_type
    class Upper(pyrs_yaml.CustomType):
        def can_parse(self, value: str) -> bool:
            return False

        def from_yaml(self, value: str):
            return value.upper()

    try:
        inner = "leaf: !upper hello\n"
        for i in range(60):
            inner = f"k{i}:\n  {inner}"
        doc = pyrs_yaml.parse(inner, max_depth=1000)
        data = doc.to_dict()
        assert data is not None
    finally:
        pyrs_yaml.clear_type_handlers()
