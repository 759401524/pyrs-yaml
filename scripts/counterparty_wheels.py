#!/usr/bin/env python3
"""Measure which interpreter bands each counterparty publishes a wheel for, and record it.

Why this exists. The test group caps each counterparty library with `python_version < 'X.Y'`, and
those caps were remembered rather than measured: on 2026-10-28 five of nine still said `< '3.14'`
while `pyjson5`, `numpy`, `ujson` and `python-rapidjson` all shipped 3.14 wheels, and the ≥3.14 legs
quietly ran 236 fewer cross-library assertions than the 3.13 leg. A cap that is too low costs
coverage nothing reads; a cap that is too high turns the leg that cannot install the group red.
This script measures the boundary so both directions become a fact, and
`tests/test_counterparty_caps_gate.py` compares `pyproject.toml` against what it recorded.

What "a wheel exists" means here: `uv pip install --dry-run --no-build` succeeds for that one
requirement against a real interpreter of that band. `--no-build` is the crisp predicate — a plain
dry-run resolves a version without saying whether a wheel exists, and CI is allowed to build an
sdist, so a refusal here does not always break a leg (PyYAML has no 3.15 linux wheel and CI builds
it) while an acceptance always means no compiler is needed.

Free-threaded bands are asked of a real interpreter only. `uv pip compile --python-version 3.14t`
resolves the GIL bands but reports no wheel for every package at a `t` band: it never constructs that
environment, so the uniform negative is the instrument, not the vendor. `--cross` therefore measures
GIL bands on another OS and says so on the `t` rows rather than guessing.

Usage:
    python scripts/counterparty_wheels.py --host windows --bands 3.8,3.13,3.14,3.15,3.14t,3.15t
    python scripts/counterparty_wheels.py --host linux --bands 3.8,3.13,3.14,3.15,3.14t,3.15t
    python scripts/counterparty_wheels.py --cross macos --platform aarch64-apple-darwin --bands 3.8,3.15

Each run merges its rows into `--out` (default `.ci/counterparty-wheels.json`) so the file is the
union of what was actually measured. Exit code 0 = rows written; 1 = a probe that should have
answered refused; 2 = the probe could not run at all (uv missing, no interpreter), never a pass.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_OUT = REPO / ".ci" / "counterparty-wheels.json"

#: distribution name -> (PEP 508 name as it appears in the test group, minimum requirement)
COUNTERPARTIES = {
    "numpy": "numpy>=1.16.0",
    "orjson": "orjson>=3.9",
    "ujson": "ujson>=5.8",
    "python-rapidjson": "python-rapidjson>=1.18",
    "rtoml": "rtoml>=0.14",
    "pyjson5": "pyjson5>=1.6",
    "ryaml": "ryaml>=0.5.0",
    "yaml-edit": "yaml-edit>=0.1.2",
    "yaml-rs": "yaml-rs>=0.1.0",
}


def run(argv: list[str], cwd: str | None = None, timeout: int = 600) -> subprocess.CompletedProcess:
    return subprocess.run(argv, capture_output=True, text=True, timeout=timeout, cwd=cwd)


def venv_python(band: str, tag: str) -> str | None:
    """A real interpreter of that band, via uv's managed builds, or None when it cannot be made.

    The directory is keyed by the OS tag because a Windows venv and a Linux one do not share a layout:
    a run that reused the other platform's directory would look for `bin/python` inside a `Scripts`
    tree and report a resolver failure as if the vendor had published no wheel.
    """
    root = REPO / "target" / f"cpw-{tag}-{band}"
    if not (root / "pyvenv.cfg").is_file():
        proc = run(["uv", "venv", str(root), "--python", band], timeout=900)
        if proc.returncode != 0:
            return None
    script = "Scripts/python.exe" if sys.platform == "win32" else "bin/python"
    return str(root / script)


def ask_host(band: str, spec: str, tag: str) -> tuple[str, str]:
    """Ask one real interpreter whether one requirement has a wheel. Returns (state, detail)."""
    exe = venv_python(band, tag)
    if exe is None:
        return "unmeasured", f"no {band} interpreter could be created"
    proc = run(["uv", "pip", "install", "--dry-run", "--no-build", "--python", exe, spec])
    out = (proc.stdout or "") + (proc.stderr or "")
    if proc.returncode != 0:
        return "no-wheel", "no wheel for this interpreter"
    picked = [line.strip()[1:].strip() for line in out.splitlines() if line.strip().startswith("+")]
    return "wheel", picked[-1] if picked else "already installed"


def ask_cross(band: str, spec: str, platform: str) -> tuple[str, str]:
    """Ask the resolver about another OS's GIL band; a free-threaded band is not askable this way."""
    if band.endswith("t"):
        return "unmeasured", "a free-threaded band needs a real interpreter on its own OS"
    with tempfile.NamedTemporaryFile("w", suffix=".in.txt", delete=False) as handle:
        handle.write(f"{spec}\n")
        src = pathlib.Path(handle.name)
    try:
        proc = run(
            [
                "uv",
                "pip",
                "compile",
                "--no-build",
                "--quiet",
                "--python-version",
                band,
                "--python-platform",
                platform,
                str(src),
            ]
        )
    finally:
        src.unlink(missing_ok=True)
    if proc.returncode != 0:
        return "no-wheel", "no wheel for that platform and band"
    for line in (proc.stdout or "").splitlines():
        stripped = line.strip()
        if stripped and not stripped.startswith("#") and "==" in stripped:
            return "wheel", stripped
    return "wheel", "resolved"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--host", help="os tag for rows measured with real interpreters on this box")
    parser.add_argument("--cross", help="os tag for rows measured by the resolver for --platform")
    parser.add_argument("--platform", help="a uv --python-platform triple, used with --cross")
    parser.add_argument("--bands", default="3.13,3.14,3.15,3.14t,3.15t", help="comma-separated bands")
    parser.add_argument("--out", type=pathlib.Path, default=DEFAULT_OUT)
    args = parser.parse_args(argv)
    if not args.host and not args.cross:
        print("nothing to measure: pass --host or --cross")
        return 2
    if args.cross and not args.platform:
        print("cannot measure: --cross needs --platform")
        return 2
    bands = [band.strip() for band in args.bands.split(",") if band.strip()]
    if not bands:
        print("cannot measure: --bands named no band")
        return 2

    path: pathlib.Path = args.out
    data = json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {"rows": {}, "measured_by": {}}
    rows: dict = data.setdefault("rows", {})
    refused = 0
    for band in bands:
        if args.cross and band.endswith("t"):
            print(
                f"{args.cross} {band:6s} skipped - a free-threaded band is only askable of a real interpreter",
                flush=True,
            )
            continue
        os_tag = args.host or args.cross
        for name, spec in COUNTERPARTIES.items():
            if args.host:
                state, detail = ask_host(band, spec, args.host)
            else:
                state, detail = ask_cross(band, spec, args.platform or "")
            if state == "unmeasured":
                refused += 1
            rows.setdefault(os_tag, {}).setdefault(band, {})[name] = {"state": state, "detail": detail}
            print(f"{os_tag} {band:6s} {name:20s} {state:11s} {detail}", flush=True)
    data["measured_by"][os_tag] = "scripts/counterparty_wheels.py"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {len(rows)} row(s) to {path.relative_to(REPO).as_posix()}")
    return 1 if refused else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
