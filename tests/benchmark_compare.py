"""
Comprehensive benchmark: pyrs-yaml vs PyYAML vs ruamel.yaml vs ryaml
Uses pytest-codspeed for statistical timing (delegates to test_benchmark.py).
"""

import io
import json

import pyrs_yaml
import ruamel.yaml as ruamel_yaml
import yaml as pyyaml
from ruamel.yaml import YAML

try:
    import ryaml

    HAS_RYAML = True
except ImportError:
    HAS_RYAML = False
    ryaml = None  # type: ignore[assignment]


# ── Ruamel helpers ─────────────────────────────────────────────────────────

_ruamel_yaml = YAML()


def ruamel_load(s):
    return _ruamel_yaml.load(s)


def ruamel_dump(data):
    stream = io.StringIO()
    _ruamel_yaml.dump(data, stream)
    return stream.getvalue()


# ── Test data ──────────────────────────────────────────────────────────────

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


# ── Feature comparison report ──────────────────────────────────────────────


def print_report(results: dict | None = None):
    """Print a feature comparison and round-trip preservation report."""
    print("=" * 75)
    print("  pyrs-yaml vs PyYAML vs ruamel.yaml vs ryaml — Feature Comparison")
    print("=" * 75)
    print(f"  Python:   {__import__('sys').version.split()[0]}")
    print(f"  pyrs-yaml: {pyrs_yaml.__version__}")
    print(f"  PyYAML:    {pyyaml.__version__}")
    print(f"  ruamel:    {ruamel_yaml.__version__}")
    if HAS_RYAML:
        print(f"  ryaml:     {ryaml.__version__}")
    print()

    # ── Round-trip preservation test ─────────────────────────────────────
    print("─" * 75)
    print("  ROUND-TRIP PRESERVATION TEST")
    print("─" * 75)

    rt_yaml = LARGE_YAML

    out1 = pyrs_yaml.parse(rt_yaml).to_yaml()
    print(
        f"  pyrs-yaml:     comments={'# Comments everywhere' in out1}  anchors={'&defaults' in out1}  tags={'!!str' in out1}"
    )

    out2 = pyyaml.safe_dump(pyyaml.safe_load(rt_yaml))
    print(
        f"  PyYAML:        comments={'# Comments everywhere' in out2}  anchors={'&defaults' in out2}  tags={'!!str' in out2}"
    )

    out3 = ruamel_dump(ruamel_load(rt_yaml))
    print(
        f"  ruamel.yaml:   comments={'# Comments everywhere' in out3}  anchors={'&defaults' in out3}  tags={'!!str' in out3}"
    )

    if HAS_RYAML:
        ryaml_buf = io.StringIO(rt_yaml)
        ryaml_data = ryaml.load(ryaml_buf)
        out4 = ryaml.dumps(ryaml_data)
        print(
            f"  ryaml:         comments={'# Comments everywhere' in out4}  anchors={'&defaults' in out4}  tags={'!!str' in out4}"
        )
    print()

    # ── Feature support comparison ───────────────────────────────────────
    print("─" * 75)
    print("  FEATURE SUPPORT COMPARISON")
    print("─" * 75)

    features = [
        ("YAML 1.2 compliance", True, True, True, True),
        ("Comments (standalone)", True, False, True, False),
        ("Comments (inline)", True, False, True, False),
        ("Anchors/aliases", True, False, True, True),
        ("Tags (explicit)", True, False, True, True),
        ("Block scalars", True, True, True, True),
        ("Flow collections", True, True, True, True),
        ("Merge keys (<<)", True, False, True, False),
        ("Complex keys", True, True, True, True),
        ("Round-trip preservation", True, False, True, False),
        ("Python bindings", True, True, True, True),
        ("ABI3 (py3.9+)", True, False, False, False),
        ("Type stubs (.pyi)", True, True, False, False),
        ("i18n error messages", True, False, False, False),
    ]

    print(f"  {'Feature':35s} {'pyrs-yaml':12s} {'PyYAML':10s} {'ruamel':10s} {'ryaml':8s}")
    print(f"  {'-' * 79}")

    def _mark(v: bool) -> str:
        return "✅" if v else "❌"

    for name, pyr, pyr2, ruamel, ryml in features:
        print(f"  {name:35s} {_mark(pyr):8s}     {_mark(pyr2):6s}     {_mark(ruamel):6s}     {_mark(ryml):6s}")
    print()

    if results:
        print("=" * 75)
        print("  Performance Results (from pytest-codspeed JSON)")
        print("=" * 75)
        print(json.dumps(results, indent=2))
        print()

    print("=" * 75)
    print("  Report complete. Run `pytest tests/test_benchmark.py --codspeed`")
    print("  for statistical timing results across all libraries.")
    print("=" * 75)


# ── Standalone CLI entry point ──────────────────────────────────────────


if __name__ == "__main__":
    print_report()
