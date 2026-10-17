"""
YAML Test Suite Runner for pyrs_yaml
Runs the official YAML test suite and reports results
"""

from pathlib import Path

import pytest

import pyrs_yaml
from pyrs_yaml.compliance import compute_compliance, load_test_cases, run_test

try:
    import yaml

    HAS_PYYAML = True
except ImportError:
    HAS_PYYAML = False
    yaml = None


SUITE_DIR = "Reference/yaml-test-suite"


def numbered_cases(test_cases):
    """Pair every case with its position inside its own file.

    A suite id names a *file*, not a case, and 18 of its 351 files carry more than one (`ZYU8`
    alone carries four). Naming a residual by id would let one case start failing while another in
    the same file stopped failing and leave the named set untouched — which is the silence the
    percentage floors were built to keep.
    """
    counts: dict[str, int] = {}
    keyed = []
    for test in test_cases:
        counts[test["id"]] = counts.get(test["id"], 0) + 1
        keyed.append((f"{test['id']}#{counts[test['id']]}", test))
    return keyed


# The valid case the engine refuses on purpose. `ZYU8` is the `%YAML 1.3` directive family, and the
# standing ruling is to report the declared version and parse with 1.2 rules.
DELIBERATE_REFUSALS = frozenset({"ZYU8#3"})

# The cases whose loaded value the suite's JSON disagrees with. This is a ledger, not a tolerance:
# an entry leaves by teaching the engine the rule, never by adding a name to this set.
JSON_MISMATCHES = frozenset(
    {
        # An alias resolves to the last anchor of its name, not the nearest preceding one:
        # `collect_anchors` keeps one node per name, so a redefined anchor rewrites history.
        "3GZX#1",
        # Not a defect, and pinned below by `test_binary_spellings_construct_the_same_bytes`: the
        # suite spells a `!!binary` node as its source text, the comparison spells the same bytes
        # as canonical base64, and the two spellings differ by the block scalar's line breaks.
        "565N#1",
        # A keep-chomped block whose content lines are all empty keeps one line break too many...
        "JEF9#1",
        "K858#1",  # ...the same rule, asked of a mapping rather than a sequence item.
        # The non-specific tag `!` does not suppress implicit resolution, so `! 12` loads as an int.
        "S4JQ#1",
    }
)


@pytest.mark.skipif(not Path(SUITE_DIR).exists(), reason="YAML Test Suite not found")
@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
def test_yaml_suite_parse_rate():
    """Every valid case parses, except the one refused on purpose.

    The floor was `rate >= 95.0` over 312 valid cases — fifteen cases of slack around a measured
    311/312 — and a gate that passes at 297 cannot tell a deliberate refusal from a regression. A
    named set reddens in both directions: a case that stops being refused as loudly as one that
    starts.
    """
    test_cases = load_test_cases(SUITE_DIR)
    assert len(test_cases) > 0, "No test cases loaded"

    refused = {key for key, test in numbered_cases(test_cases) if test["valid"] and not run_test(test)["parse_ok"]}
    assert refused == DELIBERATE_REFUSALS, (
        f"valid cases newly refused: {sorted(refused - DELIBERATE_REFUSALS)}; "
        f"deliberate refusals now accepted: {sorted(DELIBERATE_REFUSALS - refused)}"
    )


@pytest.mark.skipif(not Path(SUITE_DIR).exists(), reason="YAML Test Suite not found")
@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
def test_yaml_suite_invalid_rejected():
    """No invalid case may parse.

    The floor was `rate >= 90.0` over 94 invalid cases — nine acceptances of slack around a
    measured 94/94. The suite's invalid corpus is the parser's rejection specification, so an
    accepted member is a defect however many others it keeps, and zero is the only honest line.
    """
    accepted = {
        key
        for key, test in numbered_cases(load_test_cases(SUITE_DIR))
        if not test["valid"] and run_test(test)["parse_ok"]
    }
    assert not accepted, f"invalid YAML accepted by the parser: {sorted(accepted)}"


