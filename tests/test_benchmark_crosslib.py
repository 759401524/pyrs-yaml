"""
Cross-library comparison: pyrs-yaml vs PyYAML vs ruamel.yaml vs ryaml vs yaml_edit vs yaml_rs.

Run with: pytest tests/test_benchmark_crosslib.py --codspeed
"""

import importlib.metadata
import io
import json

import pytest

import pyrs_yaml
from tests.data.yaml_samples import (
    BENCHMARK_BLOCK_STYLE as BLOCK_STYLE_YAML,
)
from tests.data.yaml_samples import (
    BENCHMARK_LARGE as LARGE_YAML,
)
from tests.data.yaml_samples import (
    BENCHMARK_MEDIUM as MEDIUM_YAML,
)
from tests.data.yaml_samples import (
    BENCHMARK_SMALL as SMALL_YAML,
)

try:
    import yaml as pyyaml

    HAS_PYYAML = True
except ImportError:
    HAS_PYYAML = False
    pyyaml = None

try:
    import ruamel.yaml as ruamel_yaml
    from ruamel.yaml import YAML

    HAS_RUAMEL = True
    _ruamel_yaml = YAML()
except ImportError:
    HAS_RUAMEL = False
    ruamel_yaml = None
    _ruamel_yaml = None

try:
    import ryaml

    HAS_RYAML = True
except ImportError:
    HAS_RYAML = False
    ryaml = None

try:
    import yaml_edit

    HAS_YAML_EDIT = True
except ImportError:
    HAS_YAML_EDIT = False
    yaml_edit = None

try:
    import yaml_rs

    HAS_YAML_RS = True
except ImportError:
    HAS_YAML_RS = False
    yaml_rs = None


def ruamel_load(s):
    return _ruamel_yaml.load(s)


def ruamel_dump(data):
    stream = io.StringIO()
    _ruamel_yaml.dump(data, stream)
    return stream.getvalue()


def ryaml_dumps(data):
    if HAS_RYAML:
        return ryaml.dumps(data)
    return ""


YAML_INPUTS = {"small": SMALL_YAML, "medium": MEDIUM_YAML, "large": LARGE_YAML}
SIZES = ["small", "medium", "large"]


# ── pyrs-yaml benchmarks ──


@pytest.mark.benchmark(group="pyrs-yaml")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_pyrs_yaml_parse(benchmark, size):
    benchmark(pyrs_yaml.parse, YAML_INPUTS[size])


@pytest.mark.benchmark(group="pyrs-yaml")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_pyrs_yaml_serialize(benchmark, size):
    doc = pyrs_yaml.parse(YAML_INPUTS[size])
    benchmark(doc.to_yaml)


@pytest.mark.benchmark(group="pyrs-yaml")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_pyrs_yaml_roundtrip(benchmark, size):
    benchmark(lambda y=YAML_INPUTS[size]: pyrs_yaml.parse(y).to_yaml())


# ── PyYAML benchmarks ──


@pytest.mark.benchmark(group="pyyaml")
@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_pyyaml_parse(benchmark, size):
    benchmark(pyyaml.safe_load, YAML_INPUTS[size])


@pytest.mark.benchmark(group="pyyaml")
@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_pyyaml_serialize(benchmark, size):
    data = pyyaml.safe_load(YAML_INPUTS[size])
    benchmark(pyyaml.safe_dump, data)


# ── ruamel.yaml benchmarks ──


@pytest.mark.benchmark(group="ruamel")
@pytest.mark.skipif(not HAS_RUAMEL, reason="ruamel.yaml not installed")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_ruamel_parse(benchmark, size):
    benchmark(ruamel_load, YAML_INPUTS[size])


@pytest.mark.benchmark(group="ruamel")
@pytest.mark.skipif(not HAS_RUAMEL, reason="ruamel.yaml not installed")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_ruamel_serialize(benchmark, size):
    data = ruamel_load(YAML_INPUTS[size])
    benchmark(ruamel_dump, data)


