import pyrs_yaml
import pytest


@pytest.mark.parametrize(
    "yaml,expected",
    [
        ('text: "hello\\nworld"', "hello\nworld"),
        ('text: "\\u0041\\u0042\\u0043"', "ABC"),
        ('text: "tab\\there\\nnewline"', "tab\there\nnewline"),
        ('text: "back\\\\slash"', "back\\slash"),
        ('text: "say \\"hello\\""', 'say "hello"'),
        ('text: "null\\0char"', "null\x00char"),
        ('text: "tab\\tchar"', "tab\tchar"),
        ('text: "newline\\nchar"', "newline\nchar"),
    ],
    ids=["newline", "unicode", "special", "backslash", "quote", "null", "tab", "newline-char"],
)
def test_escape(yaml, expected):
    assert pyrs_yaml.parse(yaml).get("text") == expected
