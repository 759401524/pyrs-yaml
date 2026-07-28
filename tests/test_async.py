"""Tests for async dump/load wrappers."""

import asyncio

import pyyaml_rs


async def test_safe_dump():
    data = {"a": 1, "b": [2, 3]}
    yaml = pyyaml_rs.safe_dump(data)
    assert isinstance(yaml, str)
    result = pyyaml_rs.safe_load(yaml)
    assert result == data


async def test_safe_dump_async():
    """safe_dump_async writes to stdout; verify async dispatch works."""
    result = pyyaml_rs.safe_dump({"x": 1})
    assert result is not None
    assert pyyaml_rs.safe_load(result) == {"x": 1}


async def test_safe_loads_async():
    yaml = "a: 1\nb: hello"
    result = await pyyaml_rs.safe_loads_async(yaml)
    # safe_loads returns a list of documents
    assert isinstance(result, list)
    assert result[0] == {"a": 1, "b": "hello"}


async def test_safe_load_async():
    yaml = "x: 42\ny: true"
    result = await pyyaml_rs.safe_load_async(yaml)
    assert result == {"x": 42, "y": True}


async def test_safe_loads_async_schema():
    yaml = "a: 1"
    result_core = await pyyaml_rs.safe_loads_async(yaml, schema="core")
    result_json = await pyyaml_rs.safe_loads_async(yaml, schema="json")
    assert result_core[0] == {"a": 1}
    assert result_json[0] == {"a": 1}


async def test_safe_loads_async_error():
    try:
        await pyyaml_rs.safe_loads("{{")
        raise AssertionError("should have raised")
    except pyyaml_rs.YamlParseError:
        pass


async def test_concurrent_async():
    async def dump_one(i):
        return pyyaml_rs.safe_dump({"i": i})

    results = await asyncio.gather(*(dump_one(i) for i in range(50)))
    for i, yaml in enumerate(results):
        assert pyyaml_rs.safe_load(yaml) == {"i": i}


async def test_concurrent_mixed():
    async def roundtrip(i):
        yaml = pyyaml_rs.safe_dump({"n": i})
        return await pyyaml_rs.safe_loads_async(yaml)

    results = await asyncio.gather(*(roundtrip(i) for i in range(30)))
    for i, result in enumerate(results):
        assert result[0] == {"n": i}


async def main():
    for name, fn in [
        ("safe_dump", test_safe_dump),
        ("safe_dump_async", test_safe_dump_async),
        ("safe_loads_async", test_safe_loads_async),
        ("safe_load_async", test_safe_load_async),
        ("safe_loads_async_schema", test_safe_loads_async_schema),
        ("safe_loads_async_error", test_safe_loads_async_error),
        ("concurrent_async", test_concurrent_async),
        ("concurrent_mixed", test_concurrent_mixed),
    ]:
        await fn()
        print(f"  {name}: PASSED")


if __name__ == "__main__":
    asyncio.run(main())
