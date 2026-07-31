import tempfile
from pathlib import Path

import pyrs_yaml
import pytest


def pytest_collection_modifyitems(session, config, items):
    """Auto-deselect benchmark tests when --codspeed is not provided."""
    if not config.getoption("--codspeed"):
        deselected = []
        selected = []
        for item in items:
            if "benchmark" in item.keywords:
                deselected.append(item)
            else:
                selected.append(item)
        if deselected:
            config.hook.pytest_deselected(items=deselected)
            items[:] = selected


@pytest.fixture(autouse=True)
def reset_language():
    pyrs_yaml.set_language("en")
    yield
    pyrs_yaml.set_language("en")


@pytest.fixture
def yaml_strings():
    return {
        "simple_mapping": "key: value",
        "nested_mapping": "parent:\n  child: grandchild",
        "sequence": "- a\n- b\n- c",
        "mixed": "name: test\nitems:\n  - 1\n  - 2\nflag: true",
        "multiline_scalar": "description: |\n  Line one\n  Line two",
        "quoted_scalar": 'message: "hello world"',
        "empty_document": "",
        "with_comment": "key: value  # a comment",
        "anchor": "defaults: &defaults\n  timeout: 30\nref: *defaults",
        "merge_key": "base: &base\n  a: 1\nb: &b\n  <<: *base\n  b: 2",
        "flow_mapping": "{key: value, num: 42}",
        "flow_sequence": "[a, b, c]",
        "null_value": "key: null",
        "empty_mapping": "{}",
        "empty_sequence": "[]",
    }


@pytest.fixture
def temp_yaml_file(yaml_strings):
    with tempfile.TemporaryDirectory() as tmpdir:
        filepath = str(Path(tmpdir) / "test.yaml")
        with Path(filepath).open("w", encoding="utf-8") as f:
            f.write(yaml_strings["simple_mapping"])
        yield filepath


@pytest.fixture
def language_context():
    original = pyrs_yaml.get_language()
    yield
    pyrs_yaml.set_language(original)
