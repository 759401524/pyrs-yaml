"""
List all failing valid tests
"""

import os

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
    text = text.replace('\u220e', '')
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
                        'error': str(e)[:100],
                    })
        except:
            pass

    print(f'Failing valid tests: {len(failing)}')
    print()

    for item in failing:
        print(f'{item["id"]}: {item["name"][:50]}')
        print(f'  Tags: {item["tags"]}')
        print(f'  Error: {item["error"]}')
        print()


if __name__ == '__main__':
    main()
