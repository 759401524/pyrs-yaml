"""Assert the committed type stub equals what the declared regeneration route derives.

`python/pyrs_yaml/pyrs_yaml.pyi` ships inside every wheel and is the public typing
contract, but the file is machine output: AGENTS.md and `ruff.toml` both forbid
hand-editing it. Before this check the only stub assertions in CI were existence
ones (`git ls-files --error-unmatch` + `test -f py.typed`), so a binding signature
change could leave the committed stub silently behind until a user's type checker
noticed.

This script compares the *tracked* stub against a freshly generated one and exits
non-zero on any difference. Two transforms stand between raw generator output and
the bytes the repository legitimately stores; both are applied here so that the
tracked file stays fully derived instead of being patched by hand:

1. Trailing whitespace is stripped per line. `prek.toml` runs the built-in
   `trailing-whitespace` hook on commit, and the generator emits whitespace-only
   lines inside docstrings, so the committed form is the stripped form. Ignoring
   this would keep the check permanently red for a non-semantic reason.
2. `FIDELITY_FIXES` rewrites returns the generator is known to model unfaithfully.
   maturin 1.14.1 special-cases `__next__` and emits only the iterator's yield
   type, dropping the `Option` the bindings actually return
   (`PyResult<Option<Bound<'a, PyDict>>>` in `crates/pyrs-yaml/src/py/`). The
   generator gets every other `Option` right (`get_plugin`, `parse_stream`), so
   this is a narrow generator gap, not a project convention. Each fix declares how
   many times it must match; an unexpected count fails instead of silently
   rewriting, so a changed return type or a fixed upstream forces a review.

Usage:
    uv run maturin generate-stubs --out target/stubs
    python scripts/check_stub_drift.py            # verify only
    python scripts/check_stub_drift.py --fix      # write the derived stub

Exit code 0 = in sync; 1 = drift; 2 = the generated stub is missing or unusable.
"""

import argparse
import difflib
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GENERATED = ROOT / "target" / "stubs" / "pyrs_yaml.pyi"
TRACKED = ROOT / "python" / "pyrs_yaml" / "pyrs_yaml.pyi"

# Anchored on the generator's exact output so a real signature change stops
# matching (and fails) rather than being rewritten into something untrue.
NEXT_RETURN = re.compile(r"^(\s*)def __next__\(self, /\) -> dict(: \.\.\.|:)$", re.MULTILINE)

FIDELITY_FIXES = (
    {
        "name": "next-return-drops-optional",
        "pattern": NEXT_RETURN,
        "expected_matches": 2,
        "template": r"\g<1>def __next__(self, /) -> dict |None\g<2>",
        "reason": (
            "maturin 1.14.1 emits the yield type for __next__ and drops the Option "
            "the binding returns; both __next__ methods return dict | None."
        ),
    },
)

MAX_DIFF_LINES = 120


def display(path: Path) -> str:
    """Render a path relative to the repository when it lives inside it."""
    resolved = path.resolve()
    try:
        return str(resolved.relative_to(ROOT))
    except ValueError:  # caller passed a path outside the repository
        return str(resolved)


def normalize(text: str) -> str:
    """Strip trailing whitespace per line, matching prek's trailing-whitespace hook."""
    return "\n".join(line.rstrip() for line in text.split("\n"))


def derived_text(generated: str) -> tuple[str, list[str]]:
    """Turn raw generator output into the bytes the repository is expected to hold.

    Args:
        generated: the text of a freshly generated stub.

    Returns:
        The normalized text and a list of problems found while applying the
        declared fidelity fixes. A non-empty problem list means the caller must
        fail: the rewrite was not applied for that fix.
    """
    problems: list[str] = []
    text = normalize(generated)
    for fix in FIDELITY_FIXES:
        matches = fix["pattern"].findall(text)
        if len(matches) != fix["expected_matches"]:
            problems.append(
                f"fidelity fix {fix['name']!r} matched {len(matches)} site(s), "
                f"expected {fix['expected_matches']}: {fix['reason']} "
                "Review the fix declaration before trusting the comparison."
            )
            continue
        text = fix["pattern"].sub(fix["template"], text)
    return text, problems


def read_text(path: Path, hint: str) -> str:
    if not path.is_file():
        print(f"ERROR: {display(path)} not found. {hint}", file=sys.stderr)
        raise SystemExit(2)
    # Line endings are not part of the contract this gate checks. The generator writes
    # with the platform default, so a Windows box produced CRLF, `--fix` wrote that back
    # over the committed stub, and the next Linux run saw the whole file as drift - 597
    # lines of churn with no content change, and a PR gate that only agreed with whoever
    # ran it last. Normalize on read, and write LF on the fix path.
    return path.read_text(encoding="utf-8").replace("\r\n", "\n")


def report_drift(expected: str, actual: str, tracked: Path) -> int:
    diff = list(
        difflib.unified_diff(
            expected.split("\n"),
            actual.split("\n"),
            fromfile="derived (maturin generate-stubs + declared fixes)",
            tofile=display(tracked),
            lineterm="",
        )
    )
    print(
        f"ERROR: {display(tracked)} is not what the regeneration route derives "
        f"({sum(1 for line in diff if line.startswith('+'))} added, "
        f"{sum(1 for line in diff if line.startswith('-'))} removed).",
        file=sys.stderr,
    )
    for line in diff[:MAX_DIFF_LINES]:
        print(line, file=sys.stderr)
    if len(diff) > MAX_DIFF_LINES:
        print(f"... ({len(diff) - MAX_DIFF_LINES} more diff lines)", file=sys.stderr)
    print(
        "\nFix by running the declared route, not by editing the stub:\n"
        "  uv run maturin generate-stubs --out target/stubs\n"
        "  python scripts/check_stub_drift.py --fix",
        file=sys.stderr,
    )
    return 1


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Verify the committed type stub matches what the regeneration route derives."
    )
    parser.add_argument(
        "--fix",
        action="store_true",
        help="write the derived stub to the tracked path instead of only verifying it",
    )
    parser.add_argument("--generated", type=Path, default=GENERATED, help="generated stub to compare from")
    parser.add_argument("--tracked", type=Path, default=TRACKED, help="committed stub to compare against")
    args = parser.parse_args()

    hint = "Run: uv run maturin generate-stubs --out target/stubs"
    expected, problems = derived_text(read_text(args.generated, hint))
    if problems:
        for problem in problems:
            print(f"ERROR: {problem}", file=sys.stderr)
        return 2

    actual = normalize(read_text(args.tracked, "It should be committed; check .gitignore."))
    if args.fix:
        if actual != expected:
            args.tracked.write_text(expected, encoding="utf-8", newline="\n")
            print(f"Updated {display(args.tracked)} from the regeneration route.")
        else:
            print(f"{display(args.tracked)} already matches the regeneration route.")
        return 0

    if actual == expected:
        print(f"{display(args.tracked)} matches the regeneration route.")
        return 0
    return report_drift(expected, actual, args.tracked)


if __name__ == "__main__":
    sys.exit(main())
