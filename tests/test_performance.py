"""
Performance sanity tests — basic parse/serialize speed checks.
"""

import time

import pyyaml_rs


class TestPerformance:
    """Basic performance sanity checks"""

    def test_parse_speed(self):
        yaml_str = "key: value\nlist:\n  - a\n  - b\n  - c"
        start = time.perf_counter()
        for _ in range(1000):
            pyyaml_rs.parse(yaml_str)
        elapsed = time.perf_counter() - start
        assert elapsed < 1.0  # Should parse 1000 times in under 1 second

    def test_serialize_speed(self):
        doc = pyyaml_rs.parse("key: value\nlist:\n  - a\n  - b")
        start = time.perf_counter()
        for _ in range(1000):
            doc.to_yaml()
        elapsed = time.perf_counter() - start
        assert elapsed < 1.0
