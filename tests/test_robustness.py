"""Subprocess-isolated crash probes.

A native stack overflow -- the class of defect behind issue #166, where a
recursive / self-referential merge key expanded until the call stack blew --
does NOT raise a Python exception. It tears down the whole interpreter with an
access violation (Windows exit ``0xC00000FD``) or ``SIGSEGV`` / ``SIGABRT`` on
POSIX. No in-process assertion can observe that: the process is gone before the
check runs, and (worse) it would take the entire pytest session down with it.

So hostile documents are dispatched to a *child* process that runs the parse →
expand → serialize pipeline and always exits 0 on any Python-level outcome
(success OR a caught exception). A nonzero child exit status is therefore
unambiguous evidence of a native crash, not a rejected input.

This is the permanent regression harness that keeps the #166 class from
reappearing: every self-referential / degenerate merge that once segfaulted now
has to return control cleanly.
"""

import subprocess
import sys

import pytest

# Runs one document end-to-end through the paths that recurse: parse (which
# resolves merge keys when ``resolve_merges`` is on), full alias expansion via
# ``to_dict``, and re-serialization. The document arrives on **stdin** (not
# argv) so a large hostile input never collides with the OS command-line length
# limit (Windows raises WinError 206 past ~32 KB). Any Python-level result --
# including a raised parse/compose error -- is swallowed and reported on stdout,
# so the child reaches ``exit 0``. Only a hard crash prevents that.
_RUNNER = r"""
import sys
import pyrs_yaml

text = sys.stdin.read()
resolve = sys.argv[1] == "merge"
try:
    doc = pyrs_yaml.parse(text, resolve_merges=resolve, max_depth=1000)
    doc.to_dict()
    doc.to_yaml()
except BaseException as exc:  # noqa: BLE001 - a controlled error must exit 0
    sys.stdout.write("!!" + type(exc).__name__)
"""


def _hostile_cases():
    """Inputs that used to blow the native stack (or stress the expansion)."""
    # (1)-(3): the three #166 repros that segfaulted with exit 0xC00000FD.
    cases = [
        ("self_merge", "a: &a\n  b:\n    <<: *a\n"),
        ("self_merge_sibling", "a: &a\n  b: 1\n  c:\n    <<: *a\n"),
        ("nested_self_merge", "a: &a\n  x: 1\n  sub:\n    <<: *a\nb:\n  <<: *a\n"),
        # A merge sequence that references the anchor twice.
        ("self_merge_seq", "a: &a\n  b:\n    <<: [*a, *a]\n"),
        # Deep legitimate nesting: the parser's own depth budget should reject it
        # as a controlled error (exit 0), never a crash.
        ("deep_nesting", "".join("  " * d + "n:\n" for d in range(200))),
    ]
    # Wide fan-out: many independent mappings each merging the *same* anchor. The
    # path-scoped guard must not over-suppress these -- they all expand fully --
    # and the total work stays bounded (no recursion into ancestors).
    fanout = ["base: &b\n  k: 1\n"] + [f"s{i}:\n  <<: *b\n  i: {i}\n" for i in range(50)]
    cases.append(("wide_fanout", "".join(fanout)))
    # Long legitimate merge chain, each level merging the previous anchor.
    chain = ["l0: &l0\n  z: 0\n"] + [f"l{i}: &l{i}\n  <<: *l{i - 1}\n" for i in range(1, 60)]
    cases.append(("long_chain", "".join(chain)))
    return cases


@pytest.mark.parametrize("name,text", _hostile_cases(), ids=[c[0] for c in _hostile_cases()])
@pytest.mark.parametrize("resolve", ["merge", "raw"], ids=["resolve", "raw"])
def test_no_native_crash(name, text, resolve):
    r = subprocess.run(
        [sys.executable, "-c", _RUNNER, resolve],
        input=text,
        capture_output=True,
        text=True,
        timeout=60,
        check=False,
    )
    # POSIX signal deaths surface as a negative return code; Windows NTSTATUS
    # crash codes (e.g. 0xC00000FD stack overflow) surface as a large unsigned
    # value. Either way a healthy child exits exactly 0.
    unsigned = r.returncode & 0xFFFFFFFF
    assert r.returncode == 0, (
        f"[{name}/{resolve}] child exited {r.returncode} (0x{unsigned:08X}) "
        f"-- suspected native crash. stdout={r.stdout[:200]!r} "
        f"stderr={r.stderr[:400]!r}"
    )
