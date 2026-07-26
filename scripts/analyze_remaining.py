"""
Analyze remaining failing valid tests
"""

import os
from collections import defaultdict

import yaml

import pyyaml_rs


def convert_special_chars(text):
    if not text:
        return text
    text = text.replace('\u2423', ' ')
    text = text.replace('\u2014\u2014\u2014\u00bb', '\t')
    text = text.replace('\u2014\u2014\u00bb', '\t')
    text = text.replace('\u2014\u00bb', '\t')
    text = text.replace('\u00bb', '\t')
    return text


def main():
    suite_dir = 'Reference/yaml-test-suite/src'
    failing = []

    for f in sorted(os.listdir(suite_dir)):
        if not f.endswith('.yaml'):
            continue

        try:
            with open(os.path.join(suite_dir, f)) as fh:
                content = yaml.safe_load(fh)

            if not content or not isinstance(content, list):
                continue

            for test in content:
                if not isinstance(test, dict):
                    continue

                tags = test.get('tags', '')
                is_fail = test.get('fail', False)
                is_valid = 'error' not in tags.lower() and 'invalid' not in tags.lower() and not is_fail

                if not is_valid:
                    continue

                yaml_input = test.get('yaml', '')
                if not yaml_input:
                    continue

                yaml_input = convert_special_chars(yaml_input)

                try:
                    doc = pyyaml_rs.parse(yaml_input)
                except Exception as e:
                    failing.append({
                        'id': f.replace('.yaml', ''),
                        'name': test.get('name', ''),
                        'tags': tags,
                        'error': str(e)[:120],
                        'yaml': yaml_input,
                    })
        except:
            pass

    print(f'Total failing: {len(failing)}')
    print()

    # Group by test ID
    by_id = defaultdict(list)
    for f in failing:
        by_id[f['id']].append(f)

    for test_id, tests in sorted(by_id.items()):
        name = tests[0]['name']
        tags = tests[0]['tags']
        error = tests[0]['error']
        print(f'=== {test_id}: {name[:50]} ===')
        print(f'  Tags: {tags}')
        print(f'  Subtests: {len(tests)}')
        print(f'  Error: {error}')
        print()


if __name__ == '__main__':
    main()
