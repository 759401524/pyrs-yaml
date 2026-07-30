"""
Benchmarks for the public Python API of pyrs-yaml.

Complements tests/test_benchmark_crosslib.py (which compares parse/serialize against
PyYAML and ruamel.yaml) by covering the rest of the surface exposed to Python:
safe_load, safe_loads, safe_dump, from_dict, from_json,
YamlDocument.to_json, YamlDocument.validate, YamlDocument.reparse,
parse_all_docs and parse_stream.
"""

import io

import numpy as np
import pyrs_yaml
import pytest

pytestmark = pytest.mark.benchmark

CONFIG_YAML = """
server:
  host: 0.0.0.0
  port: 8080
  ssl: true
  workers: 4
  endpoints:
    - path: /api/v1/users
      methods: [GET, POST]
    - path: /api/v1/orders
      methods: [GET, POST, PATCH]

database:
  type: postgresql
  host: db.example.com
  port: 5432
  pool:
    min_size: 5
    max_size: 20
    timeout: 30

logging:
  level: INFO
  outputs:
    - stdout
    - file:/var/log/app.log

description: |
  A literal block scalar that
  preserves newlines exactly.
"""

MULTI_DOC_YAML = "\n---\n".join(
    f"doc: {index}\nvalues: [1, 2, 3]\nnested:\n  key: value_{index}\n" for index in range(20)
)

ANCHOR_YAML = """
defaults: &defaults
  timeout: 30
  retries: 3
  backoff: exponential

api:
  <<: *defaults
  port: 8080

worker:
  <<: *defaults
  concurrency: 8
"""

CONFIG_DATA = {
    "server": {
        "host": "0.0.0.0",
        "port": 8080,
        "ssl": True,
        "workers": 4,
        "tags": ["production", "eu-west-1"],
    },
    "database": {
        "type": "postgresql",
        "pool": {"min_size": 5, "max_size": 20, "timeout": 30},
    },
    "items": [{"name": f"item_{index}", "value": index * 10} for index in range(50)],
}

CONFIG_JSON = (
    '{"server": {"host": "0.0.0.0", "port": 8080, "ssl": true}, '
    '"items": [{"name": "a", "value": 1}, {"name": "b", "value": 2}]}'
)

SCHEMA = {
    "type": "object",
    "required": ["server", "database"],
    "properties": {
        "server": {
            "type": "object",
            "required": ["host", "port"],
            "properties": {
                "host": {"type": "string"},
                "port": {"type": "integer"},
                "ssl": {"type": "boolean"},
            },
        },
        "database": {"type": "object"},
    },
}


def test_safe_load(benchmark):
    result = benchmark(pyrs_yaml.safe_load, CONFIG_YAML)
    assert result["server"]["port"] == 8080


def test_safe_load_anchors(benchmark):
    result = benchmark(pyrs_yaml.safe_load, ANCHOR_YAML)
    assert result["api"]["timeout"] == 30


def test_safe_loads_multi_document(benchmark):
    result = benchmark(pyrs_yaml.safe_loads, MULTI_DOC_YAML)
    assert len(result) == 20


def test_parse_all_docs(benchmark):
    result = benchmark(pyrs_yaml.parse_all_docs, MULTI_DOC_YAML)
    assert len(result) == 20


def test_parse_stream(benchmark):
    result = benchmark(lambda: list(pyrs_yaml.parse_stream(CONFIG_YAML)))
    assert result


def test_safe_dump(benchmark):
    result = benchmark(pyrs_yaml.safe_dump, CONFIG_DATA)
    assert "postgresql" in result


def test_from_dict(benchmark):
    result = benchmark(pyrs_yaml.from_dict, CONFIG_DATA)
    assert result


def test_from_json(benchmark):
    result = benchmark(pyrs_yaml.from_json, CONFIG_JSON)
    assert "server" in result


def test_safe_dump_ndarray(benchmark):
    array = np.arange(4096, dtype="float64").reshape(64, 64)
    result = benchmark(pyrs_yaml.safe_dump, array)
    assert result


def test_document_to_yaml_sorted(benchmark):
    doc = pyrs_yaml.parse(CONFIG_YAML)
    result = benchmark(lambda: doc.to_yaml_with_options(sort_keys=True))
    assert result


def test_document_to_json(benchmark):
    doc = pyrs_yaml.parse(CONFIG_YAML)
    result = benchmark(doc.to_json)
    assert result.startswith("{")


def test_document_validate(benchmark):
    doc = pyrs_yaml.parse(CONFIG_YAML)
    benchmark(doc.validate, SCHEMA)


def test_document_reparse(benchmark):
    doc = pyrs_yaml.parse(CONFIG_YAML)
    benchmark(lambda: doc.reparse(schema="yaml1.1"))


@pytest.mark.parametrize("schema", ["core", "yaml1.1", "json"])
def test_parse_schemas(benchmark, schema):
    result = benchmark(lambda: pyrs_yaml.parse(CONFIG_YAML, schema=schema))
    assert result is not None


def test_document_to_yaml_explicit(benchmark):
    doc = pyrs_yaml.parse(CONFIG_YAML)
    result = benchmark(lambda: doc.to_yaml_with_options(indent_size=4, explicit_start=True, sort_keys=True))
    assert result


def test_parse_file(benchmark):
    file = io.StringIO(CONFIG_YAML)
    result = benchmark(lambda: pyrs_yaml.parse(file.getvalue()))
    assert result
