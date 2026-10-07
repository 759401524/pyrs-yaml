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
#   * findings are keyed on the artifact's bytes, and the panic signature is reported
#     alongside them. The signature is *not* a stop condition: it names the assertion
#     that failed, and two different root causes can fail the same `assert_eq!` on the
#     same line — measured with crash-55c199ef and crash-5561902a, which one run of
#     this script counted as "1 distinct crash signature" while they were two bugs.
#     So the loop continues while it keeps finding new bytes, and the summary states
#     inputs and signatures separately.
#
# Env: FUZZ_TIME (the TOTAL exploration budget for this target, shared across the
# rounds), FUZZ_ROUNDS (max rounds), and GITHUB_EVENT_NAME, which selects the
# policy:
#   pull_request -> `-runs=0`: replay every committed seed once, explore nothing.
#     Deterministic, so it is allowed to block a merge.
#   anything else -> the sampled discovery window, looped as above: each round gets
#     FUZZ_TIME/FUZZ_ROUNDS seconds.
#
# FUZZ_TIME is a total, not a per-round figure, because the two readings multiplied
# rather than divided: when #266's `FUZZ_TIME: 600` (written for a single window)
# met this script's 3-round default, the job asked for 30 minutes against
# `timeout-minutes: 25` and the push-to-main run of 417ef6 was cut off mid round 2/3
# with zero crashes in its log. A timed-out sampler reports "failure" for a missing
# budget, indistinguishable from a found bug, so the arithmetic is the script's job.
# Exit 0 when every round came back clean; 1 when anything was found.
set -uo pipefail

target="${1:?usage: fuzz_rounds.sh <target>}"
total_budget="${FUZZ_TIME:-600}"
rounds="${FUZZ_ROUNDS:-3}"
summary="${GITHUB_STEP_SUMMARY:-/dev/null}"

if [ "$rounds" -lt 1 ]; then rounds=1; fi
# Integer division, floored at one second: a round with no budget would spin.
time_per_round=$(( total_budget / rounds ))
[ "$time_per_round" -ge 1 ] || time_per_round=1

if [ "${GITHUB_EVENT_NAME:-}" = pull_request ]; then
    mode=replay
    rounds=1
    libfuzzer_args=(-runs=0)
else
    mode=explore
    libfuzzer_args=(-max_total_time="$time_per_round")
fi
echo "$target: mode=$mode (rounds=$rounds, ${libfuzzer_args[*]}, total budget ${total_budget}s)"

# Refuse an impossible budget instead of discovering it by being killed. Exit 2 is a
# configuration error, deliberately distinct from 1 (crashes found) and 0 (clean), so
# a reader of the job cannot mistake arithmetic for a finding.
if [ "$mode" = explore ]; then
    ceiling_seconds=$(( ${FUZZ_CEILING_MINUTES:-22} * 60 ))
    if [ "$total_budget" -gt "$ceiling_seconds" ]; then
        echo "config error: FUZZ_TIME=${total_budget}s exceeds the ${FUZZ_CEILING_MINUTES:-22}m job ceiling (${ceiling_seconds}s)." >&2
        echo "Lower FUZZ_TIME, or raise FUZZ_CEILING_MINUTES and the job's timeout-minutes together." >&2
        {
            echo "## fuzz config error: $target"
            echo
            echo "FUZZ_TIME=${total_budget}s > ceiling ${ceiling_seconds}s (timeout-minutes minus build room)."
        } >> "$summary"
        exit 2
    fi
fi

cd "$(dirname "$0")/../fuzz"
shopt -s nullglob

mkdir -p "collected/$target"
: > "collected/$target/findings.tsv"

found=0
inputs=0
for round in $(seq 1 "$rounds"); do
    rm -rf "artifacts/$target"
    mkdir -p "artifacts/$target"
    log="${TMPDIR:-/tmp}/fuzz-$target-round$round.log"

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

    # Every artifact of this round is archived *before* anything else can happen,
    # including the rounds that find nothing new. The previous cut copied them only
    # when the signature was new, and otherwise broke out of the loop, so the second
    # input survived only because the `rm -rf` at the top of the next round had not run
    # yet — an un-copied finding is a finding nobody can minimise or seed.
    fresh=0
    for art in "${crashes[@]}"; do
        base=$(basename "$art")
        if [ -e "collected/$target/$base" ]; then
            continue
        fi
        cp "$art" "collected/$target/"
        printf '%s\t%s\n' "$signature" "$base" >> "collected/$target/findings.tsv"
        fresh=$((fresh + 1))
        found=$((found + 1))
        {
            printf '### %s, round %s\n\n' "$target" "$round"
            printf 'signature: `%s`\n\n' "$signature"
            printf -- '- `%s` (%s bytes)\n' "$base" "$(wc -c < "$art")"
            echo
        } >> "$summary"
        # Stop the next round from tripping on this exact input again.
        for c in "corpus/$target"/*; do
            cmp -s "$art" "$c" && rm -f "$c"
        done
    done
    inputs=$((inputs + fresh))
    if [ "$fresh" -eq 0 ]; then
        echo "round $round: nothing new (all ${#crashes[@]} artifact(s) already collected), stopping"
        break
    fi
done

if [ "$found" -gt 0 ]; then
    # Publish every collected crash under the job's artifact path, and fail once with
    # both counts. A signature names the assertion that failed, not the bug: several
    # distinct root causes can reach one `assert_eq!` on one line — measured with
    # crash-55c199ef and crash-5561902a, two root causes reported as "1 distinct crash
    # signature" — so the signature count is a lower bound and is never called a tally.
    cp -f "collected/$target"/crash-* "artifacts/$target/" 2>/dev/null || true
    sigs=$(cut -f1 "collected/$target/findings.tsv" | sort -u | grep -c .)
    echo "$target surfaced $inputs crash input(s) across $sigs distinct signature(s):"
    cut -f1 "collected/$target/findings.tsv" | sort | uniq -c | sort -rn | sed 's/^/  /'
    echo "::error::$target surfaced $inputs crash input(s) / $sigs distinct signature(s); inputs are in the fuzz-artifacts-$target artifact"
    exit 1
fi

echo "$target: no crash in $rounds round(s)"
