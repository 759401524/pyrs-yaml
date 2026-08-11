"""
Benchmarks for the public Python API of pyrs-yaml.

Complements tests/test_benchmark_crosslib.py (which compares parse/serialize against
PyYAML and ruamel.yaml) by covering the rest of the surface exposed to Python:
safe_load, safe_loads, safe_dump, from_dict, from_json,
YamlDocument.to_json, YamlDocument.validate, YamlDocument.reparse,
parse_all_docs and parse_stream.
"""

import io
from datetime import datetime, timezone

import pytest

import pyrs_yaml
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


# ── Scalar-type classification benchmarks ──
# Synthetic single-level mapping docs whose VALUES exercise different scalar
# resolution paths: strings (resolve_core_type fast-path), numbers (full chain),
# quoted scalars (round-trip de-quoting), and YAML 1.1 legacy booleans.
# Each doc uses a single resolution class (strings start with fast-path first
# bytes, numbers with keep-set first bytes) so the L2 contrast is clean.

_SCALAR_KEYS = 1000


def _make_scalar_doc(values):
    return "".join(f"k{i}: {values[i % len(values)]}\n" for i in range(_SCALAR_KEYS))


SCALAR_DOC_STRINGS = _make_scalar_doc(["hello", "_key", "数据", "a/b", "http://x", "?q"])
SCALAR_DOC_NUMBERS = _make_scalar_doc(["42", "-10", "3.14", "0x1F", "0o17", "1e3", "+7"])
SCALAR_DOC_QUOTED = _make_scalar_doc(['"42"', "'hello world'", '"true"', '"3.14"', '"null"', "'x y z'"])
SCALAR_DOC_LEGACY_BOOLS = _make_scalar_doc(["yes", "no", "on", "off", "y", "n"])


@pytest.mark.parametrize(
    "doc",
    [SCALAR_DOC_STRINGS, SCALAR_DOC_NUMBERS, SCALAR_DOC_QUOTED],
    ids=["strings", "numbers", "quoted"],
)
def test_safe_load_scalar_types(benchmark, doc):
    """safe_load on docs whose scalars hit distinct resolution paths."""
    result = benchmark(pyrs_yaml.safe_load, doc)
    assert len(result) == _SCALAR_KEYS


def test_safe_load_scalar_types_legacy_bools(benchmark):
    """yaml1.1 legacy bools exercise the pre-core bool table."""
    result = benchmark(lambda: pyrs_yaml.safe_load(SCALAR_DOC_LEGACY_BOOLS, schema="yaml1.1"))
    assert len(result) == _SCALAR_KEYS


_YAML_INSTANCE = pyrs_yaml.YAML()


def test_safe_load_instance(benchmark):
    """YAML() instance method path (exercises the fast-path branch in safe_load)."""
    result = benchmark(_YAML_INSTANCE.safe_load, CONFIG_YAML)
    assert result["server"]["port"] == 8080


def test_safe_loads_instance(benchmark):
    """YAML() instance safe_loads path (fast-path branch, multi-doc)."""
    result = benchmark(_YAML_INSTANCE.safe_loads, MULTI_DOC_YAML)
    assert len(result) == 20


def test_to_dict(benchmark):
    """YamlDocument.to_dict on an anchor-free doc (L1 simple path)."""
    doc = pyrs_yaml.parse(CONFIG_YAML)
    result = benchmark(doc.to_dict)
    assert result["server"]["port"] == 8080


def test_to_dict_anchors(benchmark):
    """YamlDocument.to_dict on an anchored/merged doc (conservative path)."""
    doc = pyrs_yaml.parse(ANCHOR_YAML)
    result = benchmark(doc.to_dict)
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


def test_dump_stream(benchmark):
    buf = io.StringIO()
    yaml = pyrs_yaml.YAML()
    benchmark(lambda: (buf.seek(0), yaml.dump_stream(buf, [CONFIG_DATA])))
    assert "postgresql" in buf.getvalue()


def test_dump_stream_multi_doc(benchmark):
    buf = io.StringIO()
    yaml = pyrs_yaml.YAML()
    docs = [{"doc": i, "payload": "x" * 100} for i in range(500)]
    benchmark(lambda: (buf.seek(0), yaml.dump_stream(buf, docs)))
    assert len(list(pyrs_yaml.safe_loads(buf.getvalue()))) == 500


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


# ── Parse decomposition: parse -> YamlDocument vs to_dict vs safe_load ──
# These isolate the two dominant phases of safe_load:
#   (1) parse_with_options (Rust, GIL-released) + YamlDocument construction
#   (2) node_to_pyobject_* (Python object conversion, GIL held)
# Comparing test_parse_only_sized + test_document_to_dict_sized against
# test_safe_load_sized reveals the boundary + tag-resolution overhead.


@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_parse_only_sized(benchmark, size):
    """Parse into a YamlDocument only — no Python dict/list conversion."""
    result = benchmark(pyrs_yaml.parse, YAML_INPUTS[size])
    assert isinstance(result, pyrs_yaml.YamlDocument)


@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_document_to_dict_sized(benchmark, size):
    """to_dict on a pre-parsed YamlDocument — Python object conversion only."""
    doc = pyrs_yaml.parse(YAML_INPUTS[size])
    result = benchmark(doc.to_dict)
    assert result is not None


# ── YAML Schema Language benchmarks ──

HEX_SCHEMA_YAML = """\
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
  - pattern: ^\\d{4}-\\d{2}-\\d{2}$
    type: str
"""


def test_safe_load_custom_schema(benchmark):
    """safe_load with a registered custom schema (RuleResolver path)."""
    pyrs_yaml.register_schema("bench_hex", HEX_SCHEMA_YAML)
    result = benchmark(pyrs_yaml.safe_load, CONFIG_YAML, "bench_hex")
    assert result["server"]["port"] == 8080


def test_safe_load_custom_schema_vs_core(benchmark):
    """Core schema baseline — should be faster than custom schema."""
    result = benchmark(pyrs_yaml.safe_load, CONFIG_YAML, "core")
    assert result["server"]["port"] == 8080


# ── Community Plugin benchmarks ──


class BenchTimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj):
        return obj.isoformat()


def test_safe_dump_custom_type(benchmark):
    """safe_dump with a registered CustomType (isinstance + to_yaml path)."""
    pyrs_yaml.register_type("!bench_ts", BenchTimestampType())
    data = {"ts": datetime(2026, 8, 11, 10, 30, tzinfo=timezone.utc)}
    result = benchmark(pyrs_yaml.safe_dump, data)
    assert "!bench_ts" in result


def test_safe_dump_plain_dict_baseline(benchmark):
    """Plain dict dump baseline — no CustomType overhead."""
    data = {"name": "x", "count": 3, "flag": True}
    result = benchmark(pyrs_yaml.safe_dump, data)
    assert "name: x" in result
