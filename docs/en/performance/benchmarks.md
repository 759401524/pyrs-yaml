---
title: Benchmarks
description: Performance benchmarks for pyrs-yaml compared against PyYAML and ruamel.yaml, including Rust-side divan benchmarks.
tags:
  - docs
status: new
---

## Benchmarks

!!! note "Benchmark environment"
    CodSpeed CI runs in **simulation** mode (`mode: simulation`), which counts
    instructions rather than wall-clock time. The wall-clock figures below come
    from `scripts/bench_snapshot.py`, which times batches of calls and records
    the fastest of nine batches; its readings are committed to
    `.ci/perf-snapshot.json` together with the interpreter, library versions and
    fixture sizes they were measured on. Treat the ratios as indicative — they
    depend on the interpreter, hardware, and document.

Performance benchmarks for pyrs-yaml. CodSpeed's simulation numbers track relative trends; the
wall-clock tables below are batch-timed readings from one machine, reproducible with
`scripts/bench_snapshot.py`.

### Methodology

- **Tool:** `codspeed-divan-compat` (Rust) and `pytest-codspeed` (Python) in CI; `scripts/bench_snapshot.py` for the wall-clock tables
- **Rounds:** nine batches per case, each calibrated to at least 0.1 s; the fastest batch is reported
- **Metric:** microseconds (Python wall-clock and Rust instrumented); CodSpeed reports instruction counts, not time

### Parse Performance

| YAML Size | pyrs-yaml | PyYAML | ruamel.yaml | Speedup vs PyYAML |
|-----------|-----------|--------|-------------|-------------------|
| Small (~100 B) | 4.6 µs | 134 µs | 311 µs | **29×** |
| Medium (~690 B) | 24 µs | 839 µs | 2.0 ms | **34×** |
| Large (~2 KB) | 67 µs | 2.0 ms | 5.0 ms | **30×** |

### Serialize Performance

| YAML Size | pyrs-yaml | PyYAML | ruamel.yaml | Speedup vs PyYAML |
|-----------|-----------|--------|-------------|-------------------|
| Small (~100 B) | 0.37 µs | 83 µs | 201 µs | **223×** |
| Medium (~690 B) | 1.3 µs | 481 µs | 1.1 ms | **368×** |
| Large (~2 KB) | 4.5 µs | 1.2 ms | 2.6 ms | **265×** |

### Round-Trip Performance

| YAML Size | pyrs-yaml | PyYAML | ruamel.yaml | Speedup vs PyYAML |
|-----------|-----------|--------|-------------|-------------------|
| Small (~100 B) | 5.3 µs | 238 µs | 545 µs | **45×** |
| Medium (~690 B) | 26 µs | 1.5 ms | 3.3 ms | **59×** |
| Large (~2 KB) | 70 µs | 3.5 ms | 7.8 ms | **50×** |

### Rust-Side Benchmarks (divan)

Measured by the Rust CodSpeed leg (`cargo codspeed`, simulation mode): these are instrumented
times, not native wall-clock. The fixtures come from `pyrs_yaml_core::bench_inputs`, whose "small"
document is 19 B — not the 95 B document used by the Python tables — so rows with the same name are
not comparable.

| Operation | Time |
|-----------|------|
| Parse (small) | 85.6 µs |
| Parse (medium) | 277.4 µs |
| Parse (large) | 840.6 µs |
| Parse (anchors) | 254.3 µs |
| Parse (comments) | 164.9 µs |
| Parse (block scalars) | 123.8 µs |
| Serialize (small) | 7.9 µs |
| Serialize (medium) | 32.2 µs |
| Serialize (large) | 76.1 µs |
| Serialize (anchors) | 27.0 µs |
| Serialize (block scalars) | 15.4 µs |
| Round-trip (small) | 91.7 µs |
| Round-trip (medium) | 303.1 µs |
| Round-trip (large) | 910.0 µs |

### Key Takeaways

1. :material-trending-up: **pyrs-yaml parses 29–34× faster and serializes 223–368× faster than PyYAML**
2. :material-trending-up: **pyrs-yaml is 68–81× faster at parsing and 541–876× faster at serializing than ruamel.yaml** while matching its round-trip features
3. :material-trending-up: **Round-trip is 45–59× faster than PyYAML and 102–125× faster than ruamel.yaml**
4. :material-bolt: **Rust-side parsing** avoids the Python call overhead; the simulation leg decomposes parse cost by operation
5. :material-bolt: **Serialization** stays well below parsing cost in the same simulation leg
6. :material-chart-line: **The speed advantage holds across all document sizes**

### Notes

- Python tables: batch-timed wall-clock readings recorded 2026-10-20 by `scripts/bench_snapshot.py`; the snapshot (`.ci/perf-snapshot.json`) records the environment, library versions and fixture sizes
- CodSpeed CI runs in simulation mode and reports instruction counts; use it for relative trends
- Ratios are indicative and vary with interpreter, hardware, and document size
- PyYAML benchmarks use `safe_load`/`safe_dump` (same safety guarantees)
- ruamel.yaml benchmarks use default settings
- Python fixtures are 95 B / 687 B / 1949 B (`tests/data/yaml_samples.py`)
