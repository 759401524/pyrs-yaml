"""
YAML Test Suite Runner for pyamlium_custom
Runs the official YAML test suite and reports results
"""

import os
import json
import yaml
import pyamlium_custom
from pathlib import Path


def convert_special_chars(text: str) -> str:
    """Convert special Unicode characters to actual characters"""
    if not text:
        return text

    # Convert special whitespace indicators
    # Order matters - longer sequences first
    text = text.replace('\u2423', ' ')  # ␣ = space
    
    # Tab indicators (em dashes + right angle bracket)
    text = text.replace('\u2014\u2014\u2014\u00bb', '\t')  # ———» = tab
    text = text.replace('\u2014\u2014\u00bb', '\t')  # ——» = tab
    text = text.replace('\u2014\u00bb', '\t')  # —» = tab
    text = text.replace('\u00bb', '\t')  # » = tab
    
    # Also handle double vertical line variants
    text = text.replace('\u2016\u2016\u2016\u00bb', '\t')  # ‖‖‖» = tab
    text = text.replace('\u2016\u2016\u00bb', '\t')  # ‖‖» = tab
    text = text.replace('\u2016\u00bb', '\t')  # ‖» = tab
    
    text = text.replace('\u21b5', '\n')  # ↵ = newline
    text = text.replace('\u221e', '')  # ∎ = no final newline (remove)
    text = text.replace('\u2190', '\r')  # ← = carriage return
    text = text.replace('\u21d4', '\xef\xbb\xbf')  # ⇔ = BOM

    return text


def load_test_cases(suite_dir: str) -> list:
    """Load all test cases from the YAML test suite"""
    test_cases = []
    src_dir = Path(suite_dir) / "src"

    for yaml_file in sorted(src_dir.glob("*.yaml")):
        try:
            with open(yaml_file, "r", encoding="utf-8") as f:
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
                # Check both tags and the fail field
                is_fail = test.get("fail", False)
                is_valid = "error" not in tags.lower() and "invalid" not in tags.lower() and not is_fail

                test_cases.append({
                    "id": test_id,
                    "name": name,
                    "tags": tags,
                    "yaml": yaml_input,
                    "json": json_expected.strip() if json_expected else None,
                    "tree": tree_expected.strip() if tree_expected else None,
                    "dump": dump_expected.strip() if dump_expected else None,
                    "valid": is_valid,
                })
        except Exception as e:
            print(f"Error loading {yaml_file}: {e}")

    return test_cases


def normalize_json(json_str: str) -> str:
    """Normalize JSON for comparison"""
    if not json_str:
        return ""
    try:
        data = json.loads(json_str)
        return json.dumps(data, sort_keys=True, indent=2)
    except:
        return json_str.strip()


def compare_json(expected: str, actual: str) -> bool:
    """Compare two JSON strings"""
    try:
        exp = json.loads(expected)
        act = json.loads(actual)
        return exp == act
    except:
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
        "tree_match": False,
        "dump_match": False,
        "error": None,
    }

    yaml_input = test["yaml"]
    if not yaml_input:
        result["error"] = "No YAML input"
        return result

    # Test 1: Parse the YAML
    try:
        doc = pyamlium_custom.parse(yaml_input)
        result["parse_ok"] = True
    except Exception as e:
        result["error"] = f"Parse error: {str(e)[:100]}"
        return result

    # Test 2: Compare JSON output (if expected)
    if test["json"]:
        try:
            # Use safe_load to get a dict
            actual_data = pyamlium_custom.safe_load(yaml_input)
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


def main():
    suite_dir = "Reference/yaml-test-suite"

    print("=" * 70)
    print("YAML Test Suite Runner for pyamlium_custom")
    print("=" * 70)

    # Load test cases
    test_cases = load_test_cases(suite_dir)
    print(f"\nLoaded {len(test_cases)} test cases")

    # Run tests
    results = []
    for i, test in enumerate(test_cases):
        if (i + 1) % 50 == 0:
            print(f"  Running test {i+1}/{len(test_cases)}...")
        result = run_test(test)
        results.append(result)

    # Count results
    total = len(results)
    parse_ok = sum(1 for r in results if r["parse_ok"])
    parse_fail = total - parse_ok
    json_match = sum(1 for r in results if r["json_match"])
    json_total = sum(1 for r in results if r["json"])
    dump_match = sum(1 for r in results if r["dump_match"])
    dump_total = sum(1 for r in results if r["dump"])

    # Separate valid and invalid tests
    valid_tests = [r for r in results if r["valid"]]
    invalid_tests = [r for r in results if not r["valid"]]

    valid_parse_ok = sum(1 for r in valid_tests if r["parse_ok"])
    invalid_parse_ok = sum(1 for r in invalid_tests if r["parse_ok"])

    print("\n" + "=" * 70)
    print("RESULTS")
    print("=" * 70)

    print(f"\nTotal tests: {total}")
    print(f"  Valid tests: {len(valid_tests)}")
    print(f"  Invalid tests: {len(invalid_tests)}")

    print(f"\nParse Results:")
    print(f"  Parse OK: {parse_ok}/{total} ({parse_ok/total*100:.1f}%)")
    print(f"  Parse Fail: {parse_fail}/{total}")

    print(f"\nValid Tests (should parse):")
    print(f"  Parse OK: {valid_parse_ok}/{len(valid_tests)} ({valid_parse_ok/len(valid_tests)*100:.1f}%)")

    print(f"\nInvalid Tests (should fail):")
    print(f"  Parse OK (incorrect): {invalid_parse_ok}/{len(invalid_tests)} ({invalid_parse_ok/len(invalid_tests)*100:.1f}%)")

    if json_total > 0:
        print(f"\nJSON Comparison:")
        print(f"  Match: {json_match}/{json_total} ({json_match/json_total*100:.1f}%)")

    if dump_total > 0:
        print(f"\nDump Comparison:")
        print(f"  Match: {dump_match}/{dump_total} ({dump_match/dump_total*100:.1f}%)")

    # Show some failures
    failures = [r for r in results if not r["parse_ok"] and r["error"]]
    if failures:
        print(f"\n{'='*70}")
        print("SAMPLE FAILURES (first 10):")
        print("=" * 70)
        for f in failures[:10]:
            print(f"  {f['id']}: {f['name']}")
            print(f"    Tags: {f['tags']}")
            print(f"    Error: {f['error']}")
            print()

    # Compare with PyYAML
    print("=" * 70)
    print("COMPARISON WITH PyYAML")
    print("=" * 70)

    pyyaml_parse_ok = 0
    for test in test_cases:
        try:
            yaml.safe_load(test["yaml"])
            pyyaml_parse_ok += 1
        except:
            pass

    print(f"PyYAML Parse OK: {pyyaml_parse_ok}/{total} ({pyyaml_parse_ok/total*100:.1f}%)")
    print(f"pyamlium_custom Parse OK: {parse_ok}/{total} ({parse_ok/total*100:.1f}%)")


if __name__ == "__main__":
    main()