# ── ryaml benchmarks ──


@pytest.mark.benchmark(group="ryaml")
@pytest.mark.skipif(not HAS_RYAML, reason="ryaml not installed")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_ryaml_parse(benchmark, size):
    benchmark(ryaml.load, io.StringIO(YAML_INPUTS[size]))


@pytest.mark.benchmark(group="ryaml")
@pytest.mark.skipif(not HAS_RYAML, reason="ryaml not installed")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_ryaml_serialize(benchmark, size):
    data = ryaml.load(io.StringIO(YAML_INPUTS[size]))
    benchmark(ryaml_dumps, data)


# ── yaml_edit benchmarks (parse only) ──


@pytest.mark.benchmark(group="yaml_edit")
@pytest.mark.skipif(not HAS_YAML_EDIT, reason="yaml_edit not installed")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_yaml_edit_parse(benchmark, size):
    benchmark(yaml_edit.Document.parse, YAML_INPUTS[size])


# ── yaml_rs benchmarks (Rust competitor) ──


@pytest.mark.benchmark(group="yaml_rs")
@pytest.mark.skipif(not HAS_YAML_RS, reason="yaml_rs not installed")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_yaml_rs_parse(benchmark, size):
    benchmark(yaml_rs.loads, YAML_INPUTS[size])


@pytest.mark.benchmark(group="yaml_rs")
@pytest.mark.skipif(not HAS_YAML_RS, reason="yaml_rs not installed")
@pytest.mark.parametrize("size", SIZES, ids=SIZES)
def test_yaml_rs_serialize(benchmark, size):
    data = yaml_rs.loads(YAML_INPUTS[size])
    benchmark(yaml_rs.dumps, data)


# ── Speedup assertion ──


@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
def test_pyrs_yaml_faster_than_pyyaml():
    doc = pyrs_yaml.parse(BLOCK_STYLE_YAML)
    data = pyyaml.safe_load(BLOCK_STYLE_YAML)

    import time

    t0 = time.perf_counter()
    for _ in range(100):
        doc.to_yaml()
    pyr_time = time.perf_counter() - t0

    t0 = time.perf_counter()
    for _ in range(100):
        pyyaml.safe_dump(data)
    pyy_time = time.perf_counter() - t0

    assert pyr_time < pyy_time, f"pyrs-yaml slower than PyYAML: {pyr_time:.4f}s vs {pyy_time:.4f}s"


# ── Feature comparison report ──


def test_feature_comparison():
    print_report()


