"""
Analyze remaining failures after special character conversion
"""

import yaml
import os
import pyamlium_custom


def convert_special_chars(text: str) -> str:
    if not text:
        return text
    text = text.replace('\u2423', ' ')
    text = text.replace('\u2016\u2016\u2016\u00bb', '\t')
    text = text.replace('\u2016\u2016\u00bb', '\t')
    text = text.replace('\u2016\u00bb', '\t')
    text = text.replace('\u00bb', '\t')
    return text


def analyze_remaining_failures():
    suite_dir = 'Reference/yaml-test-suite/src'
    failures = []

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
                    doc = pyamlium_custom.parse(yaml_input)
                except Exception as e:
                    failures.append({
                        'id': f.replace('.yaml', ''),
                        'name': test.get('name', ''),
                        'tags': tags,
                        'error': str(e)[:100],
                    })
        except:
            pass

    return failures


def main():
    failures = analyze_remaining_failures()

    print('=' * 70)
    print(f'REMAINING FAILING VALID TESTS: {len(failures)}')
    print('=' * 70)

    # Categorize
    categories = {}
    for f in failures:
        error = f['error']
        if 'simple key expect' in error:
            cat = 'simple_key'
        elif 'mapping values are not allowed' in error:
            cat = 'mapping_context'
        elif 'block scalar' in error:
            cat = 'block_scalar'
        elif 'quoted scalar' in error:
            cat = 'quoted_scalar'
        elif 'unexpected' in error:
            cat = 'unexpected'
        elif 'indentation' in error:
            cat = 'indentation'
        else:
            cat = 'other'

        if cat not in categories:
            categories[cat] = []
        categories[cat].append(f)

    print('\nBy category:')
    for cat, items in sorted(categories.items(), key=lambda x: -len(x[1])):
        print(f'\n  {cat}: {len(items)} failures')
        for item in items[:3]:
            print(f'    - {item["id"]}: {item["name"][:50]}')


if __name__ == '__main__':
    main()
