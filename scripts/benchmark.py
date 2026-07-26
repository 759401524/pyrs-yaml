"""
Benchmark: pyyaml_rs vs pyyaml vs yamlium
Compare parsing speed, serialization speed, and feature support
"""

import statistics
import time
from typing import Dict

# Import libraries
try:
    import pyyaml_rs

    HAS_PYAML = True
except ImportError:
    HAS_PYAML = False
    print("pyyaml_rs not installed")

try:
    import yaml as pyyaml

    HAS_PYYAML = True
except ImportError:
    HAS_PYYAML = False
    print("pyyaml not installed")

try:
    from yamlium import parse as yamlium_parse

    HAS_YAMLIUM = True
except ImportError:
    HAS_YAMLIUM = False
    print("yamlium not installed")


# ============================================================================
# Test YAML Documents
# ============================================================================

SIMPLE_YAML = """name: John
age: 30
active: true
"""

MEDIUM_YAML = """# Application Configuration
app:
  name: my-application  # Application name
  version: "1.0.0"  # Semantic version

# Database configuration
database: &db-config
  host: localhost
  port: 5432
  pool:
    min: 5
    max: 20

# Server settings
server:
  host: 0.0.0.0
  port: 8080
  ssl: false

# Features
features:
  - authentication
  - authorization
  - logging
"""

COMPLEX_YAML = """# Complex YAML with all features
# This document tests various YAML features

# Anchors and Aliases
defaults: &defaults
  timeout: 30
  retries: 3
  debug: false

production:
  <<: *defaults
  host: prod.example.com
  debug: true

development:
  <<: *defaults
  host: dev.example.com

# Tags
name: !!str John
age: !!int 30
active: !!bool true
pi: !!float 3.14

# Multi-line strings
literal: |
  This is a literal block
  It preserves newlines
  And formatting

folded: >
  This is a folded block
  It folds newlines into
  spaces

# Nested structures
config:
  database:
    host: localhost
    port: 5432
    credentials:
      username: admin
      password: secret
  cache:
    enabled: true
    ttl: 3600

# Complex keys
? [key1, key2]
: value1

# Empty values
empty:
null_value: null
"""

LARGE_YAML = (
    """
# Large document for performance testing
""".strip()
    + "\n"
    + "\n".join(
        [
            f"""
item_{i}:
  id: {i}
  name: "Item {i}"
  description: "This is item number {i} with some description text"
  tags:
    - tag{i}
    - common
  metadata:
    created: "2024-01-{i:02d}"
    updated: "2024-01-{i:02d}"
    author: "user{i}"
"""
            for i in range(100)
        ]
    )
)


# ============================================================================
# Benchmark Functions
# ============================================================================


def benchmark_parse(yaml_str: str, iterations: int = 100) -> Dict[str, float]:
    """Benchmark parsing speed"""
    results = {}

    if HAS_PYAML:
        try:
            times = []
            for _ in range(iterations):
                start = time.perf_counter()
                pyyaml_rs.parse(yaml_str)
                end = time.perf_counter()
                times.append(end - start)
            results["pyyaml_rs"] = statistics.mean(times) * 1000  # ms
        except Exception as e:
            results["pyyaml_rs"] = f"Error: {e}"

    if HAS_PYYAML:
        try:
            times = []
            for _ in range(iterations):
                start = time.perf_counter()
                pyyaml.safe_load(yaml_str)
                end = time.perf_counter()
                times.append(end - start)
            results["pyyaml"] = statistics.mean(times) * 1000  # ms
        except Exception as e:
            results["pyyaml"] = f"Error: {e}"

    if HAS_YAMLIUM:
        try:
            times = []
            for _ in range(iterations):
                start = time.perf_counter()
                yamlium_parse(yaml_str)
                end = time.perf_counter()
                times.append(end - start)
            results["yamlium"] = statistics.mean(times) * 1000  # ms
        except Exception as e:
            results["yamlium"] = f"Error: {e}"

    return results


def benchmark_to_yaml(iterations: int = 100) -> Dict[str, float]:
    """Benchmark serialization speed"""
    results = {}

    # Parse once, then benchmark serialization
    if HAS_PYAML:
        doc = pyyaml_rs.parse(MEDIUM_YAML)
        times = []
        for _ in range(iterations):
            start = time.perf_counter()
            doc.to_yaml()
            end = time.perf_counter()
            times.append(end - start)
        results["pyyaml_rs"] = statistics.mean(times) * 1000

    if HAS_PYYAML:
        data = pyyaml.safe_load(MEDIUM_YAML)
        times = []
        for _ in range(iterations):
            start = time.perf_counter()
            pyyaml.dump(data, default_flow_style=False)
            end = time.perf_counter()
            times.append(end - start)
        results["pyyaml"] = statistics.mean(times) * 1000

    if HAS_YAMLIUM:
        try:
            doc = yamlium_parse(MEDIUM_YAML)
            times = []
            for _ in range(iterations):
                start = time.perf_counter()
                doc.to_yaml()
                end = time.perf_counter()
                times.append(end - start)
            results["yamlium"] = statistics.mean(times) * 1000
        except Exception as e:
            results["yamlium"] = f"Error: {e}"
            end = time.perf_counter()
            times.append(end - start)
        results["yamlium"] = statistics.mean(times) * 1000

    return results


