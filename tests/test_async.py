"""Tests for async dump/load wrappers."""

import asyncio

import pytest

import pyrs_yaml


def test_safe_dump():
    data = {"a": 1, "b": [2, 3]}
    yaml = pyrs_yaml.safe_dump(data)
    assert isinstance(yaml, str)
    result = pyrs_yaml.safe_load(yaml)
    assert result == data


async def test_safe_dump_async():
    """safe_dump_async writes to stdout; verify async dispatch works."""
    result = await pyrs_yaml.safe_dump_async({"x": 1})
    assert result is not None
    assert pyrs_yaml.safe_load(result) == {"x": 1}


async def test_safe_loads_async():
    yaml = "a: 1\nb: hello"
    result = await pyrs_yaml.safe_loads_async(yaml)
    # safe_loads returns a list of documents
    assert isinstance(result, list)
    assert result[0] == {"a": 1, "b": "hello"}


async def test_safe_load_async():
    yaml = "x: 42\ny: true"
    result = await pyrs_yaml.safe_load_async(yaml)
    assert result == {"x": 42, "y": True}


async def test_safe_loads_async_schema():
    yaml = "a: 1"
    result_core = await pyrs_yaml.safe_loads_async(yaml, schema="core")
    result_json = await pyrs_yaml.safe_loads_async(yaml, schema="json")
    assert result_core[0] == {"a": 1}
    assert result_json[0] == {"a": 1}


async def test_safe_loads_async_error():
    with pytest.raises(pyrs_yaml.YamlParseError):
        await pyrs_yaml.safe_loads_async("{{")


async def test_concurrent_async():
    async def dump_one(i):
        return pyrs_yaml.safe_dump({"i": i})

    results = await asyncio.gather(*(dump_one(i) for i in range(50)))
    for i, yaml in enumerate(results):
        assert pyrs_yaml.safe_load(yaml) == {"i": i}


async def test_concurrent_mixed():
    async def roundtrip(i):
        yaml = pyrs_yaml.safe_dump({"n": i})
        return await pyrs_yaml.safe_loads_async(yaml)

    results = await asyncio.gather(*(roundtrip(i) for i in range(30)))
    for i, result in enumerate(results):
        assert result[0] == {"n": i}
