"""
Benchmarks for the public Python API of pyrs-yaml.

Complements tests/test_benchmark_crosslib.py (which compares parse/serialize against
PyYAML and ruamel.yaml) by covering the rest of the surface exposed to Python:
safe_load, safe_loads, safe_dump, from_dict, from_json,
YamlDocument.to_json, YamlDocument.validate, YamlDocument.reparse,
parse_all_docs and parse_stream.
"""

import io
import math
from datetime import datetime, timezone

import pytest

import pyrs_yaml
from tests.data.yaml_samples import (
    BENCHMARK_ANCHOR as ANCHOR_YAML,
)
from tests.data.yaml_samples import (
    BENCHMARK_BLOCK_SCALARS,
    BENCHMARK_JSON5_NUMBERS,
    BENCHMARK_LARGE,
    BENCHMARK_SMALL,
    BENCHMARK_TOML_HOT,
    BENCHMARK_YAML_COMMENTS,
)
from tests.data.yaml_samples import (
    BENCHMARK_CONFIG_DATA as CONFIG_DATA,
)
from tests.data.yaml_samples import (
    BENCHMARK_CONFIG_JSON as CONFIG_JSON,
)
from tests.data.yaml_samples import (
    BENCHMARK_CONFIG_JSON5 as CONFIG_JSON5,
)
from tests.data.yaml_samples import (
    BENCHMARK_CONFIG_JSON_BLOCK_COMMENTS as CONFIG_JSON_BLOCK_COMMENTS,
)
from tests.data.yaml_samples import (
    BENCHMARK_CONFIG_JSON_ESCAPES as CONFIG_JSON_ESCAPES,
)
from tests.data.yaml_samples import (
    BENCHMARK_CONFIG_JSON_FLOATS as CONFIG_JSON_FLOATS,
)
from tests.data.yaml_samples import (
    BENCHMARK_CONFIG_JSON_LARGE as CONFIG_JSON_LARGE,
)
from tests.data.yaml_samples import (
    BENCHMARK_CONFIG_JSONC as CONFIG_JSONC,
)
from tests.data.yaml_samples import (
    BENCHMARK_CONFIG_TOML as CONFIG_TOML,
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


def test_from_jsonc(benchmark):
    result = benchmark(pyrs_yaml.from_jsonc, CONFIG_JSONC)
    assert "server" in result


def test_from_json5(benchmark):
    result = benchmark(pyrs_yaml.from_json5, CONFIG_JSON5)
    assert "server" in result


def test_load_json5(benchmark):
    result = benchmark(pyrs_yaml.load_json5, CONFIG_JSON5)
    assert result["server"]["port"] == 8080


def test_document_to_jsonc(benchmark):
    doc = pyrs_yaml.parse(CONFIG_YAML)
    result = benchmark(doc.to_jsonc)
    # A document-level comment is now emitted first, so don't assume the
    # payload starts at `{`; assert the object closes it instead.
    assert result.rstrip().endswith("}")
    assert '"server"' in result


def test_document_to_json5(benchmark):
    doc = pyrs_yaml.parse(CONFIG_YAML)
    result = benchmark(doc.to_json5)
    assert result.rstrip().endswith("}")
    assert '"server"' in result


# ── TOML native-kernel benchmarks ──
# The TOML spoke (from_toml / load_toml / to_toml) had no benchmark coverage;
# these pin the core paths so CodSpeed tracks regressions and the cross-library
# comparison in test_benchmark_crosslib.py has a peer.


def test_from_toml(benchmark):
    result = benchmark(pyrs_yaml.from_toml, CONFIG_TOML)
    assert "server:" in result


def test_load_toml(benchmark):
    result = benchmark(pyrs_yaml.load_toml, CONFIG_TOML)
    assert result["server"]["port"] == 8080
    assert len(result["items"]) == 50


def test_to_toml(benchmark):
    # Setup (TOML -> YAML) is outside the measured call; this times only the
    # YAML -> TOML writer (node_to_toml hot path).
    yaml_str = pyrs_yaml.from_toml(CONFIG_TOML)
    result = benchmark(pyrs_yaml.to_toml, yaml_str)
    assert "[server]" in result


# ── hot-spot samples (objective pillar 2.5) ──
# Block scalars / comment scanning / TOML multiline+radix+datetime / JSON5-only
# number spellings. No other benchmark input reaches these parser and writer
# branches, so changes there would be invisible to CodSpeed tracking.


def test_safe_load_block_scalars(benchmark):
    result = benchmark(pyrs_yaml.safe_load, BENCHMARK_BLOCK_SCALARS)
    assert result["key_0"].startswith("Line one")


def test_to_yaml_block_scalars(benchmark):
    # Writer side: Literal/Folded styles must survive re-serialization.
    doc = pyrs_yaml.parse(BENCHMARK_BLOCK_SCALARS)
    out = benchmark(doc.to_yaml)
    assert "|" in out and ">" in out


def test_safe_load_comments(benchmark):
    result = benchmark(pyrs_yaml.safe_load, BENCHMARK_YAML_COMMENTS)
    assert result["key_0"] == "value_0"


def test_load_toml_hot(benchmark):
    # Multi-line strings, comments, radix integers, underscores, exponent
    # floats and datetimes in one document.
    result = benchmark(pyrs_yaml.load_toml, BENCHMARK_TOML_HOT)
    assert result["numbers"]["hex"] == 0xDEADBEEF
    assert result["server"]["motd"] == "Welcome\nto the machine\n"


def test_to_toml_hot(benchmark):
    yaml_str = pyrs_yaml.from_toml(BENCHMARK_TOML_HOT)
    out = benchmark(pyrs_yaml.to_toml, yaml_str)
    # Radix spelling is preserved verbatim by the fidelity contract.
    assert "0xDEADBEEF" in out


def test_load_json5_numbers(benchmark):
    result = benchmark(pyrs_yaml.load_json5, BENCHMARK_JSON5_NUMBERS)
    assert result["hex"] == 0xDEADBEEF
    assert result["pos"] == 7
    assert result["lead"] == 0.5
    assert math.isinf(result["inf"]) and result["inf"] > 0
    assert math.isinf(result["ninf"]) and result["ninf"] < 0
    assert math.isnan(result["nan"])


def test_to_json5_numbers(benchmark):
    # Writer side: JSON5-only spellings emit verbatim through the hub.
    doc = pyrs_yaml.parse(pyrs_yaml.from_json5(BENCHMARK_JSON5_NUMBERS))
    out = benchmark(doc.to_json5)
    assert "0xDEADBEEF" in out and "Infinity" in out


def test_load_jsonc_large(benchmark):
    """Larger JSON payload through the JSONC loader (parses plain JSON)."""
    result = benchmark(pyrs_yaml.load_jsonc, CONFIG_JSON_LARGE)
    assert result["server"]["workers"] == 4
    assert len(result["items"]) == 50


def test_load_jsonc_floats(benchmark):
    """Float-bearing JSON through the fast path (exercises the float branch).

    Every numeric leaf here is a float, so this is the only load benchmark that
    reaches ``json_fast``'s float handling; without it a float fast-path change
    would be invisible to CodSpeed regression tracking.
    """
    result = benchmark(pyrs_yaml.load_jsonc, CONFIG_JSON_FLOATS)
    assert result["thresholds"]["cpu"] == 0.75
    assert len(result["metrics"]) == 50


def test_load_jsonc_escapes(benchmark):
    """Escape-bearing JSON through the fast path (exercises the escape decoder).

    Simple `\n`/`\t`/`\\` strings are decoded inline on the fast path; without
    this benchmark the escape branch would be unmeasured (the same "not
    reflected" gap the float sample closed).
    """
    result = benchmark(pyrs_yaml.load_jsonc, CONFIG_JSON_ESCAPES)
    assert result["logs"][0]["msg"] == "line\n0\tvalue"
    assert len(result["logs"]) == 50


# ── Strict `load_json` mirrors ──
# The three benches above exercise `load_jsonc` on canonical-strict payloads,
# which is byte-identical to what `load_json` sees (same `json_fast::try_load`
# scanner). Pinning the strict loader keeps its binding-level overhead tracked
# and guarantees that a future divergence (e.g. widening the fast path or
# routing the fallback through `from_jsonc`) shows up as a regression on both
# sides independently, not just one.


def test_load_json_large(benchmark):
    """Larger strict-JSON payload through the strict loader."""
    result = benchmark(pyrs_yaml.load_json, CONFIG_JSON_LARGE)
    assert result["server"]["workers"] == 4
    assert len(result["items"]) == 50


def test_load_json_floats(benchmark):
    """Float-bearing strict JSON through the loader (float fast branch)."""
    result = benchmark(pyrs_yaml.load_json, CONFIG_JSON_FLOATS)
    assert result["thresholds"]["cpu"] == 0.75
    assert len(result["metrics"]) == 50


def test_load_json_escapes(benchmark):
    """Escape-bearing strict JSON through the loader (escape decoder branch)."""
    result = benchmark(pyrs_yaml.load_json, CONFIG_JSON_ESCAPES)
    assert result["logs"][0]["msg"] == "line\n0\tvalue"
    assert len(result["logs"]) == 50


def test_load_jsonc_block_comments(benchmark):
    """Block-comment-dense JSONC through the tokenizer.

    Objective §测试覆盖 5 explicitly names "block-comment" as a hot sample
    that must be quantified per change. Only inline ``//`` comments were
    previously measured (via ``test_load_jsonc`` on ``CONFIG_JSONC``);
    the ``/* ... */`` branch -- both standalone and trailing-inline shapes
    -- was unbenchmarked. This closes that gap; a scanner regression on
    the block path now shows up as a CodSpeed delta.
    """
    result = benchmark(pyrs_yaml.load_jsonc, CONFIG_JSON_BLOCK_COMMENTS)
    assert len(result) == 50
    assert result["k0"]["v"] == 0
    assert result["k49"]["v"] == 49


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
