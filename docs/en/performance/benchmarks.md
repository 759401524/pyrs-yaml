---
title: Benchmarks
description: Performance benchmarks for pyrs-yaml compared against PyYAML and ruamel.yaml, including Rust-side divan benchmarks.
tags:
  - docs
status: new
---

## Benchmarks

!!! note "Benchmark environment"
    All benchmarks are measured via CodSpeed CI (`pytest-codspeed`, WallTime
    mode). Relative speedups (×N) are consistent across hardware but absolute
    times may vary.

Performance benchmarks for pyrs-yaml, measured via CodSpeed CI (`pytest-codspeed`, WallTime mode).

### Methodology

- **Tool:** divan (Rust) + `pytest-codspeed` (Python)
- **Rounds:** WallTime mode, adaptive sampling (Python), 100+ samples per benchmark (Rust)
- **Metric:** Median time in milliseconds (Python), mean time in microseconds (Rust)

### Parse Performance

| YAML Size | pyrs-yaml | PyYAML | ruamel.yaml | Speedup vs PyYAML |
|-----------|-----------|--------|-------------|-------------------|
| Small (~100 B) | 0.18 ms | 3.8 ms | 8.8 ms | **21×** |
| Medium (~500 B) | 0.56 ms | 24.2 ms | 56.1 ms | **43×** |
| Large (~2 KB) | 1.5 ms | 57.7 ms | 127.9 ms | **38×** |

### Serialize Performance

| YAML Size | pyrs-yaml | PyYAML | ruamel.yaml | Speedup vs PyYAML |
|-----------|-----------|--------|-------------|-------------------|
| Small (~100 B) | 0.04 ms | 2.2 ms | 4.9 ms | **55×** |
| Medium (~500 B) | 0.08 ms | 12.6 ms | 28.1 ms | **159×** |
| Large (~2 KB) | 0.17 ms | 30.2 ms | 63.1 ms | **177×** |

### Round-Trip Performance

| YAML Size | pyrs-yaml | PyYAML | ruamel.yaml | Speedup vs PyYAML |
|-----------|-----------|--------|-------------|-------------------|
| Small (~100 B) | 0.22 ms | 6.0 ms[^1] | 13.7 ms[^1] | **28×** |
| Medium (~500 B) | 0.63 ms | 36.8 ms[^1] | 84.2 ms[^1] | **59×** |
| Large (~2 KB) | 1.6 ms | 87.9 ms[^1] | 191.0 ms[^1] | **55×** |

[^1]: PyYAML/ruamel round-trip times are estimated as parse + serialize from the same benchmark run.

### Rust-Side Benchmarks (divan)

Measured at the Rust level (no Python overhead):

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

1. :material-trending-up: **pyrs-yaml parses 21–43× faster and serializes 55–177× faster than PyYAML**
2. :material-trending-up: **pyrs-yaml is 48–100× faster at parsing and 123–371× faster at serializing than ruamel.yaml** while matching its round-trip features
3. :material-bolt: **Rust-side parsing** is extremely fast — small documents parse in ~86 µs
4. :material-bolt: **Serialization** is fast across all sizes — small documents serialize in ~8 µs
5. :material-chart-line: **The speed advantage holds across all document sizes**

### Notes

- Benchmarks measured via CodSpeed CI; absolute times may vary
- Relative speedups (×N) are consistent across hardware
- PyYAML benchmarks use `safe_load`/`safe_dump` (same safety guarantees)
- ruamel.yaml benchmarks use default settings
