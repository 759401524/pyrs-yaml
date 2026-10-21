"""JSON Schema draft 2020-12, replayed through the public ``Document.validate`` surface.

The evaluation itself is not this repository's code: `Document.validate` hands the
instance and the schema to the reference implementation (``jsonschema``, a test-group
dependency on every tested leg), so the rules being judged are owned by
json-schema.org rather than by this tree. What this file judges is therefore the part
this repository does own — the YAML/JSON document becoming the Python instance the
engine sees (``to_dict``, including context and plugin resolution), the schema
acceptance (text or dict) and compiled-validator cache, and the ``YamlValidateError``
contract. Replaying an official corpus through that surface is the only tier that can
disagree with it: the unit, property and fuzz tiers above it all ask this engine what
it believes.

Measured 2026-10-21 on ``jsonschema 4.26.0`` at corpus commit
``0b4a7fa5e79c5e1ed60d5c034b951003bbba7fd4``: 46 files, 1302 cases. **1261 of them
pass, and the 41 that do not are named below** — 31 remote-retrieval cases excluded as
a contract boundary and 10 as engine boundaries. The same corpus replayed through
``jsonschema.validate()`` directly, without this package, reaches the identical
verdict on all 1302 cases, so **no exclusion is attributable to the conversion**: the
verdicts this gate checks are the engine's, transported without distortion.

The exclusions are keyed on the installed reference version, because the lock resolves
three of them: 4.23.0 on the Python 3.8 leg, 4.25.1 on 3.9 and 4.26.0 above. 4.23.0
needs one more exclusion, named under ``LEGACY_EXCLUDED_CASES``; 4.25.1 and 4.26.0
agree with the corpus on that case. Whichever set applies is still asserted in both
directions, so a version on which the legacy boundary moved reddens the run.

The exclusions are asserted in both directions. A named case that starts agreeing with
the corpus reddens the run, because the exclusion went stale; a case outside the list
that stops agreeing reddens it by its own id. Nothing here is a percentage: 1261/1302
would let a whole keyword family disappear behind a green ratio.

Why each exclusion exists:

* ``refRemote.json`` — the schema language's ``$ref`` is resolved locally; remote HTTP
  retrieval is not part of the shipped contract and ``Document.validate`` exposes no
  resolver. One representative valid case is probed, because one attempt at the absent
  resolver is what keeps this exclusion live; probing all sixteen waits out the same
  four-second connection timeout sixteen times. The 15 cases that expect an invalid
  instance are not credited at all: with no resolver the reference raises, and matching
  "invalid" that way is a vacuous pass, not evidence.
* ``dynamicRef.json`` (5) and ``vocabulary.json`` (1) — the reference implementation
  does not implement those two 2020-12 features, measured by failing them identically
  without this package.
* ``pattern.json`` / ``patternProperties.json`` (2 + 2) — ECMAScript ``\\p{…}`` Unicode
  property escapes are not supported by Python's ``re``, which the engine uses for
  ``pattern``.
"""

from __future__ import annotations

import json
import re
import subprocess
from importlib.metadata import version
from pathlib import Path

import pytest

import pyrs_yaml

CORPUS_ROOT = "Reference/JSON-Schema-Test-Suite"
SUITE_DIR = Path(CORPUS_ROOT) / "tests" / "draft2020-12"

#: The corpus is pinned by the CI fetch step and by this commit. A conformance result
#: describes one snapshot of one corpus, so an unpinned clone is a different claim.
CORPUS_COMMIT = "0b4a7fa5e79c5e1ed60d5c034b951003bbba7fd4"
CORPUS_FILES = 46
CORPUS_CASES = 1302

#: Remote retrieval is a contract boundary, not a defect: skip every case in this file,
#: but require the cases that expect a valid instance to keep failing so the exclusion
#: cannot outlive the boundary it describes.
REMOTE_FILE = "refRemote.json"
REMOTE_CASES = 31
REMOTE_VALID_CASES = 16

#: Engine boundaries. Each id is ``(file, group description, test description)``.
EXCLUDED_CASES = frozenset(
    {
        (
            "dynamicRef.json",
            "strict-tree schema, guards against misspelled properties",
            "instance with correct field",
        ),
        (
            "dynamicRef.json",
            "tests for implementation dynamic anchor and reference link",
            "correct extended schema",
        ),
        (
            "dynamicRef.json",
            "$ref and $dynamicAnchor are independent of order - $defs first",
            "correct extended schema",
        ),
        (
            "dynamicRef.json",
            "$ref and $dynamicAnchor are independent of order - $ref first",
            "correct extended schema",
        ),
        (
            "dynamicRef.json",
            "$ref to $dynamicRef finds detached $dynamicAnchor",
            "number is valid",
        ),
        (
            "pattern.json",
            "pattern with Unicode property escape requires unicode mode",
            "ASCII letters match",
        ),
        (
            "pattern.json",
            "pattern with Unicode property escape requires unicode mode",
            "Non-ASCII letters match",
        ),
        (
            "patternProperties.json",
            "patternProperties with Unicode property escape",
            "Unicode letter property name matches",
        ),
        (
            "patternProperties.json",
            "patternProperties with Unicode property escape",
            "Non-letter property name does not match pattern",
        ),
        (
            "vocabulary.json",
            "schema that uses custom metaschema with with no validation vocabulary",
            "no validation: invalid number, but it still validates",
        ),
    }
)

