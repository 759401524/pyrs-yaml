"""Shared I/O helpers for the pyrs-yaml CLI commands."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any, NoReturn


def fail(message: str) -> NoReturn:
    """Print an error to stderr and exit with status 1."""
    print(f"error: {message}", file=sys.stderr)
    sys.exit(1)


def _is_stdio(source: Any) -> bool:
    return source == "-" or getattr(source, "is_stdio", False)


def read_text(source: Any) -> str:
    """Read text from a file path, or from stdin when *source* is stdio."""
    if _is_stdio(source):
        return sys.stdin.read()
    try:
        return Path(source).read_text(encoding="utf-8")
    except OSError as exc:
        fail(f"cannot read '{source}': {exc.strerror or exc}")


def load_document(source: Any) -> Any:
    """Parse a YAML file (or stdin) into a round-trip ``YamlDocument``."""
    from pyrs_yaml import YamlParseError, parse, parse_file

    try:
        if _is_stdio(source):
            return parse(sys.stdin.read())
        return parse_file(str(source))
    except OSError as exc:
        fail(f"cannot read '{source}': {exc.strerror or exc}")
    except YamlParseError as exc:
        fail(str(exc))


def emit(text: str, output: Any = None) -> None:
    """Write *text* to stdout, or to a file when *output* is a path."""
    if output is None or _is_stdio(output):
        sys.stdout.write(text)
        return
    Path(output).write_text(text, encoding="utf-8")
