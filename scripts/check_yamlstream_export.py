"""Static check: YamlStream export consistency across Rust, Python, and stubs.

YamlStream is a known boundary — defined as ``#[pyclass]`` but not
``#[pymodule_export]``ed. This script verifies the three surfaces stay
in sync: if YamlStream appears in one, it must appear in the others
(or be explicitly documented as excluded).

Exit 0 = consistent (all surfaces agree).
Exit 1 = drift detected — one surface references YamlStream without the others.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

RUST_MOD = ROOT / "crates/pyrs-yaml/src/py/mod.rs"
RUST_STREAMING = ROOT / "crates/pyrs-yaml/src/py/streaming.rs"
PYTHON_INIT = ROOT / "python/pyrs_yaml/__init__.py"
STUB = ROOT / "python/pyrs_yaml/pyrs_yaml.pyi"


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def check() -> list[str]:
    issues: list[str] = []

    # --- Surface 1: #[pymodule_export] in mod.rs ---
    mod_src = _read(RUST_MOD)
    pymodule_has_yamlstream = bool(re.search(r"YamlStream", mod_src) and re.search(r"#\[pymodule_export\]", mod_src))

    # --- Surface 2: #[pyclass] in streaming.rs ---
    streaming_src = _read(RUST_STREAMING)
    pyclass_has_yamlstream = bool(re.search(r"#\[pyclass.*\]", streaming_src))

    # --- Surface 3: __init__.py __all__ ---
    init_src = _read(PYTHON_INIT)
    all_match = re.search(r"__all__\s*=\s*\[(.*?)\]", init_src, re.DOTALL)
    init_all_has = bool(all_match and "YamlStream" in all_match.group(1))
    init_import_has = "YamlStream" in init_src

    # --- Surface 4: .pyi stub ---
    stub_src = _read(STUB)
    stub_has = "YamlStream" in stub_src

    # --- Consistency rules ---
    # Rule 1: If #[pyclass] exists, it may be un-exported (known boundary).
    # Rule 2: If .pyi references it, __all__ must also reference it
    #         (otherwise type-checkers see a type that doesn't exist at import time).
    if stub_has and not init_all_has:
        issues.append(
            f"DRIFT: {STUB.name} references YamlStream but __all__ in "
            f"{PYTHON_INIT.name} does not — type-checkers will report an error. "
            f"Either add YamlStream to __all__ or remove it from the stub."
        )

    # Rule 3: If #[pyclass] exists but not #[pymodule_export], it's the known
    #         boundary. But it shouldn't silently change — log it.
    if pyclass_has_yamlstream and not pymodule_has_yamlstream:
        issues.append(
            f"NOTE: YamlStream is #[pyclass] in {RUST_STREAMING.name} but not "
            f"#[pymodule_export]ed in {RUST_MOD.name} — known boundary "
            f"(see AGENTS.md Known boundaries). Type-checkers cannot resolve it."
        )

    # Rule 4: If YamlStream suddenly appears in #[pymodule_export], that's
    #         a change worth noting — __init__.py should be updated to match.
    if pymodule_has_yamlstream and not init_import_has:
        issues.append(
            f"DRIFT: YamlStream is #[pymodule_export]ed in {RUST_MOD.name} "
            f"but not imported in {PYTHON_INIT.name}. Add it to __all__."
        )

    return issues


def main() -> int:
    issues = check()
    for issue in issues:
        print(issue, file=sys.stderr)
    if any(i.startswith("DRIFT") for i in issues):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
