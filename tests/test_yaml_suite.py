"""
YAML Test Suite Runner for pyyaml_rs
Runs the official YAML test suite and reports results
"""

import json
from pathlib import Path

import pytest
import yaml

import pyyaml_rs


def convert_special_chars(text: str) -> str:
    """Convert special Unicode characters to actual characters"""
    if not text:
        return text

    # Convert special whitespace indicators
    # Order matters - longer sequences first
    text = text.replace("\u2423", " ")  # ␣ = space

    # Tab indicators (em dashes + right angle bracket)
    text = text.replace("\u2014\u2014\u2014\u00bb", "\t")  # ———» = tab
    text = text.replace("\u2014\u2014\u00bb", "\t")  # ——» = tab
    text = text.replace("\u2014\u00bb", "\t")  # —» = tab
    text = text.replace("\u00bb", "\t")  # » = tab

    # Also handle double vertical line variants
    text = text.replace("\u2016\u2016\u2016\u00bb", "\t")  # ‖‖‖» = tab
    text = text.replace("\u2016\u2016\u00bb", "\t")  # ‖‖» = tab
    text = text.replace("\u2016\u00bb", "\t")  # ‖» = tab

    text = text.replace("\u21b5", "\n")  # ↵ = newline
    text = text.replace("\u220e", "")  # ∎ = no final newline (remove)
    text = text.replace("\u2190", "\r")  # ← = carriage return
    text = text.replace("\u21d4", "\xef\xbb\xbf")  # ⇔ = BOM

    return text


def load_test_cases(suite_dir: str) -> list:
    """Load all test cases from the YAML test suite"""
    test_cases = []
    src_dir = Path(suite_dir) / "src"

    for yaml_file in sorted(src_dir.glob("*.yaml")):
        try:
            with open(yaml_file, encoding="utf-8") as f:
                content = yaml.safe_load(f)

            if not content or not isinstance(content, list):
                continue

            for test in content:
                if not isinstance(test, dict):
                    continue

                test_id = yaml_file.stem
                name = test.get("name", "Unknown")
                tags = test.get("tags", "")
                yaml_input = test.get("yaml", "")
                json_expected = test.get("json", "")
                tree_expected = test.get("tree", "")
                dump_expected = test.get("dump", "")

                # Convert special characters in YAML input
                if yaml_input:
                    yaml_input = convert_special_chars(yaml_input)

                # Determine if test should be valid or invalid
                is_fail = test.get("fail", False)
                is_valid = "error" not in tags.lower() and "invalid" not in tags.lower() and not is_fail

                test_cases.append(
                    {
                        "id": test_id,
                        "name": name,
                        "tags": tags,
                        "yaml": yaml_input,
                        "json": json_expected.strip() if json_expected else None,
                        "tree": tree_expected.strip() if tree_expected else None,
                        "dump": dump_expected.strip() if dump_expected else None,
                        "valid": is_valid,
                    }
                )
        except Exception:
            continue

    return test_cases


def compare_json(expected: str, actual: str) -> bool:
    """Compare two JSON strings"""
    try:
        exp = json.loads(expected)
        act = json.loads(actual)
        return exp == act
    except Exception:
        return expected.strip() == actual.strip()


def run_test(test: dict) -> dict:
    """Run a single test case"""
    result = {
        "id": test["id"],
        "name": test["name"],
        "tags": test["tags"],
        "valid": test["valid"],
        "json": test.get("json"),
        "dump": test.get("dump"),
        "parse_ok": False,
        "json_match": False,
        "dump_match": False,
        "error": None,
    }

    yaml_input = test["yaml"]
    if not yaml_input:
        result["error"] = "No YAML input"
        return result

    # Test 1: Parse the YAML
    try:
        doc = pyyaml_rs.parse(yaml_input)
        result["parse_ok"] = True
    except Exception as e:
        result["error"] = f"Parse error: {str(e)[:100]}"
        return result

    # Test 2: Compare JSON output (if expected)
    if test["json"]:
        try:
            actual_data = pyyaml_rs.safe_load(yaml_input)
            actual_json = json.dumps(actual_data, sort_keys=True, indent=2)
            result["json_match"] = compare_json(test["json"], actual_json)
        except Exception as e:
            result["error"] = f"JSON comparison error: {str(e)[:100]}"

    # Test 3: Compare dump output (if expected)
    if test["dump"]:
        try:
            actual_dump = doc.to_yaml().strip()
            expected_dump = test["dump"].strip()
            result["dump_match"] = actual_dump == expected_dump
        except Exception as e:
            result["error"] = f"Dump comparison error: {str(e)[:100]}"

    return result


SUITE_DIR = "Reference/yaml-test-suite"


@pytest.mark.skipif(not Path(SUITE_DIR).exists(), reason="YAML Test Suite not found")
def test_yaml_suite_parse_rate():
    """Test that parse success rate meets threshold (>= 95% for valid tests)."""
    test_cases = load_test_cases(SUITE_DIR)
    assert len(test_cases) > 0, "No test cases loaded"

    results = [run_test(test) for test in test_cases]

    total = len(results)
    valid_tests = [r for r in results if r["valid"]]
    valid_parse_ok = sum(1 for r in valid_tests if r["parse_ok"])

    if valid_tests:
        rate = valid_parse_ok / len(valid_tests) * 100
        assert rate >= 95.0, f"Parse rate {rate:.1f}% below threshold 95%"
    else:
        pytest.skip("No valid test cases found")


@pytest.mark.skipif(not Path(SUITE_DIR).exists(), reason="YAML Test Suite not found")
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
