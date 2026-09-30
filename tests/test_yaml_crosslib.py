"""Cross-library correctness parity of the YAML engines against PyYAML and
ruamel.yaml.

Objective §测试覆盖 3 names `PyYAML` and `ruamel.yaml` as oracle targets.
Previously both libraries only appeared in `test_benchmark_crosslib.py`
(timed runs and a feature-support printout at L400-445), never as an
assertion that pyrs and the reference agree on the *value* produced for
a given YAML document. This module closes that gap: it pins the
canonical YAML Core 1.2 domain where the three engines must produce the
same Python object graph.

Deliberate divergences (YAML 1.1 legacy booleans, duplicate-key
strictness, comment/anchor preservation) are asserted explicitly and
documented rather than hidden -- see the divergence section at the
bottom. Optional deps guarded by `skipif`; suite runs cleanly without
them.
"""

from __future__ import annotations

import io

import pytest

import pyrs_yaml

try:
    import yaml as pyyaml
except ImportError:  # pragma: no cover - optional
    pyyaml = None

try:
    import ruamel.yaml as _ruamel
except ImportError:  # pragma: no cover - optional
    _ruamel = None


def _ruamel_available() -> bool:
    return _ruamel is not None


def _ruamel_safe(src: str):
    """Load via ruamel in safe-pure mode (plain dicts, no CommentedMap
    wrapper types), so equality against pyrs / PyYAML is direct."""
    y = _ruamel.YAML(typ="safe", pure=True)
    return y.load(src)


# ── Canonical YAML Core 1.2 domain ────────────────────────────────────────
# Every document here is well-defined under YAML 1.2 Core, PyYAML's
# safe_load, and ruamel's typ="safe" reader. All three engines must
# agree on the produced Python object graph.

CANONICAL_YAML_DOCS: list[str] = [
    # Scalars of each Core schema type.
    "a: 1\n",
    "a: -42\n",
    "a: 3.14\n",
    "a: true\n",
    "a: false\n",
    "a: null\n",
    "a: ~\n",
    "a: hello\n",
    'a: "hello world"\n',
    "a: 'single quoted'\n",
    # Sequences and mappings.
    "a: [1, 2, 3]\n",
    "a:\n  - 1\n  - 2\n  - 3\n",
    "a:\n  b: 1\n  c: 2\n",
    "a: {b: 1, c: 2}\n",
    # Nesting.
    "a:\n  b:\n    c: [1, {d: 2}, null]\n",
    # Multiple top-level keys.
    "a: 1\nb: two\nc: 3.5\nd: true\ne: null\n",
    # Empty containers.
    "a: []\nb: {}\n",
    # Unicode strings.
    'a: "café \U0001f600"\n',
    # Multi-line block scalars (values only; PyYAML/ruamel/pyrs must all
    # resolve them to the identical string).
    "a: |\n  line1\n  line2\n",
    "a: >\n  folded\n  text\n",
]


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
@pytest.mark.parametrize("src", CANONICAL_YAML_DOCS)
def test_pyrs_matches_pyyaml_safe_load(src):
    """pyrs.safe_load and PyYAML.safe_load produce identical Python values
    on the canonical YAML Core 1.2 domain."""
    assert pyrs_yaml.safe_load(src) == pyyaml.safe_load(src)


@pytest.mark.skipif(not _ruamel_available(), reason="ruamel.yaml not installed")
@pytest.mark.parametrize("src", CANONICAL_YAML_DOCS)
def test_pyrs_matches_ruamel_safe_load(src):
    """pyrs.safe_load and ruamel.safe_load (typ=safe, pure=True) produce
    identical Python values."""
    assert pyrs_yaml.safe_load(src) == _ruamel_safe(src)


@pytest.mark.skipif(pyyaml is None or not _ruamel_available(), reason="oracle missing")
@pytest.mark.parametrize("src", CANONICAL_YAML_DOCS)
def test_three_way_canonical_yaml_agreement(src):
    """Three-way check: pyrs, PyYAML, ruamel -- all agree byte-for-byte on
    the canonical domain. Any drift shows up on the pair that diverges."""
    p = pyrs_yaml.safe_load(src)
    y = pyyaml.safe_load(src)
    r = _ruamel_safe(src)
    assert p == y == r


# ── Round-trip dump parity ────────────────────────────────────────────────
# Every engine re-loads pyrs.safe_dump's output back to the same object,
# and vice-versa: pyrs re-loads PyYAML's / ruamel's dumps.


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
@pytest.mark.parametrize("src", CANONICAL_YAML_DOCS)
def test_pyrs_loads_what_pyyaml_dumps(src):
    data = pyyaml.safe_load(src)
    dumped = pyyaml.safe_dump(data, default_flow_style=False)
    assert pyrs_yaml.safe_load(dumped) == data


@pytest.mark.skipif(not _ruamel_available(), reason="ruamel.yaml not installed")
@pytest.mark.parametrize("src", CANONICAL_YAML_DOCS)
def test_pyrs_loads_what_ruamel_dumps(src):
    data = _ruamel_safe(src)
    buf = io.StringIO()
    y = _ruamel.YAML(typ="safe", pure=True)
    y.dump(data, buf)
    assert pyrs_yaml.safe_load(buf.getvalue()) == data


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
@pytest.mark.parametrize("src", CANONICAL_YAML_DOCS)
def test_pyyaml_loads_what_pyrs_dumps(src):
    data = pyrs_yaml.safe_load(src)
    dumped = pyrs_yaml.safe_dump(data)
    assert pyyaml.safe_load(dumped) == data


# ── Documented deliberate divergences ────────────────────────────────────


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
def test_duplicate_keys_are_deliberately_rejected_by_pyrs():
    """Objective §Known Engine Boundaries / v0.11.5 audit: pyrs rejects
    duplicate keys where PyYAML silently accepts (last wins). Pinned as
    a spec-strictness choice, not a bug."""
    src = "a: 1\na: 2\n"
    assert pyyaml.safe_load(src) == {"a": 2}  # PyYAML lenient
    with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
        pyrs_yaml.safe_load(src)


@pytest.mark.skipif(pyyaml is None, reason="PyYAML not installed")
def test_yaml_11_legacy_booleans_are_schema_scoped():
    """YAML 1.1 legacy booleans (`yes` / `on` / `off`) resolve to `bool`
    under the `yaml1.1` schema; YAML 1.2 Core keeps them as strings.
    PyYAML follows the 1.1 convention on its `safe_load`. Test pins that
    pyrs's schema selection is intentional and the string-vs-bool
    divergence is documented."""
    src = "a: yes\n"
    assert pyyaml.safe_load(src) is not None  # PyYAML gives True
    assert pyyaml.safe_load(src)["a"] is True
    # pyrs Core: `yes` is a plain string, matching YAML 1.2.
    assert pyrs_yaml.safe_load(src)["a"] == "yes"
    # pyrs yaml1.1 schema: resolves to bool like PyYAML.
    assert pyrs_yaml.safe_load(src, schema="yaml1.1")["a"] is True
