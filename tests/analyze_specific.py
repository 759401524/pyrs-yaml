"""
Analyze specific failing tests
"""

import yaml
import os
import pyamlium_custom


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
    tests_to_check = ['2G84', '5GBF', 'B3HG']

    for f in sorted(os.listdir(suite_dir)):
        if not f.endswith('.yaml'):
            continue

        test_id = f.replace('.yaml', '')
        if test_id not in tests_to_check:
            continue

        try:
            with open(os.path.join(suite_dir, f), 'r') as fh:
                content = yaml.safe_load(fh)

            if not content or not isinstance(content, list):
                continue

            for test in content:
                if not isinstance(test, dict):
                    continue

                yaml_input = test.get('yaml', '')
                if not yaml_input:
                    continue

                yaml_input = convert_special_chars(yaml_input)

                name = test.get('name', '')
                print(f'=== {test_id}: {name[:50]} ===')
                print(f'YAML:')
                for line in yaml_input.split('\n')[:5]:
                    print(f'  {repr(line)}')

                try:
                    doc = pyamlium_custom.parse(yaml_input)
                    print(f'OK')
                except Exception as e:
                    print(f'Error: {str(e)[:100]}')
                print()
        except:
            pass


if __name__ == '__main__':
    main()
