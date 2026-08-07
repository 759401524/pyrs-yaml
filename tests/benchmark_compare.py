"""Benchmark: pyrs-yaml vs lava-sh/yaml-rs.

Run:  uv run pytest tests/benchmark_compare.py -v --benchmark-only
Save: uv run pytest tests/benchmark_compare.py -v --benchmark-only --benchmark-save=compare
"""

import gc
import os

import pytest


def _generate_large_yaml(lines: int) -> str:
    parts = ["# Large generated YAML\n"]
    for i in range(lines):
        parts.append(f"key_{i:06d}: value_{i}\n")
        if i % 10 == 0:
            parts.append(f"# comment for key_{i:06d}\n")
        if i % 100 == 0:
            parts.append(f"group_{i:06d}:\n  subkey: nested_value_{i}\n")
    return "".join(parts)


def _generate_complex_yaml(lines: int) -> str:
    parts = ["# Complex YAML with mixed features\n"]
    for i in range(lines):
        if i % 20 == 0:
            parts.append(f"anchored_{i:06d}: &anchor_{i:06d}\n  value: data_{i}\n")
        elif i % 15 == 0:
            parts.append(f"tagged_{i:06d}: !!str tagged_value_{i}\n")
        elif i % 10 == 0:
            parts.append(f"block_{i:06d}: |\n  block content line 1\n  block content line 2\n")
        elif i % 5 == 0:
            parts.append(f"folded_{i:06d}: >\n  folded content that\n  wraps and folds\n")
        else:
            parts.append(f"key_{i:06d}: value_{i}  # inline\n")
    return "".join(parts)


SMALL_YAML = "key: value\nname: test\ncount: 1\n"

MEDIUM_YAML = """server:
  host: localhost
  port: 8080
  timeout: 30
  debug: false

database:
  driver: postgres
  host: db.example.com
  port: 5432
  name: myapp
  pool_size: 10
  settings:
    max_connections: 100
    idle_timeout: 300
    ssl: true

logging:
  level: info
  format: json
  outputs:
    - stdout
    - file: /var/log/app.log

features:
  auth: true
  cache: true
  rate_limit: false
  experimental:
    - beta_feature_1
    - beta_feature_2

metadata:
  version: 2.1.0
  environment: production
  deployed_at: 2024-01-15T12:00:00Z
  tags: [deploy, production, v2]
"""

LARGE_YAML = _generate_large_yaml(10000)
COMPLEX_YAML = _generate_complex_yaml(5000)


def _peak_memory_mb() -> float:
    import psutil

    return psutil.Process(os.getpid()).memory_info().rss / 1024 / 1024


def _bench(benchmark, fn, *args, **kwargs):
    gc.collect()
    mem_before = _peak_memory_mb()
    result = benchmark(fn, *args, **kwargs)
    gc.collect()
    mem_after = _peak_memory_mb()
    benchmark.extra_info["mem_mb"] = round(mem_after - mem_before, 2)
    return result


@pytest.fixture(scope="session")
def pyrs():
    import pyrs_yaml

    return pyrs_yaml


@pytest.fixture(scope="session")
def yrs():
    import yaml_rs

    return yaml_rs


@pytest.fixture(scope="session")
def pyrs_docs(pyrs):
    return {
        "small": pyrs.parse(SMALL_YAML),
        "medium": pyrs.parse(MEDIUM_YAML),
        "large": pyrs.parse(LARGE_YAML),
        "complex": pyrs.parse(COMPLEX_YAML),
    }


@pytest.fixture(scope="session")
def yrs_docs(yrs):
    return {
        "small": yrs.loads(SMALL_YAML),
        "medium": yrs.loads(MEDIUM_YAML),
        "large": yrs.loads(LARGE_YAML),
        "complex": yrs.loads(COMPLEX_YAML),
    }


# ── Parse benchmarks ──


def test_parse_small_pyrs(benchmark, pyrs):
    _bench(benchmark, pyrs.parse, SMALL_YAML)


