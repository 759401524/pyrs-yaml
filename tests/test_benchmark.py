"""
Benchmark: pyrs-yaml vs PyYAML vs ruamel.yaml
Uses pytest-benchmark for statistical timing.
"""

import io

import pyrs_yaml
import pytest
import yaml as pyyaml
from ruamel.yaml import YAML

_ruamel_yaml = YAML()


def ruamel_load(s):
    return _ruamel_yaml.load(s)


def ruamel_dump(data):
    stream = io.StringIO()
    _ruamel_yaml.dump(data, stream)
    return stream.getvalue()


SMALL_YAML = """
# Application config
app:
  name: pyrs-yaml
  version: 0.2.0
  debug: false
  log_level: info
"""

MEDIUM_YAML = """
# Server configuration
server:
  host: 0.0.0.0
  port: 8080
  ssl: true
  workers: 4

database:
  type: postgresql
  host: db.example.com
  port: 5432
  name: myapp
  pool:
    min_size: 5
    max_size: 20
    timeout: 30

logging:
  level: INFO
  format: "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
  handlers:
    console:
      class: logging.StreamHandler
      stream: sys.stdout
    file:
      class: logging.FileHandler
      filename: app.log
      max_bytes: 10485760

features:
  authentication:
    enabled: true
    provider: oauth2
    token_expiry: 3600
  rate_limiting:
    enabled: true
    requests_per_minute: 100
  caching:
    enabled: true
    ttl: 300
"""

LARGE_YAML = """
# Complex application configuration with all YAML features
---
metadata:
  title: "YAML Test Suite — Large Config"
  version: 1.0
  author:
    name: Test User
    email: test@example.com
  tags:
    - production
    - configuration
    - benchmark

# Anchors and aliases
defaults: &defaults
  timeout: 30
  retries: 3
  backoff: exponential

services:
  api:
    <<: *defaults
    port: 8080
    endpoints:
      - path: /api/v1/users
        methods: [GET, POST]
      - path: /api/v1/users/{id}
        methods: [GET, PUT, DELETE]
      - path: /api/v1/orders
        methods: [GET, POST, PATCH]

  worker:
    <<: *defaults
    concurrency: 8
    queue:
      type: redis
      url: redis://localhost:6379/0

# Block scalars
description: |
  This is a literal block scalar that
  preserves newlines and formatting.
  It's used for multi-line strings.

formatted: >
  This is a folded block scalar that
  converts newlines to spaces.
  Useful for wrapping long text.

chomped: |+
  Keep all trailing newlines


stripped: |-
  Remove all trailing newlines

# Flow collections
flow_mapping: {key: value, another: 42}
flow_sequence: [1, 2, 3, 4, 5]

# Special values
null_value: null
empty_value: ~
boolean: true
float_value: 3.14159
integer: 42
octal: 0o77
hexadecimal: 0xFF
scientific: 1.23e-4
infinity: .inf
nan: .nan

# Tags
explicit_string: !!str 123
explicit_int: !!int 0xFF
explicit_bool: !!bool yes
explicit_null: !!null ~

# Comments everywhere
database:  # main database connection
  host: localhost
  port: 5432
  name: mydb
  credentials:  # authentication info
    username: admin
    password: secret123

cache:
  backend: redis
  url: "redis://localhost:6379/1"
  key_prefix: "app:"
  serializers:
    - pickle
    - json

monitoring:
  enabled: true
  metrics:
    - cpu_usage
    - memory_usage
    - disk_usage
    - network_io
    - request_count
    - error_count
  tags:
    environment: production
    region: us-east-1
    team: platform
"""

BLOCK_STYLE_YAML = (
    "key1: value1\n"
    "key2: value2\n"
    "nested:\n"
    "  subkey1: subvalue1\n"
    "  subkey2: subvalue2\n"
    "list:\n"
    "  - item1\n"
    "  - item2\n"
    "  - item3\n"
)


