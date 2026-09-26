#!/usr/bin/env bash
# WSL-side CodSpeed measurement runner (walltime mode).
#
# Why: this Windows box shows ±38% cross-run benchmark noise; WSL2 walltime is
# ~±10%, and repeating runs + taking per-benchmark medians cuts it further.
# Usage (from inside WSL, in the repo root):
#   bash scripts/wsl_codspeed.sh [runs]   # default runs=3
# Results land in .codspeed/ as usual. After the runs, compare any two files:
#   uv run python scripts/compare_codspeed.py .codspeed/results_A.json .codspeed/results_B.json
set -euo pipefail

RUNS="${1:-3}"
REPO="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO"
export VIRTUAL_ENV="$REPO/.venv-linux"
PY="$REPO/.venv-linux/bin/python"

[ -x "$PY" ] || { echo "missing $PY — build first: maturin develop --release"; exit 1; }

for i in $(seq 1 "$RUNS"); do
    echo "=== codspeed run $i/$RUNS $(date -u +%H:%M:%S) ==="
    "$PY" -m pytest tests/test_benchmark_crosslib.py tests/test_benchmark_api.py \
        --codspeed -q "${@:2}"
done

# Consolidate: per-benchmark median across runs -> print as one summary line set.
LATEST=$(ls -t .codspeed/results_*.json | head -n "$RUNS" | tr '\n' ' ')
"$PY" - "$LATEST" <<'EOF'
import json, statistics, sys
files = sys.argv[1].split()
acc = {}
for f in files:
    for b in json.load(open(f))["benchmarks"]:
        acc.setdefault(b["name"], []).append(b["stats"]["median_ns"])
print(f"consolidated {len(files)} runs -> median-of-medians (us):")
for name, vals in sorted(acc.items(), key=lambda kv: -statistics.median(kv[1])):
    print(f"{statistics.median(vals)/1000:10.2f}  {name}")
EOF
