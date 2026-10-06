#!/usr/bin/env bash
# One fuzz round per crash, so a single CI run reports every bug it finds instead
# of stopping at the first.
#
# Why a loop and not a flag: measured on this harness (nightly-2026-08-15,
# cargo-fuzz 0.13.2, GNU host target), `-keep_going=10 -error_exitcode=42` still
# produced exactly one artifact, because a Rust panic reaches libFuzzer as a
# deadly signal that kills the process. The loop does work: with three crash sites
# compiled into the target, successive rounds reported site one, then two, then
# three - three distinct bugs in one job. Two details make that honest:
#
#   * each crashing input is dropped from the corpus before the next round, else a
#     later round just re-finds the same bug through the same corpus entry;
#   * findings are deduped by crash signature, not by artifact file name, because
#     the artifact hash covers the input bytes: four artifacts can be one bug.
#
# Env: FUZZ_TIME (seconds per round), FUZZ_ROUNDS (max rounds), and
# GITHUB_EVENT_NAME, which selects the policy:
#   pull_request -> `-runs=0`: replay every committed seed once, explore nothing.
#     Deterministic, so it is allowed to block a merge.
#   anything else -> the sampled discovery window, looped as above.
# Exit 0 when every round came back clean; 1 when anything was found.
set -uo pipefail

target="${1:?usage: fuzz_rounds.sh <target>}"
time_per_round="${FUZZ_TIME:-60}"
rounds="${FUZZ_ROUNDS:-3}"
summary="${GITHUB_STEP_SUMMARY:-/dev/null}"

if [ "${GITHUB_EVENT_NAME:-}" = pull_request ]; then
    mode=replay
    rounds=1
    libfuzzer_args=(-runs=0)
else
    mode=explore
    libfuzzer_args=(-max_total_time="$time_per_round")
fi
echo "$target: mode=$mode (rounds=$rounds, libFuzzer args: ${libfuzzer_args[*]})"

cd "$(dirname "$0")/../fuzz"
shopt -s nullglob

mkdir -p "collected/$target"
: > "collected/$target/.signatures"

found=0
for round in $(seq 1 "$rounds"); do
    rm -rf "artifacts/$target"
    mkdir -p "artifacts/$target"
    log="/tmp/fuzz-$target-round$round.log"

    echo "=== $target round $round/$rounds (${libfuzzer_args[*]})"
    cargo fuzz run "$target" --target x86_64-unknown-linux-gnu -- \
        "${libfuzzer_args[@]}" -rss_limit_mb=3072 > "$log" 2>&1 || true

    crashes=("artifacts/$target"/crash-*)
    if [ ${#crashes[@]} -eq 0 ]; then
        echo "round $round: clean"
        break
    fi

    # One signature per round: the panic location, the assertion text, or the
    # libFuzzer summary line, whichever comes first.
    signature=$(grep -m1 -hoE "panicked at [^:]+|assertion [^ ]+ failed|SUMMARY: libFuzzer: .*" "$log")
    [ -n "$signature" ] || signature="unknown (see job log)"
    if grep -qxF "$signature" "collected/$target/.signatures"; then
        echo "round $round: re-found a known signature, stopping"
        break
    fi
    printf '%s\n' "$signature" >> "collected/$target/.signatures"
    found=$((found + 1))

    {
        printf '### %s, round %s\n\n' "$target" "$round"
        printf 'signature: `%s`\n\n' "$signature"
        for art in "${crashes[@]}"; do
            cp "$art" "collected/$target/"
            printf -- '- `%s` (%s bytes)\n' "$(basename "$art")" "$(wc -c < "$art")"
            # Stop the next round from tripping on this exact input again.
            for c in "corpus/$target"/*; do
                cmp -s "$art" "$c" && rm -f "$c"
            done
        done
        echo
    } >> "$summary"
done

if [ "$found" -gt 0 ]; then
    # Publish every collected crash under the job's artifact path, and fail once
    # with the count, so the job says "N distinct bugs" rather than "1 of N".
    cp -f "collected/$target"/crash-* "artifacts/$target/" 2>/dev/null || true
    signatures=$(wc -l < "collected/$target/.signatures")
    echo "::error::$target surfaced $signatures distinct crash signature(s); inputs are in the fuzz-artifacts-$target artifact"
    exit 1
fi

echo "$target: no crash in $rounds round(s)"