# ── pyrs-yaml benchmarks ──


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyrs_yaml_parse_small(benchmark):
    benchmark(pyrs_yaml.parse, SMALL_YAML)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyrs_yaml_parse_medium(benchmark):
    benchmark(pyrs_yaml.parse, MEDIUM_YAML)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyrs_yaml_parse_large(benchmark):
    benchmark(pyrs_yaml.parse, LARGE_YAML)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyrs_yaml_serialize_small(benchmark):
    doc = pyrs_yaml.parse(SMALL_YAML)
    benchmark(doc.to_yaml)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyrs_yaml_serialize_medium(benchmark):
    doc = pyrs_yaml.parse(MEDIUM_YAML)
    benchmark(doc.to_yaml)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyrs_yaml_serialize_large(benchmark):
    doc = pyrs_yaml.parse(LARGE_YAML)
    benchmark(doc.to_yaml)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyrs_yaml_roundtrip_small(benchmark):
    benchmark(lambda: pyrs_yaml.parse(SMALL_YAML).to_yaml())


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyrs_yaml_roundtrip_medium(benchmark):
    benchmark(lambda: pyrs_yaml.parse(MEDIUM_YAML).to_yaml())


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyrs_yaml_roundtrip_large(benchmark):
    benchmark(lambda: pyrs_yaml.parse(LARGE_YAML).to_yaml())


# ── PyYAML benchmarks ──


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyyaml_parse_small(benchmark):
    benchmark(pyyaml.safe_load, SMALL_YAML)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyyaml_parse_medium(benchmark):
    benchmark(pyyaml.safe_load, MEDIUM_YAML)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyyaml_parse_large(benchmark):
    benchmark(pyyaml.safe_load, LARGE_YAML)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyyaml_serialize_small(benchmark):
    data = pyyaml.safe_load(SMALL_YAML)
    benchmark(pyyaml.safe_dump, data)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyyaml_serialize_medium(benchmark):
    data = pyyaml.safe_load(MEDIUM_YAML)
    benchmark(pyyaml.safe_dump, data)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_pyyaml_serialize_large(benchmark):
    data = pyyaml.safe_load(LARGE_YAML)
    benchmark(pyyaml.safe_dump, data)


# ── ruamel.yaml benchmarks ──


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_ruamel_parse_small(benchmark):
    benchmark(ruamel_load, SMALL_YAML)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_ruamel_parse_medium(benchmark):
    benchmark(ruamel_load, MEDIUM_YAML)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_ruamel_parse_large(benchmark):
    benchmark(ruamel_load, LARGE_YAML)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_ruamel_serialize_small(benchmark):
    data = ruamel_load(SMALL_YAML)
    benchmark(ruamel_dump, data)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_ruamel_serialize_medium(benchmark):
    data = ruamel_load(MEDIUM_YAML)
    benchmark(ruamel_dump, data)


@pytest.mark.benchmark(min_rounds=100, warmup=True)
def test_ruamel_serialize_large(benchmark):
    data = ruamel_load(LARGE_YAML)
    benchmark(ruamel_dump, data)


# ── Speedup assertion ──


def test_pyrs_yaml_faster_than_pyyaml():
    """Assert pyrs-yaml serialize is faster than PyYAML on block-style YAML."""
    import time

    doc = pyrs_yaml.parse(BLOCK_STYLE_YAML)
    data = pyyaml.safe_load(BLOCK_STYLE_YAML)

    t0 = time.perf_counter()
    for _ in range(100):
        doc.to_yaml()
    pyr_time = time.perf_counter() - t0

    t0 = time.perf_counter()
    for _ in range(100):
        pyyaml.safe_dump(data)
    pyy_time = time.perf_counter() - t0

    assert pyr_time < pyy_time, f"pyrs-yaml slower than PyYAML: {pyr_time:.4f}s vs {pyy_time:.4f}s"
