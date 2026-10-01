"""Tests for max_depth parameter in pyrs-yaml Python API."""

import subprocess
import sys
import textwrap

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


# ── TOML depth guard ────────────────────────────────────────────────────────
# The TOML parser previously had NO nesting budget (unlike JSON's
# DEFAULT_MAX_DEPTH and YAML's parse max_depth): a deeply nested array or
# inline table recursed `parse_value` until the native stack overflowed and
# ABORTED the whole process (verified: a 5000-deep array → exit code
# 0xC00000FD STACK_OVERFLOW, no Python exception). This is the TOML
# analogue of the #166 YAML merge stack overflow. The parser now carries a
# 1000-deep guard mirroring JSON, so over-nested input raises cleanly.


def _deep_toml_array(n: int) -> str:
    return "a = " + "[" * n + "]" * n


def _deep_toml_inline_table(n: int) -> str:
    return "a = " + "{b = " * n + "1" + "}" * n


@pytest.mark.parametrize("build", [_deep_toml_array, _deep_toml_inline_table], ids=["array", "inline_table"])
def test_toml_within_max_depth_succeeds(build):
    # 500 deep is comfortably within the 1000 budget (both constructors).
    assert pyrs_yaml.load_toml(build(500)) is not None


@pytest.mark.parametrize("build", [_deep_toml_array, _deep_toml_inline_table], ids=["array", "inline_table"])
def test_toml_rejects_exceeded_max_depth_cleanly(build):
    # Just over the boundary: must surface a typed parse error, not a crash.
    # Safe in-process because the guard prevents the stack overflow.
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_toml(build(2000))


def test_toml_extreme_depth_crash_canary():
    """Regression canary: an absurdly deep array must reject, never abort.

    Runs in a subprocess so that if the depth guard is ever removed, the
    stack overflow crashes only the child (non-zero exit) and this test
    fails cleanly -- instead of killing the whole pytest/nextest runner
    the way the pre-fix abort did.
    """
    child = textwrap.dedent(
        """
        import sys, pyrs_yaml
        src = "a = " + "[" * 100000 + "]" * 100000
        try:
            pyrs_yaml.load_toml(src)
        except pyrs_yaml.YamlParseError:
            print("REJECTED")
        """
    )
    r = subprocess.run([sys.executable, "-c", child], capture_output=True, text=True, timeout=60)
    assert r.returncode == 0, f"child crashed (stack overflow?): rc={r.returncode} {r.stderr[-200:]}"
    assert "REJECTED" in r.stdout