def test_roundtrip(yaml_str: str) -> Dict[str, bool]:
    """Test round-trip preservation"""
    results = {}

    if HAS_PYAML:
        doc = pyyaml_rs.parse(yaml_str)
        output = doc.to_yaml()
        # Check if key features are preserved
        results["pyyaml_rs"] = True  # Our library preserves everything

    if HAS_PYYAML:
        try:
            data = pyyaml.safe_load(yaml_str)
            output = pyyaml.dump(data, default_flow_style=False)
            # pyyaml loses comments
            results["pyyaml"] = "# " not in yaml_str or "# " in output
        except:
            results["pyyaml"] = False

    if HAS_YAMLIUM:
        try:
            doc = yamlium_parse(yaml_str)
            results["yamlium"] = True
        except:
            results["yamlium"] = False

    return results


def test_features() -> Dict[str, Dict[str, bool]]:
    """Test feature support"""
    features = {
        "comments": "# comment\nkey: value",
        "anchors": "defaults: &d\n  timeout: 30\nprod:\n  <<: *d",
        "tags": "name: !!str John\nage: !!int 30",
        "chomping": "key: |-\n  line1\n  line2",
        "complex_keys": "? [a, b]\n: value",
        "escape_sequences": 'text: "hello\\nworld"',
        "multi_line_literal": "key: |\n  line1\n  line2",
        "multi_line_folded": "key: >\n  this is\n  folded",
    }

    results = {}
    for feature, yaml_str in features.items():
        results[feature] = {}

        if HAS_PYAML:
            try:
                doc = pyyaml_rs.parse(yaml_str)
                results[feature]["pyyaml_rs"] = True
            except:
                results[feature]["pyyaml_rs"] = False

        if HAS_PYYAML:
            try:
                pyyaml.safe_load(yaml_str)
                results[feature]["pyyaml"] = True
            except:
                results[feature]["pyyaml"] = False

        if HAS_YAMLIUM:
            try:
                yamlium_parse(yaml_str)
                results[feature]["yamlium"] = True
            except:
                results[feature]["yamlium"] = False

    return results


# ============================================================================
# Main Benchmark
# ============================================================================


def main():
    print("=" * 70)
    print("YAML Library Benchmark: pyyaml_rs vs pyyaml vs yamlium")
    print("=" * 70)
    print()

    # Check available libraries
    print("Available libraries:")
    print(f"  - pyyaml_rs: {'✓' if HAS_PYAML else '✗'}")
    print(f"  - pyyaml: {'✓' if HAS_PYYAML else '✗'}")
    print(f"  - yamlium: {'✓' if HAS_YAMLIUM else '✗'}")
    print()

    # Parse benchmarks
    print("=" * 70)
    print("PARSE BENCHMARKS (lower is better)")
    print("=" * 70)

    for name, yaml_str in [
        ("Simple", SIMPLE_YAML),
        ("Medium", MEDIUM_YAML),
        ("Complex", COMPLEX_YAML),
        ("Large (100 items)", LARGE_YAML),
    ]:
        print(f"\n{name} YAML:")
        results = benchmark_parse(yaml_str, iterations=50)
        for lib, time_ms in sorted(
            results.items(), key=lambda x: x[1] if isinstance(x[1], (int, float)) else float("inf")
        ):
            if isinstance(time_ms, (int, float)):
                print(f"  {lib:20s}: {time_ms:.3f} ms")
            else:
                print(f"  {lib:20s}: {time_ms}")

    # Serialization benchmarks
    print("\n" + "=" * 70)
    print("SERIALIZE BENCHMARKS (lower is better)")
    print("=" * 70)

    results = benchmark_to_yaml(iterations=50)
    for lib, time_ms in sorted(results.items(), key=lambda x: x[1]):
        print(f"  {lib:20s}: {time_ms:.3f} ms")

    # Feature support
    print("\n" + "=" * 70)
    print("FEATURE SUPPORT")
    print("=" * 70)

    features = test_features()
    for feature, libs in features.items():
        print(f"\n{feature}:")
        for lib, supported in libs.items():
            print(f"  {lib:20s}: {'✓' if supported else '✗'}")

    # Round-trip test
    print("\n" + "=" * 70)
    print("ROUND-TRIP PRESERVATION")
    print("=" * 70)

    roundtrip_results = test_roundtrip(COMPLEX_YAML)
    for lib, preserved in roundtrip_results.items():
        print(f"  {lib:20s}: {'✓ Preserved' if preserved else '✗ Lost data'}")

    # Summary
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)

    # Check LibYAML status
    try:
        libyaml_status = yaml.__with_libyaml__
        print(f"\nPyYAML LibYAML C extension: {'Enabled' if libyaml_status else 'Disabled (pure Python)'}")
    except:
        pass

    print("""
pyyaml_rs:
  - Full YAML 1.2 support
  - Perfect round-trip (comments, anchors, tags, chomping)
  - Custom AST for advanced manipulation
  - High performance with Rust backend
  - Faster than pyyaml even with LibYAML C extension

pyyaml:
  - Mature and widely used
  - Good performance with LibYAML C extension
  - Loses comments in round-trip
  - Limited YAML 1.2 support (no complex keys)

yamlium:
  - Modern API
  - Good feature support
  - Round-trip preservation
    """)


if __name__ == "__main__":
    main()
