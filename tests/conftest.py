from __future__ import annotations

import tempfile
from pathlib import Path

import pytest
from _pytest.config import Config
from _pytest.nodes import Item
from _pytest.python import Session

import pyrs_yaml
from tests.data import yaml_samples as yaml


def pytest_collection_modifyitems(session: Session, config: Config, items: list[Item]) -> None:
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
        "simple_mapping": yaml.SIMPLE_MAPPING,
        "nested_mapping": yaml.NESTED_MAPPING,
        "sequence": yaml.SEQUENCE,
        "mixed": yaml.MIXED,
        "multiline_scalar": yaml.MULTILINE_SCALAR,
        "quoted_scalar": yaml.QUOTED_SCALAR,
        "empty_document": yaml.EMPTY_DOCUMENT,
        "with_comment": yaml.WITH_COMMENT,
        "anchor": yaml.ANCHOR,
        "merge_key": yaml.MERGE_KEY,
        "flow_mapping": yaml.FLOW_MAPPING,
        "flow_sequence": yaml.FLOW_SEQUENCE,
        "null_value": yaml.NULL_VALUE,
        "empty_mapping": yaml.EMPTY_MAPPING,
        "empty_sequence": yaml.EMPTY_SEQUENCE,
    }


@pytest.fixture
def temp_yaml_file(yaml_strings):
    with tempfile.TemporaryDirectory() as tmpdir:
        filepath = str(Path(tmpdir) / "test.yaml")
        with Path(filepath).open("w", encoding="utf-8") as f:
            f.write(yaml_strings["simple_mapping"])
        yield filepath
