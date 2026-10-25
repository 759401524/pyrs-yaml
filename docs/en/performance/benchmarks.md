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
| YAML Size | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | Speedup vs PyYAML |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------- |
| Small (~100 B) | 4.4 µs | 151 µs | 347 µs | 1.9 µs | 5.7 µs | **35×** |
| Medium (~690 B) | 27 µs | 976 µs | 2.3 ms | 10 µs | 33 µs | **37×** |
| Large (~2 KB) | 72 µs | 2.3 ms | 5.4 ms | 27 µs | 86 µs | **32×** |

### Serialize Performance

<!-- bench: layout=sizes format=yaml operation=serialize sizes=small,medium,large baseline=PyYAML -->
| YAML Size | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | Speedup vs PyYAML |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------- |
| Small (~100 B) | 0.42 µs | 86 µs | 227 µs | 0.97 µs | 3.1 µs | **204×** |
| Medium (~690 B) | 1.6 µs | 568 µs | 1.4 ms | 6.1 µs | 17 µs | **347×** |
| Large (~2 KB) | 5.4 µs | 1.4 ms | 3.3 ms | 17 µs | 42 µs | **261×** |

### Round-Trip Performance

<!-- bench: layout=sizes format=yaml operation=roundtrip sizes=small,medium,large baseline=PyYAML -->
| YAML Size | pyrs-yaml | PyYAML | ruamel.yaml | yaml-rs | ryaml | Speedup vs PyYAML |
| ----------- | ----------- | -------- | ------------- | ---: | ---: | ------------------- |
| Small (~100 B) | 5.7 µs | 265 µs | 619 µs | 3.1 µs | 9.7 µs | **47×** |
| Medium (~690 B) | 30 µs | 1.7 ms | 3.6 ms | 17 µs | 51 µs | **55×** |
| Large (~2 KB) | 82 µs | 3.9 ms | 9.1 ms | 48 µs | 135 µs | **48×** |

### JSON

The same JSON text through three readers. `orjson` is the faster one on the large document; the row is published because it is the measurement, not because it flatters us.

<!-- bench: layout=sizes format=json operation=parse sizes=config,large baseline=json -->
| Document | pyrs-yaml | json | orjson | Speedup vs json |
| --- | ---: | ---: | ---: | ---: |
| Config (~123 B) | 0.56 µs | 1.3 µs | 0.45 µs | **2.3×** |
| Large (~1.9 KB) | 7.8 µs | 9.8 µs | 4.5 µs | **1.3×** |

### TOML

TOML text through `pyrs_yaml.load_toml`, the standard library's `tomllib` and `rtoml` — the `toml-rs` binding. `rtoml` decodes straight into a dictionary and still reads about 1.5× faster on both fixtures. The node model is no longer what we pay for: `load_toml` now walks the scanner's own table tree into Python, and what is left of the row is that tree. Measured on this host, the scan plus the tree already costs what `rtoml`'s entire parse does, so the remaining target is to build no tree at all.

<!-- bench: layout=sizes format=toml operation=parse sizes=hot,config baseline=rtoml -->
| Document | pyrs-yaml | tomllib | rtoml | Speedup vs rtoml |
| --- | ---: | ---: | ---: | ---: |
| Hot (~1.8 KB) | 29 µs | 212 µs | 20 µs | **0.7×** |
| Config (~2.1 KB) | 45 µs | 358 µs | 31 µs | **0.7×** |

### JSON5

One small JSON5 document against `pyjson5`, which is the only independent JSON5 reader in the ecosystem. We are the slower reader here, and that is why the JSON5 read path stays an open optimisation target.

<!-- bench: layout=sizes format=json5 operation=parse sizes=config baseline=pyjson5 -->
| Document | pyrs-yaml | pyjson5 | Speedup vs pyjson5 |
| --- | ---: | ---: | ---: |
| Config (~83 B) | 2.0 µs | 0.82 µs | **0.4×** |

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

1. :material-trending-up: **pyrs-yaml parses 32–37× faster and serializes 204–347× faster than PyYAML**
2. :material-trending-up: **pyrs-yaml is 75–85× faster at parsing and 540–832× faster at serializing than ruamel.yaml** while matching its round-trip features
3. :material-trending-up: **Round-trip is 47–55× faster than PyYAML and 109–119× faster than ruamel.yaml**
4. :material-scale-balance: **The Rust-backed readers split the result** — `yaml-rs` decodes plain objects rather than a round-trip tree, and parses 2.3–2.7× faster and round-trips 1.7–1.8× faster; pyrs-yaml serializes 2.3–3.7× faster in return
5. :material-bolt: **Rust-side parsing** avoids the Python call overhead; the simulation leg decomposes parse cost by operation
6. :material-bolt: **Serialization** stays well below parsing cost in the same simulation leg
7. :material-chart-line: **The speed advantage holds across all document sizes**

### Notes

- Python tables: batch-timed wall-clock readings recorded 2026-10-25 by `scripts/bench_snapshot.py`; the snapshot (`.ci/perf-snapshot.json`) records the environment, library versions and fixture sizes
- CodSpeed CI runs in simulation mode and reports instruction counts; use it for relative trends
- Ratios are indicative and vary with interpreter, hardware, and document size
- PyYAML benchmarks use `safe_load`/`safe_dump` (same safety guarantees)
- ruamel.yaml benchmarks use default settings
- `yaml-rs` and `ryaml` hand back plain Python objects, so their rows are not the same job as pyrs-yaml's, which builds the document a later edit can format-preserve
- Python fixtures are 95 B / 687 B / 1949 B (`tests/data/yaml_samples.py`)
