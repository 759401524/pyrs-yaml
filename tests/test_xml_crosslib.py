"""The XML corpus is measured against Python's own XML reader.

`xml.etree.ElementTree` is the second implementation: it parses the same
committed fixtures with comments and processing instructions enabled, and the
infoset it hands back is normalised with the same six lexical rules this
engine documents (declaration, BOM, CDATA delimiters, entity spelling,
self-closing form, quote style and inter-attribute whitespace, plus the
conditional whitespace-only-text drop). What survives normalisation is the
claim that can actually break: element and attribute names in document order,
character data, comments and PIs.

DTD and entity fixtures are deliberately absent from the oracle. ElementTree
expands internal entities, so handing it a billion-laughs fixture would
materialise the bomb before either reader could refuse it. Those fixtures are
matched against exact error codes in `crates/pyrs-xml/tests/security_refusals.rs`
and in `tests/test_xml_api.py` only.

Run with pytest, or directly to print the corpus names.
"""

from __future__ import annotations

import xml.etree.ElementTree as ET
import xml.parsers.expat as expat
from pathlib import Path

import pytest

import pyrs_yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURE_DIR = REPO_ROOT / "crates" / "pyrs-xml" / "tests" / "fixtures"


def corpus() -> list[tuple[str, str]]:
    """The 15 golden fixtures; the security subdirectory is never enumerated."""
    items: list[tuple[str, str]] = []
    for path in sorted(FIXTURE_DIR.glob("*.xml")):
        if path.parent.name == "security":
            continue
        items.append((path.name, path.read_text(encoding="utf-8")))
    return items


def parse_tree(source: str):
    """Build an ElementTree without expat namespace expansion.

    `ET.XMLParser` always creates its expat parser with a namespace separator,
    which rewrites `p:outer` to `{urn:x}outer` and drops the `xmlns:p`
    attribute. This engine preserves QNames lexically (normalisation 5), so
    the oracle has to too: the same `ET.TreeBuilder` is fed by an expat parser
    with no namespace separator. Comments and PIs are still inserted.
    """
    target = ET.TreeBuilder(insert_comments=True, insert_pis=True)
    scanner = expat.ParserCreate()
    scanner.StartElementHandler = target.start
    scanner.EndElementHandler = target.end
    scanner.CharacterDataHandler = target.data
    scanner.CommentHandler = target.comment
    scanner.ProcessingInstructionHandler = target.pi
    scanner.Parse(source.encode("utf-8"), True)
    return target.close()


def text_is_space(value: str | None) -> bool:
    return bool(value) and all(ch in " \t\r\n" for ch in (value or ""))


def normalise_parts(element) -> list[tuple]:
    """ElementTree children plus text/tails, normalised into ordered parts."""
    raw: list[tuple] = []
    if element.text:
        raw.append(("text", element.text))
    for child in element:
        if child.tag is ET.Comment:
            raw.append(("comment", child.text or ""))
        elif child.tag is ET.ProcessingInstruction:
            raw.append(("pi", child.text or ""))
        else:
            raw.append(("element", child))
        if child.tail:
            raw.append(("text", child.tail))

    merged: list[tuple] = []
    for part in raw:
        if part[0] == "text" and merged and merged[-1][0] == "text":
            merged[-1] = ("text", merged[-1][1] + part[1])
        else:
            merged.append(part)

    has_element_child = any(part[0] == "element" for part in merged)
    if has_element_child:
        merged = [part for part in merged if not (part[0] == "text" and text_is_space(part[1]))]
    return merged


class Occurrence:
    """One shared, 0-based index space for preserved comments and PIs."""

    def __init__(self) -> None:
        self.value = 0

    def take(self) -> int:
        index = self.value
        self.value += 1
        return index


def normalise_element(element, counter: Occurrence):
    parts = normalise_parts(element)
    attrs = {f"+@{name}": value for name, value in element.attrib.items()}

    if not parts:
        return attrs or None

    if len(parts) == 1 and parts[0][0] == "text":
        value = parts[0][1]
        return {**attrs, "+content": value} if attrs else value

    if all(part[0] == "element" for part in parts):
        result = dict(attrs)
        for part in parts:
            child = part[1]
            key = child.tag
            value = normalise_element(child, counter)
            if key in result:
                if isinstance(result[key], list):
                    result[key].append(value)
                else:
                    result[key] = [result[key], value]
            else:
                result[key] = value
        return result

    sequence: list = []
    for part in parts:
        if part[0] == "text":
            sequence.append(part[1])
        elif part[0] == "element":
            sequence.append({part[1].tag: normalise_element(part[1], counter)})
        elif part[0] == "comment":
            sequence.append({f"+c_{counter.take()}": part[1]})
        else:
            target, _, data = part[1].partition(" ")
            index = counter.take()
            sequence.append({f"+p_{index}": {"target": target, "data": data}})
    return {**attrs, "+content": sequence}


def normalise(source: str):
    root = parse_tree(source)
    return {root.tag: normalise_element(root, Occurrence())}


@pytest.mark.parametrize("name,source", corpus(), ids=lambda value: value if isinstance(value, str) else "")
def test_engine_agrees_with_elementtree_on_every_golden_fixture(name: str, source: str) -> None:
    expected = normalise(source)
    assert pyrs_yaml.load_xml(source) == expected, name


@pytest.mark.parametrize("name,source", corpus(), ids=lambda value: value if isinstance(value, str) else "")
def test_from_xml_matches_the_committed_golden_text(name: str, source: str) -> None:
    golden = FIXTURE_DIR / name.replace(".xml", ".yaml")
    expected = golden.read_text(encoding="utf-8")
    assert pyrs_yaml.from_xml(source) == expected, name


def test_elementtree_reads_the_golden_example_the_same_way() -> None:
    source = (
        '<root id="7"><name>Ada</name><item>a</item><item>b</item>'
        "<!--note--><?pi go?><p>Hello <b>x</b>!</p><empty/></root>"
    )
    # The unnormalised ElementTree parse and the engine's YAML text are both
    # asked directly here, so the parity cell sees the two readers meet.
    assert ET.fromstring(source).tag == "root"
    tree = parse_tree(source)
    assert tree.tag == "root"
    assert list(tree.attrib.items()) == [("id", "7")]
    assert pyrs_yaml.from_xml(source).startswith("root:")
    assert pyrs_yaml.load_xml(source) == normalise(source)


if __name__ == "__main__":
    for name, _source in corpus():
        print(name)