@pytest.mark.skipif(not Path(SUITE_DIR).exists(), reason="YAML Test Suite not found")
@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
def test_yaml_suite_json_match():
    """The five cases whose JSON disagrees, named — and no others.

    The floor was `rate >= 80.0` over 267 JSON cases — fifty-three of slack around a measured 244.
    Most of that slack was the harness rather than the engine: a multi-document stream's
    expectation is several JSON values in one field, `json.loads` refused the second, and the
    comparison fell back to comparing text, so eighteen stream cases were never judged. Comparing
    one document at a time (`json_documents`) moved the measurement to 262/267 and left exactly the
    five causes above, four of them open defects and one a spelling difference pinned on its own.
    """
    mismatched = set()
    for key, test in numbered_cases(load_test_cases(SUITE_DIR)):
        result = run_test(test)
        if result["json"] and not result["json_match"]:
            mismatched.add(key)
    assert mismatched == JSON_MISMATCHES, (
        f"cases newly mismatching: {sorted(mismatched - JSON_MISMATCHES)}; "
        f"ledger entries now matching: {sorted(JSON_MISMATCHES - mismatched)}"
    )


@pytest.mark.skipif(not Path(SUITE_DIR).exists(), reason="YAML Test Suite not found")
@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
def test_binary_spellings_construct_the_same_bytes():
    """Why `565N` may sit in the ledger without being an engine defect.

    The suite spells a `!!binary` node as the scalar's own text, so the quoted form — whose line
    breaks were escaped away — and the literal block — whose breaks *are* content — differ by those
    newlines, and a base64 of the loaded bytes erases the difference. What the two forms do have to
    agree on is the value, and that is the question this asks.
    """
    case = next(test for test in load_test_cases(SUITE_DIR) if test["id"] == "565N")
    loaded = pyrs_yaml.safe_load(case["yaml"])
    assert isinstance(loaded["canonical"], bytes)
    assert loaded["canonical"] == loaded["generic"], "the two binary spellings constructed apart"


@pytest.mark.skipif(not Path(SUITE_DIR).exists(), reason="YAML Test Suite not found")
@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
def test_compliance_report():
    """The suite number the docs advertise, enforced rather than printed.

    The floor used to be `rate >= 0.95`, which is 385/406 — twenty-one suite cases could rot
    inside it, and three did: the alias-in-key refusal and the `!!binary` JSON comparison pulled
    the report down to 402 while `docs/en/features.md` and `docs/en/index.md` still advertised
    405 (99.75%). Nothing reddened, because no gate read the number the marketing page quotes.
    So: a floor tight enough to notice a three-case slide, and every failure must be the one
    refused on purpose. An exact `passed == 405` would not do — CI clones the suite's default
    branch (`ci.yml`), so the case count is upstream's to change.
    """
    report = compute_compliance()
    msg = f"\nCompliance: {report['rate'] * 100:.2f}% ({report['passed']}/{report['total']} passed)"
    print(msg)
    assert report["rate"] >= 0.995, f"Compliance too low: {report['rate'] * 100:.2f}%"
    unexpected = [test["id"] for test in report["failed_tests"] if test["id"] != "ZYU8"]
    assert not unexpected, f"suite cases refused that are not the deliberate refusal: {unexpected}"


def test_compliance_report_version_is_dynamic():
    from pyrs_yaml import __version__, compliance_report

    if not Path(SUITE_DIR).exists():
        pytest.skip("YAML Test Suite not found")
    report = compliance_report()
    assert report["version"] == __version__


def test_compliance_report_missing_data_raises(tmp_path):
    from pyrs_yaml import compliance_report

    with pytest.raises(FileNotFoundError):
        compliance_report(str(tmp_path / "nonexistent"))


# ── pinned per-case strictness gates ─────────────────────────────────────────
# The suite gates above need the untracked ``Reference/`` corpus and skip without
# it, and they name their residuals case by case so a slide reddens a run. These
# gates are the same claims made on the LITERAL input: they run in every
# environment and fail on any drift. (They were written when the suite gates were
# threshold-gated at >= 95%, which drowned a single-case regression — that is
# exactly how granit-parser 1.3 shipped the `9C9N` acceptance bug past green CI.)

# yaml-test-suite 9C9N "Wrong indented flow sequence": a multi-line flow
# collection used as a block value whose continuation line is indented no
# further than the block key. Invalid YAML; must be rejected.
WRONGLY_INDENTED_FLOW = "flow: [a,\nb,\nc]\n"


def test_9c9n_wrongly_indented_flow_is_rejected():
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.parse(WRONGLY_INDENTED_FLOW)


def test_properly_indented_flow_continuation_is_accepted():
    # The guard must not over-reject: the same flow, correctly out-dented,
    # is valid and parses to the expected list.
    assert pyrs_yaml.safe_load("flow: [a,\n  b,\n  c]\n") == {"flow": ["a", "b", "c"]}
