"""Load an INI file into plain values (configparser-backed).

INI is an exchange-only spoke: no official grammar, so this wraps the
stdlib parser rather than inventing a dialect. Duplicate sections or
keys raise ValueError instead of silently merging.
"""

from __future__ import annotations

import configparser

from typing_extensions import override

__all__ = ["load_ini"]


class _CaseSensitiveParser(configparser.RawConfigParser):
    """Preserve option key case (the default lowercases them)."""

    @override
    def optionxform(self, optionstr: str) -> str:
        return optionstr


def load_ini(text: str) -> dict[str, dict[str, str]]:
    """Parse INI text into a ``{section: {key: value}}`` dict.

    Values are returned as raw strings (typed interpretation is left to
    the caller). Anonymous leading ``key = value`` lines land under the
    DEFAULT section and are inlined into every section, matching
    configparser semantics.
    """
    parser = _CaseSensitiveParser(strict=True, inline_comment_prefixes=None)
    try:
        parser.read_string(text)
    except configparser.Error as e:
        # configparser raises its own hierarchy; the library's load-error
        # contract is ValueError-based (YamlParseError et al).
        raise ValueError(f"INI parse error: {e}") from e
    result: dict[str, dict[str, str]] = {}
    for section in parser.sections():
        result[section] = dict(parser.items(section))
    default = dict(parser.defaults())
    if default:
        result["DEFAULT"] = default
    return result
