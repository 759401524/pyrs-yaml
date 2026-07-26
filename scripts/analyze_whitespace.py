"""
Analyze whitespace-related YAML test failures
"""

import os

import yaml

import pyyaml_rs


def analyze_whitespace_failures():
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

                if "whitespace" not in tags:
                    continue

                yaml_input = test.get("yaml", "")
                if not yaml_input:
                    continue

                try:
                    doc = pyyaml_rs.parse(yaml_input)
                except Exception as e:
                    failures.append(
                        {
                            "id": f.replace(".yaml", ""),
                            "name": test.get("name", ""),
                            "tags": tags,
                            "yaml": yaml_input,
                            "error": str(e)[:150],
                        }
                    )
        except:
            pass

    return failures


def main():
    failures = analyze_whitespace_failures()

    print("=" * 70)
    print("WHITESPACE-RELATED FAILURES")
    print("=" * 70)
    print(f"Total: {len(failures)}")
    print()

    for f in failures[:10]:
        print(f"ID: {f['id']}")
        print(f"Name: {f['name'][:60]}")
        print(f"Tags: {f['tags']}")
        print("YAML:")
        for line in f["yaml"].split("\n")[:5]:
            print(f"  {line!r}")
        print(f"Error: {f['error']}")
        print()


if __name__ == "__main__":
    main()
