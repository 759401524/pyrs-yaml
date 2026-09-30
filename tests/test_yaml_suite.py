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


@pytest.mark.skipif(not Path(SUITE_DIR).exists(), reason="YAML Test Suite not found")
@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
def test_yaml_suite_parse_rate():
    """Test that parse success rate meets threshold (>= 95% for valid tests)."""
    test_cases = load_test_cases(SUITE_DIR)
    assert len(test_cases) > 0, "No test cases loaded"

    results = [run_test(test) for test in test_cases]

    _ = len(results)
    valid_tests = [r for r in results if r["valid"]]
    valid_parse_ok = sum(1 for r in valid_tests if r["parse_ok"])

    if valid_tests:
        rate = valid_parse_ok / len(valid_tests) * 100
        assert rate >= 95.0, f"Parse rate {rate:.1f}% below threshold 95%"
    else:
        pytest.skip("No valid test cases found")


@pytest.mark.skipif(not Path(SUITE_DIR).exists(), reason="YAML Test Suite not found")
@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
def test_yaml_suite_invalid_rejected():
    """Test that invalid YAML is correctly rejected."""
    test_cases = load_test_cases(SUITE_DIR)

    invalid_tests = [t for t in test_cases if not t["valid"]]
    rejected = 0
    for test in invalid_tests:
        result = run_test(test)
        if not result["parse_ok"]:
            rejected += 1

    if invalid_tests:
        rate = rejected / len(invalid_tests) * 100
        assert rate >= 90.0, f"Invalid rejection rate {rate:.1f}% below threshold 90%"
    else:
        pytest.skip("No invalid test cases found")


@pytest.mark.skipif(not Path(SUITE_DIR).exists(), reason="YAML Test Suite not found")
@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
def test_yaml_suite_json_match():
    """Test JSON comparison against expected output."""
    test_cases = load_test_cases(SUITE_DIR)
    results = [run_test(test) for test in test_cases]

    json_results = [r for r in results if r["json"]]
    json_match = sum(1 for r in json_results if r["json_match"])

    if json_results:
        rate = json_match / len(json_results) * 100
        assert rate >= 80.0, f"JSON match rate {rate:.1f}% below threshold 80%"
    else:
        pytest.skip("No JSON comparison results found")


@pytest.mark.skipif(not Path(SUITE_DIR).exists(), reason="YAML Test Suite not found")
@pytest.mark.skipif(not HAS_PYYAML, reason="PyYAML not installed")
def test_compliance_report():
    """Print compliance percentage to stdout."""
    report = compute_compliance()
    msg = f"\nCompliance: {report['rate'] * 100:.1f}% ({report['passed']}/{report['total']} passed)"
    print(msg)
    assert report["rate"] >= 0.95, f"Compliance too low: {report['rate'] * 100:.1f}%"


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
# The suite tests above are threshold-gated (>= 95%), so a single-case
# regression is silently drowned (that is exactly how granit-parser 1.3 shipped
# the `9C9N` acceptance bug past green CI). These gates assert individual cases
# on the LITERAL input, independent of the untracked ``Reference/`` corpus, so
# they run in every environment and fail on any drift.

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
