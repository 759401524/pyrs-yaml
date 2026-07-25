"""
Test error recovery improvements for YAML test suite
"""

import pyyaml_rs


def test_specific_failures():
    """Test specific failing cases from YAML test suite"""

    test_cases = [
        # simple_key_error cases
        ('236B', "foo:\n  bar\ninvalid"),
        ('4QFQ', "- |\n detected\n- >"),

        # mapping_context_error cases
        ('26DV', '"top1" :\n  "key1" : scalar1'),

        # block_scalar_error cases
        ('2G84', "--- |0"),

        # other cases
        ('5U3A', "key: - a\n     - b"),
        ('9C9N', "---\nflow: [a,\nb,"),
    ]

    results = {'pass': 0, 'fail': 0}

    for test_id, yaml_str in test_cases:
        print(f'=== {test_id} ===')
        print(f'YAML: {repr(yaml_str[:80])}')
        try:
            doc = pyyaml_rs.parse(yaml_str)
            print(f'Result: PASS')
            results['pass'] += 1
        except Exception as e:
            print(f'Error: {str(e)[:80]}')
            results['fail'] += 1
        print()

    print(f'Results: {results["pass"]} pass, {results["fail"]} fail')
    return results


def test_flow_constructs():
    """Test flow constructs which might be fixable"""

    test_cases = [
        # Flow mapping
        ('flow_map', '{a: 1, b: 2}'),
        ('flow_seq', '[1, 2, 3]'),
        ('nested_flow', '{a: {b: 1}}'),
    ]

    results = {'pass': 0, 'fail': 0}

    for test_id, yaml_str in test_cases:
        print(f'=== {test_id} ===')
        try:
            doc = pyyaml_rs.parse(yaml_str)
            print(f'PASS: {doc.to_yaml()[:50]}')
            results['pass'] += 1
        except Exception as e:
            print(f'FAIL: {str(e)[:50]}')
            results['fail'] += 1

    return results


def test_multiline_scalars():
    """Test multiline scalar parsing"""

    test_cases = [
        # Literal block
        ('literal', "key: |\n  line1\n  line2"),
        ('literal_indent', "key: |\n    indented\n    lines"),
        ('literal_chomp_strip', "key: |-\n  no trailing newline"),
        ('literal_chomp_keep', "key: |+\n  keep newlines\n"),

        # Folded block
        ('folded', "key: >\n  this is\n  folded"),
        ('folded_chomp', "key: >-\n  folded no newline"),
    ]

    results = {'pass': 0, 'fail': 0}

    for test_id, yaml_str in test_cases:
        print(f'=== {test_id} ===')
        try:
            doc = pyyaml_rs.parse(yaml_str)
            output = doc.to_yaml()
            print(f'PASS: {repr(output[:50])}')
            results['pass'] += 1
        except Exception as e:
            print(f'FAIL: {str(e)[:50]}')
            results['fail'] += 1

    return results


if __name__ == '__main__':
    print('=' * 70)
    print('ERROR RECOVERY TEST')
    print('=' * 70)

    r1 = test_specific_failures()
    r2 = test_flow_constructs()
    r3 = test_multiline_scalars()

    total_pass = r1['pass'] + r2['pass'] + r3['pass']
    total_fail = r1['fail'] + r2['fail'] + r3['fail']

    print('\n' + '=' * 70)
    print(f'TOTAL: {total_pass} pass, {total_fail} fail')
    print('=' * 70)
