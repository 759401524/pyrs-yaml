"""
Performance sanity tests — basic parse/serialize speed checks.
Uses pytest-codspeed for statistical timing.
"""

import pyrs_yaml


def test_parse_speed(benchmark):
    yaml_str = "key: value\nlist:\n  - a\n  - b\n  - c"
    result = benchmark(pyrs_yaml.parse, yaml_str)
    assert result is not None


def test_serialize_speed(benchmark):
    doc = pyrs_yaml.parse("key: value\nlist:\n  - a\n  - b")
    result = benchmark(doc.to_yaml)
    assert result is not None