def test_parse_small_yrs(benchmark, yrs):
    _bench(benchmark, yrs.loads, SMALL_YAML)


def test_parse_medium_pyrs(benchmark, pyrs):
    _bench(benchmark, pyrs.parse, MEDIUM_YAML)


def test_parse_medium_yrs(benchmark, yrs):
    _bench(benchmark, yrs.loads, MEDIUM_YAML)


def test_parse_large_pyrs(benchmark, pyrs):
    _bench(benchmark, pyrs.parse, LARGE_YAML)


def test_parse_large_yrs(benchmark, yrs):
    _bench(benchmark, yrs.loads, LARGE_YAML)


def test_parse_complex_pyrs(benchmark, pyrs):
    _bench(benchmark, pyrs.parse, COMPLEX_YAML)


def test_parse_complex_yrs(benchmark, yrs):
    _bench(benchmark, yrs.loads, COMPLEX_YAML)


# ── Dump/Serialize benchmarks ──


def test_dump_small_pyrs(benchmark, pyrs_docs):
    doc = pyrs_docs["small"]
    _bench(benchmark, doc.to_yaml)


def test_dump_small_yrs(benchmark, yrs_docs, yrs):
    _bench(benchmark, yrs.dumps, yrs_docs["small"])


def test_dump_medium_pyrs(benchmark, pyrs_docs):
    _bench(benchmark, pyrs_docs["medium"].to_yaml)


def test_dump_medium_yrs(benchmark, yrs_docs, yrs):
    _bench(benchmark, yrs.dumps, yrs_docs["medium"])


def test_dump_large_pyrs(benchmark, pyrs_docs):
    _bench(benchmark, pyrs_docs["large"].to_yaml)


def test_dump_large_yrs(benchmark, yrs_docs, yrs):
    _bench(benchmark, yrs.dumps, yrs_docs["large"])


def test_dump_complex_pyrs(benchmark, pyrs_docs):
    _bench(benchmark, pyrs_docs["complex"].to_yaml)


def test_dump_complex_yrs(benchmark, yrs_docs, yrs):
    _bench(benchmark, yrs.dumps, yrs_docs["complex"])


# ── Roundtrip benchmarks (parse + dump) ──


def test_roundtrip_small_pyrs(benchmark, pyrs):
    def _():
        doc = pyrs.parse(SMALL_YAML)
        doc.to_yaml()

    _bench(benchmark, _)


def test_roundtrip_small_yrs(benchmark, yrs):
    def _():
        doc = yrs.loads(SMALL_YAML)
        yrs.dumps(doc)

    _bench(benchmark, _)


def test_roundtrip_medium_pyrs(benchmark, pyrs):
    def _():
        doc = pyrs.parse(MEDIUM_YAML)
        doc.to_yaml()

    _bench(benchmark, _)


def test_roundtrip_medium_yrs(benchmark, yrs):
    def _():
        doc = yrs.loads(MEDIUM_YAML)
        yrs.dumps(doc)

    _bench(benchmark, _)


def test_roundtrip_large_pyrs(benchmark, pyrs):
    def _():
        doc = pyrs.parse(LARGE_YAML)
        doc.to_yaml()

    _bench(benchmark, _)


def test_roundtrip_large_yrs(benchmark, yrs):
    def _():
        doc = yrs.loads(LARGE_YAML)
        yrs.dumps(doc)

    _bench(benchmark, _)


# ── Access benchmarks ──


def test_access_key_pyrs(benchmark, pyrs_docs):
    doc = pyrs_docs["large"]
    _bench(benchmark, lambda: doc["key_000000"])


def test_access_key_yrs(benchmark, yrs_docs):
    doc = yrs_docs["large"]
    _bench(benchmark, lambda: doc["key_000000"])


def test_access_nested_pyrs(benchmark, pyrs_docs):
    doc = pyrs_docs["large"]
    _bench(benchmark, lambda: doc["key_000001"])


def test_access_nested_yrs(benchmark, yrs_docs):
    doc = yrs_docs["large"]
    _bench(benchmark, lambda: doc["group_000000"]["subkey"])
