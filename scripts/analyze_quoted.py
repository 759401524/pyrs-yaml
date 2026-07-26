"""
Analyze quoted scalar failures
"""

import os

import yaml

import pyyaml_rs


def convert_special_chars(text: str) -> str:
    if not text:
        return text
    text = text.replace("\u2423", " ")
    text = text.replace("\u2016\u2016\u2016\u00bb", "\t")
    text = text.replace("\u2016\u2016\u00bb", "\t")
    text = text.replace("\u2016\u00bb", "\t")
    text = text.replace("\u00bb", "\t")
    return text


def analyze_quoted_failures():
    suite_dir = "Reference/yaml-test-suite/src"
    failures = []

    for f in sorted(os.listdir(suite_dir)):
        if not f.endswith(".yaml"):
            continue

        try:
            with open(os.path.join(suite_dir, f)) as fh:
                content = yaml.safe_load(fh)

            if not content or not isinstance(content, list):
                continue

            for test in content:
                if not isinstance(test, dict):
                    continue

                tags = test.get("tags", "")
                if "error" in tags.lower() or "invalid" in tags.lower():
                    continue

                yaml_input = test.get("yaml", "")
                if not yaml_input:
                    continue

                yaml_input = convert_special_chars(yaml_input)

                try:
                    doc = pyyaml_rs.parse(yaml_input)
                except Exception as e:
                    error = str(e)
                    if "quoted scalar" in error or "escape" in error:
                        failures.append(
                            {
                                "id": f.replace(".yaml", ""),
                                "name": test.get("name", ""),
                                "tags": tags,
                                "error": error[:150],
                                "yaml": yaml_input[:300],
                            }
                        )
        except:
            pass

    return failures


def main():
    failures = analyze_quoted_failures()

    print("=" * 70)
    print(f"QUOTED SCALAR FAILURES: {len(failures)}")
    print("=" * 70)

    for f in failures:
        print(f"\nID: {f['id']}")
        print(f"Name: {f['name'][:60]}")
        print(f"Tags: {f['tags']}")
        print("YAML:")
        for line in f["yaml"].split("\n")[:3]:
            print(f"  {line!r}")
        print(f"Error: {f['error']}")


if __name__ == "__main__":
    main()
