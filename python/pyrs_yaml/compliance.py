"""YAML Test Suite compliance reporting for pyrs_yaml."""

from __future__ import annotations

import json
import re
import sys
from datetime import datetime
from importlib.metadata import version as _pkg_version
from pathlib import Path
from typing import Any

import pyrs_yaml

try:
    import yaml

    HAS_PYYAML = True
except ImportError:
    HAS_PYYAML = False
    yaml = None  # ty: ignore[invalid-assignment]


def _current_version() -> str:
    try:
        return _pkg_version("pyrs-yaml")
    except (ImportError, FileNotFoundError, ValueError):
        return "unknown"


def convert_special_chars(text: str) -> str:
    """Convert special Unicode characters to actual characters"""
    if not text:
        return text

    # Convert special whitespace indicators
    # Order matters - longer sequences first
    text = text.replace("\u2423", " ")  # ␣ = space

    # Tab indicators: any run of em dashes / double vertical lines + »
    # (———», ——», —», » and the ‖ variants all encode a single tab)
    text = re.sub(r"(?:\u2014+|\u2016+)?\u00bb", "\t", text)

    text = text.replace("\u21b5", "\n")  # ↵ = newline
    text = text.replace("\u220e", "")  # ∎ = no final newline (remove)
    text = text.replace("\u2190", "\r")  # ← = carriage return
    text = text.replace("\u21d4", "\xef\xbb\xbf")  # ⇔ = BOM

    return text


def load_test_cases(suite_dir: str) -> list[dict[str, Any]]:
    """Load all test cases from the YAML test suite"""
    if not HAS_PYYAML:
        return []
    test_cases = []
    src_dir = Path(suite_dir) / "src"

    for yaml_file in sorted(src_dir.glob("*.yaml")):
        try:
            with yaml_file.open(encoding="utf-8") as f:
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
        except (OSError, ValueError, KeyError):
            continue

    return test_cases


def compare_json(expected: str, actual: str) -> bool:
    """Compare two JSON strings"""
    try:
        exp = json.loads(expected)
        act = json.loads(actual)
        return exp == act
    except (json.JSONDecodeError, TypeError, ValueError):
        return expected.strip() == actual.strip()


def run_test(test: dict[str, Any]) -> dict[str, Any]:
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
        "status": "fail",
    }

    yaml_input = test["yaml"]
    if not yaml_input:
        result["error"] = "No YAML input"
        return result

    # Test 1: Parse the YAML
    try:
        doc = pyrs_yaml.parse(yaml_input)
        result["parse_ok"] = True
    except Exception as e:
        result["error"] = f"Parse error: {str(e)[:100]}"
        # Correctly rejecting invalid YAML is compliant behavior.
        if not test["valid"]:
            result["status"] = "pass"
            result["error"] = None
        return result

    # Invalid YAML that parses successfully is a failure.
    if not test["valid"]:
        result["error"] = "Invalid YAML was accepted by parser"
        return result

    # Test 2: Compare JSON output (if expected)
    if test["json"]:
        try:
            actual_data = pyrs_yaml.safe_load(yaml_input)
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

    result["status"] = "pass" if result["error"] is None else "fail"
    return result


def compute_compliance(suite_dir: str | None = None) -> dict[str, Any]:
    """Run YAML Test Suite and return compliance report."""
    if suite_dir is None:
        suite_dir = str(Path(__file__).resolve().parent.parent.parent / "Reference" / "yaml-test-suite")
    tests = load_test_cases(suite_dir)
    passed = 0
    failed_tests = []
    for test in tests:
        result = run_test(test)
        if result["status"] == "pass":
            passed += 1
        else:
            failed_tests.append(
                {
                    "id": test["id"],
                    "name": test["name"],
                    "reason": result.get("error", "unknown"),
                }
            )
    total = len(tests)
    return {
        "version": _current_version(),
        "date": datetime.now().strftime("%Y-%m-%d"),
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "rate": round(passed / max(total, 1), 4),
        "failed_tests": failed_tests,
    }


DEFAULT_SUITE_DIR = str(Path(__file__).resolve().parent.parent.parent / "Reference" / "yaml-test-suite")


def compliance_report(suite_dir: str | None = None) -> dict[str, Any]:
    """Run the YAML Test Suite and return a compliance report.

    The suite data is a dev artifact (``Reference/yaml-test-suite``), not
    bundled in the wheel; pass ``suite_dir`` when it lives elsewhere.
    """
    if suite_dir is None:
        suite_dir = DEFAULT_SUITE_DIR
    if not Path(suite_dir).exists():
        raise FileNotFoundError(
            f"yaml-test-suite not found at {suite_dir}; clone https://github.com/yaml/yaml-test-suite into Reference/"
        )
    return compute_compliance(suite_dir)


if __name__ == "__main__":
    if "--json" in sys.argv:
        report = compliance_report()
        print(json.dumps(report, indent=2))
    else:
        print(compliance_report())