#: `jsonschema` 4.23.0 - the lock's Python 3.8 resolution - loses the annotation of a
#: non-boolean ``additionalProperties`` schema, so a sibling-evaluated property counts
#: as unevaluated. Measured: 4.23.0 disagrees, 4.25.1 and 4.26.0 agree. The bound below
#: is the 3.9 resolution; no leg between the two installs another version.
LEGACY_EXCLUDED_CASES = frozenset(
    {
        (
            "unevaluatedProperties.json",
            "unevaluatedProperties with adjacent non-bool additionalProperties",
            "with additional properties",
        ),
    }
)

#: One representative valid case from the remote file, solved by ``$ref`` alone. The
#: boundary is that no resolver is exposed at all, so a single attempt is the evidence;
#: each attempt waits out a connection timeout to ``localhost:1234``.
REMOTE_PROBE = ("remote ref", "remote ref valid")


def _reference_version() -> tuple[int, ...]:
    match = re.match(r"(\d+)\.(\d+)\.(\d+)", version("jsonschema"))
    assert match, "jsonschema's version is not numeric, so the exclusion set cannot be keyed"
    return tuple(int(part) for part in match.groups())


def _excluded_cases() -> frozenset[tuple[str, str, str]]:
    """The exclusions that apply to the installed reference implementation."""
    if _reference_version() < (4, 25, 1):
        return EXCLUDED_CASES | LEGACY_EXCLUDED_CASES
    return EXCLUDED_CASES


pytestmark = pytest.mark.skipif(
    not SUITE_DIR.is_dir(),
    reason=(
        "JSON Schema Test Suite not found (clone "
        "https://github.com/json-schema-org/JSON-Schema-Test-Suite into Reference/ at the "
        "recorded commit; CI does this for you)"
    ),
)


def _files() -> list[Path]:
    return sorted(SUITE_DIR.glob("*.json"))


def _groups(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def _case_id(path: Path, group: dict, case: dict) -> tuple[str, str, str]:
    return (path.name, group["description"], case["description"])


def _passes(document, schema) -> bool:
    try:
        document.validate(schema)
    except pyrs_yaml.YamlValidateError:
        return False
    return True


@pytest.fixture(scope="module", autouse=True)
def corpus_is_the_recorded_snapshot() -> None:
    """The result names a corpus and a version, so the version is checked, not assumed."""
    if not (Path(CORPUS_ROOT) / ".git").exists():
        pytest.fail(
            f"{CORPUS_ROOT} has no git metadata, so the recorded commit cannot be checked - "
            "clone it (CI does) instead of copying the files"
        )
    head = subprocess.run(
        ["git", "-C", CORPUS_ROOT, "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
        check=False,
    )
    assert head.returncode == 0, head.stderr
    assert head.stdout.strip() == CORPUS_COMMIT, (
        f"the corpus is at {head.stdout.strip()}, the recorded result is for {CORPUS_COMMIT}; "
        "re-run the measurement and update the exclusions before moving the pin"
    )
    files = _files()
    assert len(files) == CORPUS_FILES, f"{len(files)} corpus files, the recorded shape has {CORPUS_FILES}"
    cases = sum(len(group["tests"]) for path in files for group in _groups(path))
    assert cases == CORPUS_CASES, f"{cases} corpus cases, the recorded shape has {CORPUS_CASES}"


def test_every_named_exclusion_is_a_case_in_the_corpus() -> None:
    """A typo in an exclusion id would silently excuse nothing, so the ids are resolved."""
    seen = {_case_id(path, group, case) for path in _files() for group in _groups(path) for case in group["tests"]}
    excluded = _excluded_cases()
    assert not (excluded - seen), sorted(excluded - seen)


@pytest.mark.parametrize("path", _files(), ids=lambda path: path.name)
def test_corpus_file(path: Path) -> None:
    problems = []
    stale = []
    if path.name == REMOTE_FILE:
        cases = [case for group in _groups(path) for case in group["tests"]]
        assert len(cases) == REMOTE_CASES, f"{path.name} has {len(cases)} cases, recorded {REMOTE_CASES}"
        valid = [case for case in cases if case["valid"]]
        assert len(valid) == REMOTE_VALID_CASES, (
            f"{path.name} has {len(valid)} valid cases, recorded {REMOTE_VALID_CASES}"
        )
        probe = [
            (group, case)
            for group in _groups(path)
            for case in group["tests"]
            if (group["description"], case["description"]) == REMOTE_PROBE
        ]
        assert probe, f"{REMOTE_PROBE} is not a case in {path.name}"
        group, case = probe[0]
        assert case["valid"], f"{REMOTE_PROBE} is not a case the corpus expects to be valid"
        document = pyrs_yaml.parse(json.dumps(case["data"]))
        assert not _passes(document, group["schema"]), (
            "remote retrieval is now resolving, so this whole-file exclusion is stale: "
            f"re-run the measurement and credit {path.name} instead of excluding it"
        )
        return

    excluded = _excluded_cases()
    for group in _groups(path):
        schema = group["schema"]
        for case in group["tests"]:
            case_id = _case_id(path, group, case)
            document = pyrs_yaml.parse(json.dumps(case["data"]))
            got = _passes(document, schema)
            if case_id in excluded:
                if got == case["valid"]:
                    stale.append(case_id)
            elif got != case["valid"]:
                problems.append(f"{case_id}: expected valid={case['valid']}, got valid={got}")

    lines = []
    if problems:
        lines.append(
            f"{len(problems)} case(s) the corpus judges differently - fix the surface or name the case "
            "in EXCLUDED_CASES with the engine boundary it hits:"
        )
        lines.extend(f"  {problem}" for problem in problems)
    if stale:
        lines.append(
            f"{len(stale)} named exclusion(s) now agree with the corpus - the boundary moved, so "
            "remove the exclusion and credit the case:"
        )
        lines.extend(f"  {stale_case}" for stale_case in stale)
    assert not lines, "\n".join(lines)
