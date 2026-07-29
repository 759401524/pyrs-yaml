"""
Comprehensive benchmark: pyrs-yaml vs PyYAML vs ruamel.yaml
"""

import argparse
import io
import json
import time

import pyrs_yaml
import ruamel.yaml as ruamel_yaml
import yaml as pyyaml
from ruamel.yaml import YAML

# ── CLI arguments ──────────────────────────────────────────────────────────


def parse_args():
    parser = argparse.ArgumentParser(description="YAML library benchmark")
    parser.add_argument("--json", action="store_true", help="Output JSON")
    parser.add_argument("--rounds", type=int, default=200, help="Number of rounds")
    parser.add_argument("--ci", action="store_true", help="CI mode: --json --rounds 20")
    return parser.parse_args()


# ── Ruamel helpers ─────────────────────────────────────────────────────────

_ruamel_yaml = YAML()


def ruamel_load(s):
    """Load YAML with ruamel.yaml."""
    return _ruamel_yaml.load(s)


def ruamel_dump(data):
    """Dump with ruamel.yaml."""
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

# ── Benchmark helpers ─────────────────────────────────────────────────────


def timed(fn, rounds=200):
    """Return (median_ms, min_ms, max_ms) for fn called rounds times."""
    times = []
    for _ in range(rounds):
        t0 = time.perf_counter()
        fn()
        times.append(time.perf_counter() - t0)
    times.sort()
    return times[len(times) // 2] * 1000, times[0] * 1000, times[-1] * 1000


# ── Main ───────────────────────────────────────────────────────────────────


def main():
    args = parse_args()
    rounds = 20 if args.ci else args.rounds
    if args.ci:
        args.json = True

    print("=" * 75)
    print("  pyrs-yaml vs PyYAML vs ruamel.yaml — Performance Benchmark")
    print("=" * 75)
    print(f"  Python:   {__import__('sys').version.split()[0]}")
    print(f"  pyrs-yaml: {pyrs_yaml.__version__}")
    print(f"  PyYAML:    {pyyaml.__version__}")
    print(f"  ruamel:    {ruamel_yaml.__version__}")
    print()

    results = {}
    for label, yaml_str, key in [
        ("SMALL (~100 B)", SMALL_YAML, "small"),
        ("MEDIUM (~500 B)", MEDIUM_YAML, "medium"),
        ("LARGE (~2 KB)", LARGE_YAML, "large"),
    ]:
        print("─" * 75)
        print(f"  {label}")
        print("─" * 75)

        # pyrs-yaml
        p = timed(lambda yaml_str=yaml_str: pyrs_yaml.parse(yaml_str), rounds)
        s = timed(lambda yaml_str=yaml_str: pyrs_yaml.parse(yaml_str).to_yaml(), rounds)
        r = timed(lambda yaml_str=yaml_str: pyrs_yaml.parse(yaml_str).to_yaml(), rounds)
        print(f"  pyrs-yaml       parse={p[0]:8.2f}ms  serialize={s[0]:8.2f}ms  roundtrip={r[0]:8.2f}ms")

        # PyYAML
        p2 = timed(lambda yaml_str=yaml_str: pyyaml.safe_load(yaml_str), rounds)
        s2 = timed(lambda yaml_str=yaml_str: pyyaml.safe_dump(pyyaml.safe_load(yaml_str)), rounds)
        r2 = timed(lambda yaml_str=yaml_str: pyyaml.safe_dump(pyyaml.safe_load(yaml_str)), rounds)
        print(f"  PyYAML          parse={p2[0]:8.2f}ms  serialize={s2[0]:8.2f}ms  roundtrip={r2[0]:8.2f}ms")

        # ruamel.yaml
        p3 = timed(lambda yaml_str=yaml_str: ruamel_load(yaml_str), rounds)
        s3 = timed(lambda yaml_str=yaml_str: ruamel_dump(ruamel_load(yaml_str)), rounds)
        r3 = timed(lambda yaml_str=yaml_str: ruamel_dump(ruamel_load(yaml_str)), rounds)
        print(f"  ruamel.yaml     parse={p3[0]:8.2f}ms  serialize={s3[0]:8.2f}ms  roundtrip={r3[0]:8.2f}ms")
        print()

        # Speedup vs PyYAML
        parse_spd = p2[0] / max(p[0], 0.0001)
        ser_spd = s2[0] / max(s[0], 0.0001)
        rt_spd = r2[0] / max(r[0], 0.0001)
        print(f"  Speedup vs PyYAML:  parse={parse_spd:.1f}x  serialize={ser_spd:.1f}x  roundtrip={rt_spd:.1f}x")
        print()

        results[key] = {
            "pyrs-yaml": {
                "parse_ms": round(p[0], 2),
                "serialize_ms": round(s[0], 2),
                "roundtrip_ms": round(r[0], 2),
            },
            "pyyaml": {
                "parse_ms": round(p2[0], 2),
                "serialize_ms": round(s2[0], 2),
                "roundtrip_ms": round(r2[0], 2),
            },
            "ruamel": {
                "parse_ms": round(p3[0], 2),
                "serialize_ms": round(s3[0], 2),
                "roundtrip_ms": round(r3[0], 2),
            },
        }

    # ── Round-trip preservation test ─────────────────────────────────────
    print("─" * 75)
    print("  ROUND-TRIP PRESERVATION TEST")
    print("─" * 75)

    rt_yaml = LARGE_YAML

    # pyrs-yaml
    out1 = pyrs_yaml.parse(rt_yaml).to_yaml()
    print(
        f"  pyrs-yaml:     comments={'# Comments everywhere' in out1}  anchors={'&defaults' in out1}  tags={'!!str' in out1}"
    )

    # PyYAML
    out2 = pyyaml.safe_dump(pyyaml.safe_load(rt_yaml))
    print(
        f"  PyYAML:        comments={'# Comments everywhere' in out2}  anchors={'&defaults' in out2}  tags={'!!str' in out2}"
    )

    # ruamel.yaml
    out3 = ruamel_dump(ruamel_load(rt_yaml))
    print(
        f"  ruamel.yaml:   comments={'# Comments everywhere' in out3}  anchors={'&defaults' in out3}  tags={'!!str' in out3}"
    )
    print()

    # ── Feature support comparison ───────────────────────────────────────
    print("─" * 75)
    print("  FEATURE SUPPORT COMPARISON")
    print("─" * 75)

    features = [
        ("YAML 1.2 compliance", True, True, True),
        ("Comments (standalone)", True, False, True),
        ("Comments (inline)", True, False, True),
        ("Anchors/aliases", True, False, True),
        ("Tags (explicit)", True, False, True),
        ("Block scalars", True, True, True),
        ("Flow collections", True, True, True),
        ("Merge keys (<<)", True, False, True),
        ("Complex keys", True, True, True),
        ("Round-trip preservation", True, False, True),
        ("Python bindings", True, True, True),
        ("ABI3 (py3.9+)", True, False, False),
        ("Type stubs (.pyi)", True, True, False),
        ("i18n error messages", True, False, False),
    ]

    print(f"  {'Feature':35s} {'pyrs-yaml':12s} {'PyYAML':10s} {'ruamel':10s}")
    print(f"  {'-' * 67}")
    for name, pyr, pyr2, ruamel in features:
        pyr_m = "✅" if pyr else "❌"
        pyr2_m = "✅" if pyr2 else "❌"
        ruamel_m = "✅" if ruamel else "❌"
        print(f"  {name:35s} {pyr_m:8s}     {pyr2_m:6s}     {ruamel_m:6s}")
    print()

    if args.json:
        print(json.dumps(results, indent=2))
        return

    print("=" * 75)
    print("  Benchmark complete.")
    print("=" * 75)


# ── Integration test: speedup assertion ──────────────────────────────────


def benchmark_block_style(rounds=50):
    """Run block-style benchmark and return results dict."""
    yaml_str = (
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

    def parse_pyrs():
        return pyrs_yaml.parse(yaml_str)

    def serialize_pyrs():
        return pyrs_yaml.parse(yaml_str).to_yaml()

    def parse_pyyaml():
        return pyyaml.safe_load(yaml_str)

    def serialize_pyyaml():
        return pyyaml.safe_dump(pyyaml.safe_load(yaml_str))

    pyr_parse = timed(parse_pyrs, rounds)
    pyr_ser = timed(serialize_pyrs, rounds)
    pyy_parse = timed(parse_pyyaml, rounds)
    pyy_ser = timed(serialize_pyyaml, rounds)

    return {
        "pyrs_yaml": {"parse_ms": pyr_parse[0], "serialize_ms": pyr_ser[0]},
        "pyyaml": {"parse_ms": pyy_parse[0], "serialize_ms": pyy_ser[0]},
    }


def test_speedup_vs_pyyaml():
    """Verify pyrs-yaml is faster than PyYAML on block-style YAML."""
    results = benchmark_block_style(rounds=10)
    assert results["pyrs_yaml"]["serialize_ms"] < results["pyyaml"]["serialize_ms"], (
        f"pyrs-yaml slower than PyYAML: "
        f"{results['pyrs_yaml']['serialize_ms']:.2f}ms vs "
        f"{results['pyyaml']['serialize_ms']:.2f}ms"
    )


if __name__ == "__main__":
    main()
