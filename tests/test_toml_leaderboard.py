"""TOML fast-path floors — the native paths must beat the in-process paths they replace.

Runs in normal CI (no ``--codspeed``). These gate the *structural* wins of the
native TOML kernel against a baseline computed **in the same process on the same
runner**, so the ratio is stable under shared-CI contention.

**Cross-library ranking does not belong here.** The "pyrs beats tomllib"
comparison is already owned by CodSpeed (``tests/test_benchmark_crosslib.py``:
``test_pyrs_load_toml`` vs ``test_tomllib_load``, groups ``pyrs-toml`` vs
``tomllib``, isolated WallTime on a fixed runner). It was additionally asserted
here with a 2x floor, which is not a property of the code:

- The floor is **platform-dependent, not a code property**. Measured ratio vs
  ``tomllib`` on a 100-item document: ~4.5x on Windows x86-64, ~1.8x on the
  macOS runner. A pure-Python stdlib parser simply fares relatively better on
  Apple Silicon, so *any* floor that holds on one OS can break on another while
  the Rust kernel is unchanged - the CI matrix runs this file on 3 OS x 7 Python,
  so one slow cell blocks every unrelated PR.
- The previous 2x floor failed exactly that way: ``toml parse/medium: pyrs not
  >2x faster than tomllib (381.1us vs 688.4us)`` on macos-latest while passing
  on the same commit's other 20 cells. This file's own ``small`` fixture had
  already been dropped for the same reason (#142).

What is a real invariant, and is gated below, is *less work per unit of input*:

- ``load_toml`` (single-pass TOML -> Python objects) vs the AST route it
  bypasses (``parse(from_toml(doc)).to_dict()``: TOML -> YAML text -> CustomNode
  tree -> Python objects). Measured ~2.2x / ~1.9x on medium / large.
- ``to_toml`` (single-pass AST -> TOML text) vs the serialize-to-YAML-then-
  reparse round-trip the method eliminates (#142).

Both native paths do strictly less work than their baselines on every platform, so the floors hold
and still fail loudly if a fast path regresses back through the slow route. Sizes are medium/large: a
tiny document is dominated by fixed call overhead.

**Contention does not move the two sides together, and that is measured, not assumed.** The claim this
file used to make - that timing both baselines end to end in one process keeps the ratio honest - held
only while the process owned its cores. The recorded macos-latest incident has `load_toml` at 336us
against 369us for the AST route, 1.10x, where the same pair measures ~2.2x locally: on a two-core
runner the whole process is preempted, so a spike lands on whichever phase happens to be running. Two
sequential blocks per side therefore cannot cancel it, which is why both gates below sample through
`tests/timing.py`: candidate and reference are measured adjacently inside each block, the verdict is
a majority of the pairs, and the failure message carries all five pairs so a red can be attributed to
one inflated side or to a genuinely thin margin.
"""

import pytest

import pyrs_yaml
from tests.timing import compare, majority


def _doc(items):
    head = (
        '[server]\nhost = "0.0.0.0"\nport = 8080\nssl = true\n'
        '[database]\ntype = "postgresql"\n\n'
        "[database.pool]\nmin_size = 5\nmax_size = 20\n\n"
    )
    body = "".join(f'[[items]]\nname = "item_{index}"\nvalue = {index * 10}\n\n' for index in range(items))
    return head + body


_SIZES = {"medium": _doc(100), "large": _doc(500)}


@pytest.mark.parametrize("size", sorted(_SIZES))
def test_toml_parse_beats_ast_route(size):
    """load_toml must beat the to-YAML-then-reparse AST route it bypasses.

    Parity guard: both routes must produce the same dict, so we are timing real work rather than a
    path that bails early. The AST route is timed end-to-end, `from_toml` included: it converts TOML
    to YAML text, builds a CustomNode tree, then converts that tree to Python objects. Hoisting
    `from_toml` into setup would compare against a different pipeline (YAML -> dict only) and can
    measure the native path as *slower*.
    """
    doc = _SIZES[size]
    assert pyrs_yaml.load_toml(doc) == pyrs_yaml.parse(pyrs_yaml.from_toml(doc)).to_dict()
    result = compare(
        lambda: pyrs_yaml.load_toml(doc),
        lambda: pyrs_yaml.parse(pyrs_yaml.from_toml(doc)).to_dict(),
    )
    assert majority(result), f"parse/{size}: " + result.verdict("native load_toml vs the AST route it bypasses")


@pytest.mark.parametrize("size", sorted(_SIZES))
def test_toml_serialize_beats_yaml_round_trip(size):
    """Native doc.to_toml() must beat the to_yaml()+to_toml round-trip it replaces.

    The earlier version gated a cross-library margin vs tomli_w at 2x; calibrated on a local ~4.3x it
    flaked to 1.97x on a loaded macOS runner (the #142 lesson: do not put a cross-implementation timing
    floor in a blocking assert). This in-process self-relative floor compares doc.to_toml() (single
    native AST pass, #142) against pyrs_yaml.to_toml(doc.to_yaml()) - the serialize-to-YAML-then-
    reparse round-trip the method eliminates. Measured locally: 4.5x medium, 4.8x large. It used to
    take one block per side, which is the weakest possible estimator; it now shares the paired sampler
    so a single scheduler spike cannot decide the verdict.
    """
    doc_text = _SIZES[size]
    parsed = pyrs_yaml.parse(pyrs_yaml.from_toml(doc_text))  # setup: get a document
    # Parity: the writer's output reloads to the same data.
    assert pyrs_yaml.load_toml(parsed.to_toml()) == pyrs_yaml.load_toml(doc_text)
    result = compare(lambda: parsed.to_toml(), lambda: pyrs_yaml.to_toml(parsed.to_yaml()))
    assert majority(result), f"toml serialize/{size}: " + result.verdict(
        "native to_toml vs the to_yaml()+to_toml round trip"
    )
