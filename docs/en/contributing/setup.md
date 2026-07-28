# Development Setup

Set up your environment to contribute to pyyaml-rs.

## Prerequisites

- **Python** ≥ 3.8 (CPython)
- **Rust** ≥ 1.70 (via [rustup](https://rustup.rs/))
- **Git**
- **uv** (recommended) or **pip**
- **NumPy** — required for running the NumPy serialization test suite (`pytest tests/test_numpy.py`)

## Clone and Install

```bash
git clone https://github.com/759401524/pyyaml-rs.git
cd pyyaml-rs

# Using uv (recommended)
uv sync

# Or using pip (without uv)
pip install maturin
maturin develop --release
```

## Verify Installation

```bash
# Run Rust tests
cargo test

# Run Python tests (with uv lockfile for reproducible deps)
uv run --frozen pytest tests/

# Run benchmarks
cargo bench
```

## Project Structure

```text
pyyaml-rs/
├── src/
│   ├── lib.rs              # PyO3 module entry
│   ├── ast.rs              # Custom AST (CustomNode)
│   ├── serializer.rs       # YAML serialization
│   ├── i18n.rs             # i18n configuration
│   ├── i18n/               # Internationalization bundles
│   ├── parser/
│   │   ├── mod.rs          # Core parsing logic (AstReceiver)
│   │   ├── stream.rs       # Streaming event parser
│   │   └── yaml/           # YAML-specific parsing
│   │       ├── comment.rs  # Comment extraction
│   │       ├── merge.rs    # Merge key (<<) resolution
│   │       ├── scalar.rs   # Escape sequences & chomping
│   │       ├── schema.rs   # YAML schema resolution
│   │       └── types.rs    # YAML 1.2 type resolution
│   ├── py/                 # PyO3 Python bindings
│   │   ├── mod.rs          # Module definition & exports
│   │   ├── convert.rs      # Rust → Python type conversion
│   │   ├── ndarray.rs      # NumPy ndarray conversion
│   │   ├── python_types.rs # Python → CustomNode conversion
│   │   └── stream_events.rs# Stream event types
│   └── integration/        # Integration test helpers
├── python/pyyaml_rs/
│   ├── __init__.py         # Python package init
│   ├── py.typed            # PEP 561 marker
│   └── async_dump.py       # Async dump utilities
├── tests/                  # Python test suite (~395 tests)
├── benches/                # Rust benchmarks
├── docs/                   # Documentation (mkdocs)
├── ruff.toml               # Ruff linter config
├── pytest.ini              # Pytest config
└── Cargo.toml              # Rust dependencies
```

## Build Commands

```bash
# Build Python extension (with uv lockfile)
uv run --frozen maturin develop --release

# Build wheel
uv run --frozen maturin build --release --out dist

# Build with debug info
cargo build
```

## Development Workflow

1. **Write tests first** (TDD)
2. **Implement changes** in `src/`
3. **Run `cargo test`** to verify Rust tests
4. **Run `uv run --frozen pytest tests/`** to verify Python tests
5. **Run `cargo clippy -- -D warnings`** to check code quality
6. **Run `cargo fmt`** to format code