def print_report(results=None):
    print("=" * 75)
    print("  pyrs-yaml vs PyYAML vs ruamel.yaml vs ryaml vs yaml_edit vs yaml_rs — Feature Comparison")
    print("=" * 75)
    print(f"  Python:   {__import__('sys').version.split()[0]}")
    print(f"  pyrs-yaml: {pyrs_yaml.__version__}")
    print(f"  PyYAML:    {pyyaml.__version__ if HAS_PYYAML else '(not installed)'}")
    print(f"  ruamel:    {ruamel_yaml.__version__ if HAS_RUAMEL else '(not installed)'}")
    if HAS_RYAML:
        try:
            print(f"  ryaml:     {importlib.metadata.version('ryaml')}")
        except importlib.metadata.PackageNotFoundError:
            print("  ryaml:     (not installed)")
    if HAS_YAML_EDIT:
        try:
            print(f"  yaml_edit: {importlib.metadata.version('yaml-edit')}")
        except importlib.metadata.PackageNotFoundError:
            print("  yaml_edit: (not installed)")
    if HAS_YAML_RS:
        try:
            print(f"  yaml_rs:   {importlib.metadata.version('yaml-rs')}")
        except importlib.metadata.PackageNotFoundError:
            print("  yaml_rs:   (not installed)")
    print()

    print("─" * 75)
    print("  ROUND-TRIP PRESERVATION TEST")
    print("─" * 75)

    rt_yaml = LARGE_YAML

    out1 = pyrs_yaml.parse(rt_yaml).to_yaml()
    print(
        f"  pyrs-yaml:     comments={'# Comments everywhere' in out1}  anchors={'&defaults' in out1}  tags={'!!str' in out1}"
    )

    if HAS_PYYAML:
        out2 = pyyaml.safe_dump(pyyaml.safe_load(rt_yaml))
        print(
            f"  PyYAML:        comments={'# Comments everywhere' in out2}  anchors={'&defaults' in out2}  tags={'!!str' in out2}"
        )
    else:
        print("  PyYAML:        (not installed)")

    if HAS_RUAMEL:
        out3 = ruamel_dump(ruamel_load(rt_yaml))
        print(
            f"  ruamel.yaml:   comments={'# Comments everywhere' in out3}  anchors={'&defaults' in out3}  tags={'!!str' in out3}"
        )
    else:
        print("  ruamel.yaml:   (not installed)")

    if HAS_RYAML:
        ryaml_buf = io.StringIO(rt_yaml)
        ryaml_data = ryaml.load(ryaml_buf)
        out4 = ryaml.dumps(ryaml_data)
        print(
            f"  ryaml:         comments={'# Comments everywhere' in out4}  anchors={'&defaults' in out4}  tags={'!!str' in out4}"
        )

    if HAS_YAML_EDIT:
        try:
            doc = yaml_edit.Document()
            doc.parse(rt_yaml)
            print("  yaml_edit:     (parse only — no serialization API)")
        except Exception:
            print("  yaml_edit:     (parse error)")

    if HAS_YAML_RS:
        out5 = yaml_rs.dumps(yaml_rs.loads(rt_yaml))
        print(
            f"  yaml_rs:       comments={'# Comments everywhere' in out5}  anchors={'&defaults' in out5}  tags={'!!str' in out5}"
        )
    print()

    print("─" * 75)
    print("  FEATURE SUPPORT COMPARISON")
    print("─" * 75)

    features = [
        ("YAML 1.2 compliance", True, True, True, True, True, True),
        ("Comments (standalone)", True, False, True, False, False, True),
        ("Comments (inline)", True, False, True, False, False, True),
        ("Anchors/aliases", True, False, True, True, False, True),
        ("Tags (explicit)", True, False, True, True, False, True),
        ("Block scalars", True, True, True, True, True, True),
        ("Flow collections", True, True, True, True, True, True),
        ("Merge keys (<<)", True, False, True, False, False, True),
        ("Complex keys", True, True, True, True, True, True),
        ("Round-trip preservation", True, False, True, False, False, True),
        ("Python bindings", True, True, True, True, True, True),
        ("ABI3 (py3.9+)", True, False, False, False, False, False),
        ("Type stubs (.pyi)", True, True, False, False, False, True),
        ("i18n error messages", True, False, False, False, False, False),
    ]

    print(
        f"  {'Feature':35s} {'pyrs-yaml':12s} {'PyYAML':10s} {'ruamel':10s} {'ryaml':8s} {'yaml_edit':10s} {'yaml_rs':10s}"
    )
    print(f"  {'-' * 101}")

    def _mark(v: bool) -> str:
        return "✅" if v else "❌"

    for name, pyr, pyr2, ruamel, ryml, yedit, yrs in features:
        print(
            f"  {name:35s} {_mark(pyr):8s}     {_mark(pyr2):6s}     {_mark(ruamel):6s}     {_mark(ryml):6s}     {_mark(yedit):6s}     {_mark(yrs):6s}"
        )
    print()

    if results:
        print("=" * 75)
        print("  Performance Results (from pytest-codspeed JSON)")
        print("=" * 75)
        print(json.dumps(results, indent=2))
        print()

    print("=" * 75)
    print("  Report complete.")
    print("=" * 75)
