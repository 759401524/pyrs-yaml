#!/usr/bin/env python3
"""Exercise the shipped wheel on an interpreter the suite tier does not reach.

`ci.yml`'s `wheel-smoke` job runs this after installing the abi3 wheel that job's platform built, for
every (OS, interpreter) cell the suite tier demoted. The claim under test is the one in
`docs/en/installation.md` - a single abi3 wheel from Python 3.8 through 3.15 on three platforms - and
that claim is per-cell: it fails on the cell where the loader refuses the artefact, not on the cell
where 2 500 assertions happen to be re-run.

So this file does what a first session with the package does, in each of the five formats the project
owns, and nothing more: import, parse, round-trip with the metadata that makes a round trip worth
having (a comment and an anchor), then the JSON family and TOML out and back. A print at the end so the
job log names the interpreter rather than only its exit code.

Kept in a file rather than inline in the workflow because a `python -c "..."` inside a YAML plain
scalar is not valid YAML - `a: 1` in the program is parsed as a mapping, measured when this job first
ran. That is also why it is a `scripts/*.py`: the 3.8 leg imports every one of those, so the program
cannot quietly become 3.9-only syntax.
"""

from __future__ import annotations

import sys

import pyrs_yaml

YAML_DOC = "a: 1\nb: [1, 2]\n# note\n"
EXPECTED = {"a": 1, "b": [1, 2]}


DIALECTS = (("to_json", "load_json"), ("to_jsonc", "load_jsonc"), ("to_json5", "load_json5"))


def main() -> int:
    doc = pyrs_yaml.parse(YAML_DOC)
    round_one = doc.to_yaml()

    # What is asserted is survival and a fixed point, not byte equality with the input. Measured on the
    # first run of this script: a standalone `# note` on its own line at the end of a short document
    # comes back attached to the previous member (`b: [1, 2]  # note`), so the text is not identical
    # while nothing is lost. Demanding identity here would have made the smoke job red on behaviour
    # that the round-trip tier already characterises deliberately; demanding that the note survives and
    # that a second pass changes nothing is the part a per-cell wheel check should hold.
    if "# note" not in round_one or pyrs_yaml.parse(round_one).to_yaml() != round_one:
        print(f"YAML round trip lost or did not settle: {YAML_DOC!r} -> {round_one!r}", file=sys.stderr)
        return 1

    for writer, loader in DIALECTS:
        emitted = getattr(doc, writer)()
        loaded = getattr(pyrs_yaml, loader)(emitted)
        if loaded != EXPECTED:
            print(
                f"{writer}() does not read back as {EXPECTED!r} via {loader}: {emitted!r}",
                file=sys.stderr,
            )
            return 1

    toml_text = pyrs_yaml.to_toml(round_one)
    if pyrs_yaml.load_toml(toml_text) != EXPECTED:
        print(f"to_toml/load_toml lost something: {toml_text!r}", file=sys.stderr)
        return 1

    # Anchors and aliases are the other half of the round-trip promise, and the cheapest thing a
    # mis-built wheel gets wrong: they live in the AST's metadata, not in the scalar path.
    anchored = pyrs_yaml.parse("d: &d\n  x: 1\ne: *d\n")
    if "d: &d" not in anchored.to_yaml():
        print(f"anchor lost: {anchored.to_yaml()!r}", file=sys.stderr)
        return 1

    print(f"wheel smoke ok: pyrs_yaml {pyrs_yaml.__version__} on Python {sys.version.split()[0]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
