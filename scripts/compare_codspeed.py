"""Compare two local pytest-codspeed results files (same-machine A/B).

Usage: python scripts/compare_codspeed.py <base.json> <head.json> [--top N]
Reports per-benchmark median changes sorted by |delta%|, flags regressions.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def load(path: str | Path) -> dict[str, float]:
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    out: dict[str, float] = {}
    for bench in data["benchmarks"]:
        out[bench["name"]] = bench["stats"]["median_ns"]
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("base")
    ap.add_argument("head")
    ap.add_argument("--top", type=int, default=25)
    ap.add_argument(
        "--filter",
        default="",
        help="substring filter on benchmark names",
    )
    args = ap.parse_args()

    base, head = load(args.base), load(args.head)
    rows = []
    for name, b in base.items():
        if args.filter and args.filter not in name:
            continue
        h = head.get(name)
        if h is None:
            rows.append((name, b, None))
            continue
        rows.append((name, b, (h - b) / b * 100.0))

    rows.sort(key=lambda r: abs(r[2]) if r[2] is not None else 1e9, reverse=True)
    regressions = [r for r in rows if r[2] is not None and r[2] > 3.0]
    improvements = [r for r in rows if r[2] is not None and r[2] < -3.0]

    print(f"{'delta%':>8}  {'base(ns)':>12}  benchmark")
    for name, b, d in rows[: args.top]:
        if d is None:
            print(f"{'removed':>8}  {b:12.0f}  {name}")
        else:
            print(f"{d:+8.1f}  {b:12.0f}  {name}")

    total = sum(1 for r in rows if r[2] is not None)
    print(f"\n{total} compared | {len(improvements)} improved (>3%) | {len(regressions)} regressed (>3%)")
    return 1 if regressions else 0


if __name__ == "__main__":
    sys.exit(main())
