"""
Analyze failing valid tests from YAML test suite
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
    return text


def main():
    suite_dir = 'Reference/yaml-test-suite/src'
    failing_valid = []

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
                if 'error' in tags.lower() or 'invalid' in tags.lower():
                    continue

                yaml_input = test.get('yaml', '')
                if not yaml_input:
                    continue

                yaml_input = convert_special_chars(yaml_input)

                try:
                    doc = pyyaml_rs.parse(yaml_input)
                except Exception as e:
                    failing_valid.append({
                        'id': f.replace('.yaml', ''),
                        'name': test.get('name', ''),
                        'tags': tags,
                        'error': str(e)[:120],
                        'yaml': yaml_input[:200],
                    })
        except:
            pass

    print(f'Failing valid tests: {len(failing_valid)}')
    print()

    # Show first 15
    for item in failing_valid[:15]:
        print(f'{item["id"]}: {item["name"][:50]}')
        print(f'  Tags: {item["tags"]}')
        print(f'  Error: {item["error"]}')
        print()


if __name__ == '__main__':
    main()
