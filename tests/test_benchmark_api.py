"""
Benchmarks for the public Python API of pyrs-yaml.

Complements tests/test_benchmark_crosslib.py (which compares parse/serialize against
PyYAML and ruamel.yaml) by covering the rest of the surface exposed to Python:
safe_load, safe_loads, safe_dump, from_dict, from_json,
YamlDocument.to_json, YamlDocument.validate, YamlDocument.reparse,
parse_all_docs and parse_stream.
"""

import io

import pyrs_yaml
import pytest

from tests.data.yaml_samples import (
    BENCHMARK_ANCHOR as ANCHOR_YAML,
)
from tests.data.yaml_samples import (
    BENCHMARK_CONFIG_DATA as CONFIG_DATA,
)
from tests.data.yaml_samples import (
    BENCHMARK_CONFIG_JSON as CONFIG_JSON,
)
from tests.data.yaml_samples import (
    BENCHMARK_LARGE,
    BENCHMARK_SMALL,
)
from tests.data.yaml_samples import (
    BENCHMARK_MEDIUM as CONFIG_YAML,
)
from tests.data.yaml_samples import (
    BENCHMARK_MULTI_DOC as MULTI_DOC_YAML,
)
from tests.data.yaml_samples import (
    BENCHMARK_SCHEMA as SCHEMA,
)

pytest.importorskip("numpy")
import numpy as np

pytestmark = pytest.mark.benchmark


def test_safe_load(benchmark):
    result = benchmark(pyrs_yaml.safe_load, CONFIG_YAML)
    assert result["server"]["port"] == 8080


YAML_INPUTS = {"small": BENCHMARK_SMALL, "medium": CONFIG_YAML, "large": BENCHMARK_LARGE}
SIZES = ["small", "medium", "large"]


@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_safe_load_sized(benchmark, size):
    """safe_load across sizes: parse + Python object conversion."""
    result = benchmark(pyrs_yaml.safe_load, YAML_INPUTS[size])
    assert result is not None


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


def test_load_stream(benchmark):
    """Benchmark load_stream (incremental YamlStream wrapper)."""
    file = io.StringIO(CONFIG_YAML)
    result = benchmark(lambda: list(pyrs_yaml.YAML().load_stream(file)))
    assert result


def test_parse_stream_multidoc(benchmark):
    """Benchmark parse_stream with multi-document YAML."""
    result = benchmark(lambda: list(pyrs_yaml.parse_stream(MULTI_DOC_YAML)))
    assert len(result) > 20  # 20 docs + stream events


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


# ── Cross-library safe_load comparison ──

try:
    import yaml as pyyaml

    HAS_PYYAML = True
except ImportError:
    HAS_PYYAML = False
    pyyaml = None


@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_pyyaml_safe_load(benchmark, size):
    """PyYAML safe_load for cross-library comparison."""
    result = benchmark(pyyaml.safe_load, YAML_INPUTS[size])
    assert result is not None
