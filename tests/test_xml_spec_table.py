"""XML 1.0 well-formedness clauses, asked of two implementations.

One row per clause this engine exercises. ElementTree is the second
implementation: where XML 1.0 gives a reader no repair to make, the two must
refuse the same document. The DTD/entity rows are engine-only by design —
ElementTree expands internal entities, so it is never given those inputs (the
engine's structural refusal is measured in the Rust security fixtures and in
`tests/test_xml_api.py`).
"""

from __future__ import annotations

import xml.etree.ElementTree as ET

import pytest

import pyrs_yaml

# (clause, document). Every document here is refused by ElementTree too, so
# the second implementation is a real check rather than a rubber stamp.
WELL_FORMEDNESS = [
    ("matching end tag", "<a></b>"),
    ("single root element", "<a/><b/>"),
    ("quoted attribute values", "<a x=1/>"),
    ("no duplicate attributes", "<a x='1' x='2'/>"),
    ("legal Name start character", "<1a/>"),
    ("no double hyphen in comments", "<!-- a -- b --><a/>"),
    ("no xml PI target", "<?XML?><a/>"),
    ("no undeclared entity", "<a>&foo;</a>"),
]


@pytest.mark.parametrize("clause,source", WELL_FORMEDNESS)
def test_engine_and_elementtree_refuse_the_same_well_formedness_defect(clause: str, source: str) -> None:
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.from_xml(source)
    with pytest.raises(ET.ParseError):
        ET.fromstring(source)


@pytest.mark.parametrize("clause,source", WELL_FORMEDNESS)
def test_load_xml_refuses_the_same_defects(clause: str, source: str) -> None:
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_xml(source)


@pytest.mark.parametrize(
    "source",
    [
        "<!DOCTYPE root [<!ENTITY x 'boom'>]><root>&x;</root>",
        "<!ENTITY x 'boom'>",
    ],
)
def test_dtd_and_entity_declarations_are_engine_only_refusals(source: str) -> None:
    """No oracle call here: ElementTree expands internal entities."""
    with pytest.raises(pyrs_yaml.YamlParseError) as caught:
        pyrs_yaml.from_xml(source)
    assert "E-XML-00" in str(caught.value)


def test_declaration_accepts_only_1_0_and_utf8() -> None:
    assert pyrs_yaml.from_xml("<?xml version='1.0' encoding='UTF-8'?><a/>") == "a: null\n"
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.from_xml("<?xml version='1.1'?><a/>")
    assert ET.fromstring("<?xml version='1.0'?><a/>").tag == "a"
