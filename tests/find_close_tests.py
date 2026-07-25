"""
Find tests that are close to passing
"""

import yaml
import os
import json
import pyyaml_rs


def convert_special_chars(text: str) -> str:
    if not text:
        return text
    text = text.replace('\u2423', ' ')
    text = text.replace('\u2016\u2016\u2016\u00bb', '\t')
    text = text.replace('\u2016\u2016\u00bb', '\t')
    text = text.replace('\u2016\u00bb', '\t')
    text = text.replace('\u00bb', '\t')
    return text


def find_close_tests():
    suite_dir = 'Reference/yaml-test-suite/src'
    close_to_passing = []

    for f in sorted(os.listdir(suite_dir)):
        if not f.endswith('.yaml'):
            continue

        try:
            with open(os.path.join(suite_dir, f), 'r') as fh:
                content = yaml.safe_load(fh)

            if not content or not isinstance(content, list):
                continue

            for test in content:
                if not isinstance(test, dict):
                    continue

                tags = test.get('tags', '')
                if 'error' in tags.lower() or 'invalid' in tags.lower():
                    continue

                yaml_input = test.get('yaml', '')
                if not yaml_input:
                    continue

                yaml_input = convert_special_chars(yaml_input)

                try:
                    doc = pyyaml_rs.parse(yaml_input)
                    # Parsed OK, check if JSON matches
                    json_expected = test.get('json', '')
                    if json_expected:
                        actual_json = json.dumps(doc.to_dict(), sort_keys=True, indent=2)
                        expected_json = json.dumps(json.loads(json_expected), sort_keys=True, indent=2)
                        if actual_json != expected_json:
                            close_to_passing.append({
                                'id': f.replace('.yaml', ''),
                                'name': test.get('name', ''),
                                'tags': tags,
                            })
                except:
                    pass
        except:
            pass

    return close_to_passing


def main():
    close_to_passing = find_close_tests()

    print('=' * 70)
    print(f'Tests that parse but JSON does not match: {len(close_to_passing)}')
    print('=' * 70)
    print()

    for t in close_to_passing[:15]:
        print(f'  {t["id"]}: {t["name"][:50]}')


if __name__ == '__main__':
    main()
