---

title: Development Setup
lang: ko-KR

## 개발 환경 설정

Set up your environment to contribute to pyyaml-rs.

### Prerequisites

- **Python** ≥ 3.8 (CPython)
- **Rust** ≥ 1.70 (via [rustup](https://rustup.rs/))
- **Git**
- **uv** (recommended) or **pip**
- **NumPy** — required for running the NumPy serialization test suite (`uv run --frozen pytest tests/test_numpy.py`)

### Clone and Install

```bash
git clone https://github.com/759401524/pyyaml-rs.git
cd pyyaml-rs

# Using uv (recommended)
uv sync

# Or using pip
pip install maturin
uv run --frozen maturin develop --release
```

### Verify 설치

```bash
# Run Rust tests
cargo test

# Run Python tests
uv run --frozen pytest tests/

# Run benchmarks
cargo bench
```

### Project Structure

```text
pyyaml-rs/
├── src/
│   ├── lib.rs              # PyO3 module definition
│   ├── ast.rs              # Custom AST (CustomNode)
│   ├── parser/
│   │   ├── mod.rs          # saphyr-parser integration
│   │   └── yaml/           # YAML-specific parsing
│   │       ├── comment.rs  # Comment extraction
│   │       ├── merge.rs    # Merge key resolution
│   │       ├── scalar.rs   # Scalar parsing
│   │       └── types.rs    # YAML 1.2 type resolution
│   └── serializer.rs       # YAML serialization
├── python/pyyaml_rs/
│   ├── __init__.py         # Python package init
│   ├── pyyaml_rs.pyi       # Type stubs
│   └── py.typed            # PEP 561 marker
├── tests/                  # Python test suite
├── benches/                # Rust benchmarks
└── docs/                   # Documentation (mkdocs)
```

### Build Commands

```bash
# Build Python extension
uv run --frozen maturin develop --release

# Build wheel
maturin build --release --out dist

# Build with debug info
cargo build
```

### Development Workflow

1. **Write tests first** (TDD)
2. **Implement changes** in `src/`
3. **Run `cargo test`** to verify Rust tests
4. **Run `uv run --frozen pytest tests/`** to verify Python tests
5. **Run `cargo clippy -- -D warnings`** to check code quality
6. **Run `cargo fmt`** to format code
