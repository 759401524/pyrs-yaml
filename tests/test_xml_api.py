"""The Python XML API: two readers, no writer, localized refusals.

`from_xml` returns YAML text through the hub's serializer, `load_xml` returns
the Python value graph. Both carry the engine's stable code in the localized
`xml-parse-error` detail. There is no `to_xml`: the write direction belongs to
a later slice and must not appear on the wheel surface.
"""

from __future__ import annotations

import pytest

import pyrs_yaml


def test_from_xml_emits_the_plus_convention() -> None:
    yaml = pyrs_yaml.from_xml(
        '<root id="7"><name>Ada</name><item>a</item><item>b</item><!--note--><?pi go?><empty/></root>'
    )
    assert '+@id: "7"' in yaml
    assert "+c_0: note" in yaml
    assert "+p_1:" in yaml
    assert "- name: Ada" in yaml
    assert "- empty: null" in yaml


def test_from_xml_defaults_and_explicit_flags_are_equivalent() -> None:
    source = "<root><!--note--><?pi go?><a>1</a></root>"
    assert pyrs_yaml.from_xml(source) == pyrs_yaml.from_xml(source, comments=True, pi=True)


def test_from_xml_flags_drop_only_their_own_parts() -> None:
    source = "<root><!--note--><?pi go?><a>1</a></root>"
    no_comments = pyrs_yaml.from_xml(source, comments=False)
    assert "+c_" not in no_comments
    assert "+p_0:" in no_comments
    no_pi = pyrs_yaml.from_xml(source, pi=False)
    assert "+p_" not in no_pi
    assert "+c_0:" in no_pi
    neither = pyrs_yaml.from_xml(source, comments=False, pi=False)
    assert "+c_" not in neither and "+p_" not in neither
    assert 'a: "1"' in neither


def test_load_xml_returns_python_values_without_retyping() -> None:
    value = pyrs_yaml.load_xml("<root><n>007</n><flag>true</flag><empty/><item>a</item><item>b</item></root>")
    assert value == {
        "root": {
            "n": "007",
            "flag": "true",
            "empty": None,
            "item": ["a", "b"],
        }
    }


def test_load_xml_respects_content_flags() -> None:
    source = "<root><!--note--><?pi go?><a>1</a></root>"
    content = pyrs_yaml.load_xml(source)["root"]["+content"]
    assert any("+c_0" in part for part in content if isinstance(part, dict))
    assert any("+p_1" in part for part in content if isinstance(part, dict))
    lean = pyrs_yaml.load_xml(source, comments=False, pi=False)
    assert lean == {"root": {"a": "1"}}


@pytest.mark.parametrize(
    "source,code",
    [
        ("<!DOCTYPE root [<!ENTITY x 'y'>]><root>&x;</root>", "E-XML-001"),
        ("<!ENTITY x 'y'>", "E-XML-002"),
        ("<root>&x;</root>", "E-XML-003"),
        ("<root", "E-XML-004"),
        ("<root></other>", "E-XML-005"),
        ("<a/><b/>", "E-XML-006"),
        ("<!--x--><a/>", "E-XML-007"),
        ("<?xml version='1.1'?><a/>", "E-XML-008"),
        ("<a x='1' x='2'/>", "E-XML-009"),
        ("<a>&#xD800;</a>", "E-XML-010"),
        ("<p:a/>", "E-XML-013"),
    ],
)
def test_every_refusal_family_raises_a_localized_parse_error(source: str, code: str) -> None:
    with pytest.raises(pyrs_yaml.YamlParseError) as caught:
        pyrs_yaml.from_xml(source)
    assert code in str(caught.value), str(caught.value)
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_xml(source)


def test_ten_level_billion_laughs_is_refused_without_expansion() -> None:
    lines = ["<!DOCTYPE lolz [", ' <!ENTITY lol "lol">']
    for level in range(1, 11):
        reference = "&lol;" if level == 1 else f"&lol{level - 1};"
        lines.append(f' <!ENTITY lol{level} "' + reference * 10 + '">')
    lines += ["]>", "<lolz>&lol10;</lolz>"]
    source = "\n".join(lines)
    with pytest.raises(pyrs_yaml.YamlParseError) as caught:
        pyrs_yaml.from_xml(source)
    assert "E-XML-001" in str(caught.value)


def test_there_is_no_writer_on_the_wheel_surface() -> None:
    assert not hasattr(pyrs_yaml, "to_xml")
    assert hasattr(pyrs_yaml, "from_xml")
    assert hasattr(pyrs_yaml, "load_xml")
