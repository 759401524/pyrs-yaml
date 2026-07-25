"""
Analyze YAML test suite failures to find quick fixes
"""

import pyyaml_rs
import yaml
import os


def analyze_failures():
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

                yaml_input = test.get('yaml', '')
                if not yaml_input:
                    continue

                try:
                    doc = pyyaml_rs.parse(yaml_input)
                    # Test passed
                except Exception as e:
                    error_msg = str(e)
                    tags = test.get('tags', '')
                    name = test.get('name', '')
                    failures.append({
                        'id': f.replace('.yaml', ''),
                        'name': name,
                        'tags': tags,
                        'error': error_msg[:200],
                        'yaml': yaml_input[:300],
                    })
        except:
            pass

    return failures


def categorize_failures(failures):
    categories = {}

    for f in failures:
        error = f['error']

        if 'mapping values are not allowed in this context' in error:
            cat = 'mapping_context_error'
        elif 'simple key expect' in error:
            cat = 'simple_key_error'
        elif 'while scanning a block scalar' in error:
            cat = 'block_scalar_error'
        elif 'while parsing a quoted scalar' in error:
            cat = 'quoted_scalar_error'
        elif 'document end marker' in error:
            cat = 'document_end_error'
        elif 'indentation' in error:
            cat = 'indentation_error'
        elif 'unexpected' in error:
            cat = 'unexpected_token'
        elif 'Scan error' in error:
            cat = 'other_scan_error'
        else:
            cat = 'unknown'

        if cat not in categories:
            categories[cat] = []
        categories[cat].append(f)

    return categories


def main():
    print('=' * 70)
    print('YAML TEST SUITE FAILURE ANALYSIS')
    print('=' * 70)

    failures = analyze_failures()
    categories = categorize_failures(failures)

    print(f'\nTotal failures: {len(failures)}')
    print()

    # Sort by count
    for cat, items in sorted(categories.items(), key=lambda x: -len(x[1])):
        print(f'{"="*60}')
        print(f'{cat}: {len(items)} failures')
        print(f'{"="*60}')

        # Show first 5 examples
        for item in items[:5]:
            print(f'\n  ID: {item["id"]}')
            print(f'  Name: {item["name"][:60]}')
            print(f'  Tags: {item["tags"]}')
            print(f'  Error: {item["error"][:100]}')
            print(f'  YAML preview:')
            for line in item['yaml'].split('\n')[:3]:
                print(f'    {line}')
            print()

        if len(items) > 5:
            print(f'  ... and {len(items) - 5} more')
        print()

    # Identify quick fixes
    print('=' * 70)
    print('POTENTIAL QUICK FIXES')
    print('=' * 70)

    # Check for simple patterns
    quick_fixes = []

    for cat, items in categories.items():
        if cat == 'mapping_context_error':
            # These are often due to missing flow mapping support
            quick_fixes.append({
                'category': cat,
                'count': len(items),
                'fix': 'Add flow mapping/sequence support in parser',
                'difficulty': 'Medium',
            })
        elif cat == 'simple_key_error':
            # Simple key parsing issues
            quick_fixes.append({
                'category': cat,
                'count': len(items),
                'fix': 'Improve simple key detection in scanner',
                'difficulty': 'Hard',
            })
        elif cat == 'block_scalar_error':
            # Block scalar parsing issues
            quick_fixes.append({
                'category': cat,
                'count': len(items),
                'fix': 'Fix block scalar indentation handling',
                'difficulty': 'Medium',
            })

    for fix in quick_fixes:
        print(f'\n{fix["category"]}:')
        print(f'  Count: {fix["count"]}')
        print(f'  Fix: {fix["fix"]}')
        print(f'  Difficulty: {fix["difficulty"]}')


if __name__ == '__main__':
    main()
