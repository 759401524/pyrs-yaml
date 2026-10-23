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

<!-- bench: layout=sizes format=yaml operation=parse sizes=small,medium,large baseline=PyYAML -->
| YAML Size | pyrs-yaml | PyYAML | ruamel.yaml | Speedup vs PyYAML |
|-----------|-----------|--------|-------------|-------------------|
| Small (~100 B) | 4.3 µs | 125 µs | 293 µs | **29×** |
| Medium (~690 B) | 23 µs | 807 µs | 1.9 ms | **36×** |
| Large (~2 KB) | 61 µs | 2.0 ms | 4.6 ms | **32×** |

### Serialize Performance

<!-- bench: layout=sizes format=yaml operation=serialize sizes=small,medium,large baseline=PyYAML -->
| YAML Size | pyrs-yaml | PyYAML | ruamel.yaml | Speedup vs PyYAML |
|-----------|-----------|--------|-------------|-------------------|
| Small (~100 B) | 0.38 µs | 75 µs | 178 µs | **199×** |
| Medium (~690 B) | 1.3 µs | 438 µs | 1.0 ms | **340×** |
| Large (~2 KB) | 4.5 µs | 1.1 ms | 2.4 ms | **240×** |

### Round-Trip Performance

<!-- bench: layout=sizes format=yaml operation=roundtrip sizes=small,medium,large baseline=PyYAML -->
| YAML Size | pyrs-yaml | PyYAML | ruamel.yaml | Speedup vs PyYAML |
|-----------|-----------|--------|-------------|-------------------|
| Small (~100 B) | 4.8 µs | 221 µs | 514 µs | **46×** |
| Medium (~690 B) | 24 µs | 1.3 ms | 3.0 ms | **54×** |
| Large (~2 KB) | 67 µs | 3.2 ms | 7.1 ms | **47×** |

### JSON

The same JSON text through three readers. `orjson` is the faster one on the large document; the row is published because it is the measurement, not because it flatters us.

<!-- bench: layout=sizes format=json operation=parse sizes=config,large baseline=json -->
| Document | pyrs-yaml | json | orjson | Speedup |
| --- | ---: | ---: | ---: | ---: |
| Config (~123 B) | 0.49 µs | 1.1 µs | 0.33 µs | **2.2×** |
| Large (~1.9 KB) | 5.5 µs | 8.2 µs | 3.6 µs | **1.5×** |

### TOML

TOML text through `pyrs_yaml.load_toml` and the standard library's `tomllib`.

<!-- bench: layout=sizes format=toml operation=parse sizes=hot,config baseline=tomllib -->
| Document | pyrs-yaml | tomllib | Speedup |
| --- | ---: | ---: | ---: |
| Hot (~1.8 KB) | 31 µs | 170 µs | **5.6×** |
| Config (~2.1 KB) | 49 µs | 293 µs | **5.9×** |

### JSON5

One small JSON5 document against `pyjson5`, which is the only independent JSON5 reader in the ecosystem. We are the slower reader here, and that is why the JSON5 read path stays an open optimisation target.

<!-- bench: layout=sizes format=json5 operation=parse sizes=config baseline=pyjson5 -->
| Document | pyrs-yaml | pyjson5 | Speedup |
| --- | ---: | ---: | ---: |
| Config (~83 B) | 1.6 µs | 0.65 µs | **0.4×** |

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

1. :material-trending-up: **pyrs-yaml parses 29–36× faster and serializes 199–340× faster than PyYAML**
2. :material-trending-up: **pyrs-yaml is 69–82× faster at parsing and 473–809× faster at serializing than ruamel.yaml** while matching its round-trip features
3. :material-trending-up: **Round-trip is 46–54× faster than PyYAML and 105–122× faster than ruamel.yaml**
4. :material-bolt: **Rust-side parsing** avoids the Python call overhead; the simulation leg decomposes parse cost by operation
5. :material-bolt: **Serialization** stays well below parsing cost in the same simulation leg
6. :material-chart-line: **The speed advantage holds across all document sizes**

### Notes

- Python tables: batch-timed wall-clock readings recorded 2026-10-23 by `scripts/bench_snapshot.py`; the snapshot (`.ci/perf-snapshot.json`) records the environment, library versions and fixture sizes
- CodSpeed CI runs in simulation mode and reports instruction counts; use it for relative trends
- Ratios are indicative and vary with interpreter, hardware, and document size
- PyYAML benchmarks use `safe_load`/`safe_dump` (same safety guarantees)
- ruamel.yaml benchmarks use default settings
- Python fixtures are 95 B / 687 B / 1949 B (`tests/data/yaml_samples.py`)
