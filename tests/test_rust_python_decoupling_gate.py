"""The Rust/Python decoupling gate has to be able to go red, and the reason must be legible.

`scripts/check_rust_python_decoupling.py` is the rule that keeps the crate matrix free of the
Python runtime: the extension crate is the only one that produces a cdylib, every other crate's
dependency closure carries no Python-runtime crate, the extension holds no Rust test tier, no
other crate may lose its tests, and a feature-gated target names a declared feature that is off
by default.

Two things about that script are worth a test of their own, and neither is the rule itself.

The first is that it can fail. A gate that has never been seen to bite is a comment, so every
rule is broken on purpose on a synthetic workspace - a Python runtime crate in an engine crate, a
`#[cfg(test)]` block back in the extension, a crate that disables its own test tier, a feature
gate that names a default feature - and the gate has to go red with that rule's id in the
output, not with some other one. A rule that fires for the wrong reason is a gate that measures
nothing.

The second is that its manifest reader is right. The script carries its own reader because
`tomllib` is 3.11+ and this package supports 3.8, so the reader is cross-checked against the
standard one on every manifest in the tree - the fields the gate reads, compared value by value.
The reader refuses a construct it does not understand rather than guessing at it, and a refusal
has to surface as exit 2, never as a pass.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
GATE_PATH = REPO_ROOT / "scripts" / "check_rust_python_decoupling.py"

try:  # Python 3.11+
    import tomllib
except ImportError:  # pragma: no cover - the 3.8 leg
    import tomli as tomllib


def _load_gate():
    spec = importlib.util.spec_from_file_location("check_rust_python_decoupling", GATE_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def gate():
    return _load_gate()


# ── the synthetic workspace the controls run against ─────────────────────────────────────

ROOT_MANIFEST = """\
[workspace]
members = ["crates/engine", "crates/binding"]
resolver = "2"
"""

ENGINE_MANIFEST = """\
[package]
name = "engine"
version = "0.1.0"
edition = "2024"

[lib]
name = "engine"

[dependencies]
indexmap = "2.7"
"""

ENGINE_SOURCE = """\
pub fn parse() -> usize {
    0
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses() {
        assert_eq!(super::parse(), 0);
    }
}
"""

BINDING_MANIFEST = """\
[package]
name = "binding"
version = "0.1.0"
edition = "2024"

[lib]
name = "binding"
crate-type = ["cdylib", "lib"]
test = false

[dependencies]
pyo3 = "0.29"
engine = { path = "../engine" }

[features]
default = []
ir-gate = ["pyo3/auto-initialize"]

[[bench]]
name = "ir_gate"
harness = false
required-features = ["ir-gate"]
"""

BINDING_SOURCE = """\
pub fn bind() -> usize {
    0
}
"""


def _write(root: Path, relative: str, body: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(body)


def _workspace(root: Path, **overrides) -> Path:
    """A two-crate workspace: one engine without Python, one extension with it."""
    files = {
        "Cargo.toml": ROOT_MANIFEST,
        "crates/engine/Cargo.toml": ENGINE_MANIFEST,
        "crates/engine/src/lib.rs": ENGINE_SOURCE,
        "crates/binding/Cargo.toml": BINDING_MANIFEST,
        "crates/binding/src/lib.rs": BINDING_SOURCE,
    }
    files.update(overrides)
    for relative, body in files.items():
        if body is None:
            continue
        _write(root, relative, body)
    return root


def _run(gate, capsys, root: Path):
    """The gate's verdict and everything it said while reaching it."""
    code = gate.main([str(root)])
    captured = capsys.readouterr()
    return code, captured.out + captured.err


# ── the positive control ────────────────────────────────────────────────────────────────


def test_the_synthetic_workspace_is_clean(gate, capsys, tmp_path):
    """The controls below are only meaningful if the same tree passes untouched."""
    code, output = _run(gate, capsys, _workspace(tmp_path))
    assert code == 0, output
    assert "2 crate(s)" in output, output


def test_the_measurement_is_derived_not_declared(gate, capsys, tmp_path):
    """A crate the gate was not told about is still counted, so the matrix is read, not listed."""
    root = _workspace(
        tmp_path,
        **{
            "Cargo.toml": '[workspace]\nmembers = ["crates/engine", "crates/binding", "crates/third"]\n',
            "crates/third/Cargo.toml": '[package]\nname = "third"\nversion = "0.1.0"\n\n[lib]\nname = "third"\n',
            "crates/third/src/lib.rs": "#[cfg(test)]\nmod tests {\n    #[test]\n    fn works() {}\n}\n",
        },
    )
    code, output = _run(gate, capsys, root)
    assert code == 0, output
    assert "3 crate(s)" in output, output


# ── the falsification control: every rule, broken on purpose ────────────────────────────


def test_a_python_runtime_in_an_engine_crate_is_a_finding(gate, capsys, tmp_path):
    """R1: the closure of a crate that produces no cdylib must carry no Python runtime."""
    manifest = ENGINE_MANIFEST.replace(
        '[dependencies]\nindexmap = "2.7"\n', '[dependencies]\nindexmap = "2.7"\npyo3 = "0.29"\n'
    )
    code, output = _run(gate, capsys, _workspace(tmp_path, **{"crates/engine/Cargo.toml": manifest}))
    assert code == 1, output
    assert "[R1] engine" in output, output


def test_a_python_runtime_reached_through_a_path_edge_is_a_finding(gate, capsys, tmp_path):
    """R1 reads the closure, not the direct dependencies: the edge can be two crates deep."""
    manifest = ENGINE_MANIFEST.replace(
        '[dependencies]\nindexmap = "2.7"\n', '[dependencies]\nindexmap = "2.7"\nbridge = { path = "../bridge" }\n'
    )
    root = _workspace(
        tmp_path,
        **{
            "crates/engine/Cargo.toml": manifest,
            "crates/bridge/Cargo.toml": '[package]\nname = "bridge"\nversion = "0.1.0"\n\n[dependencies]\npyo3 = "0.29"\n',
            "crates/bridge/src/lib.rs": "pub fn b() {}\n",
            "Cargo.toml": '[workspace]\nmembers = ["crates/engine", "crates/binding", "crates/bridge"]\n',
        },
    )
    code, output = _run(gate, capsys, root)
    assert code == 1, output
    assert "[R1] engine" in output, output


def test_a_test_block_in_the_extension_crate_is_a_finding(gate, capsys, tmp_path):
    """R2: a `#[cfg(test)]` block in the extension puts the interpreter back in the Rust legs."""
    source = BINDING_SOURCE + "\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn binds() {}\n}\n"
    code, output = _run(gate, capsys, _workspace(tmp_path, **{"crates/binding/src/lib.rs": source}))
    assert code == 1, output
    assert "[R2] binding" in output, output


def test_a_tests_directory_in_the_extension_crate_is_a_finding(gate, capsys, tmp_path):
    """R2 covers the integration-test directory as well as the unit blocks."""
    root = _workspace(tmp_path, **{"crates/binding/tests/integration.rs": "#[test]\nfn works() {}\n"})
    code, output = _run(gate, capsys, root)
    assert code == 1, output
    assert "[R2] binding" in output, output


def test_a_test_target_in_the_extension_crate_is_a_finding(gate, capsys, tmp_path):
    """R2 covers a declared `[[test]]` target, which is a third way to hold tests."""
    manifest = BINDING_MANIFEST + '\n[[test]]\nname = "integration"\n'
    code, output = _run(gate, capsys, _workspace(tmp_path, **{"crates/binding/Cargo.toml": manifest}))
    assert code == 1, output
    assert "[R2] binding" in output, output


def test_the_extension_must_disable_its_lib_test_target(gate, capsys, tmp_path):
    """R2: the declaration is the half that keeps cargo from building an empty test binary."""
    manifest = BINDING_MANIFEST.replace(
        'crate-type = ["cdylib", "lib"]\ntest = false\n', 'crate-type = ["cdylib", "lib"]\n'
    )
    code, output = _run(gate, capsys, _workspace(tmp_path, **{"crates/binding/Cargo.toml": manifest}))
    assert code == 1, output
    assert "[R2] binding" in output, output
    assert "test = false" in output, output


def test_disabling_a_crates_own_test_tier_is_a_finding(gate, capsys, tmp_path):
    """R3: decoupling is the absence of the coupling, not the absence of the tests."""
    manifest = ENGINE_MANIFEST.replace('[lib]\nname = "engine"\n', '[lib]\nname = "engine"\ntest = false\n')
    code, output = _run(gate, capsys, _workspace(tmp_path, **{"crates/engine/Cargo.toml": manifest}))
    assert code == 1, output
    assert "[R3] engine" in output, output


def test_deleting_a_crates_tests_is_a_finding(gate, capsys, tmp_path):
    """R3 catches the other half of the same move: emptying the tier instead of disabling it."""
    code, output = _run(
        gate, capsys, _workspace(tmp_path, **{"crates/engine/src/lib.rs": "pub fn parse() -> usize {\n    0\n}\n"})
    )
    assert code == 1, output
    assert "[R3] engine" in output, output


def test_a_feature_gate_on_a_default_feature_is_a_finding(gate, capsys, tmp_path):
    """R4: a gate that names a default feature does not keep the target out of the default build."""
    manifest = BINDING_MANIFEST.replace("default = []", 'default = ["ir-gate"]')
    code, output = _run(gate, capsys, _workspace(tmp_path, **{"crates/binding/Cargo.toml": manifest}))
    assert code == 1, output
    assert "[R4] binding" in output, output


def test_a_feature_gate_on_an_undeclared_feature_is_a_finding(gate, capsys, tmp_path):
    """R4: a gate that names a feature nobody declared disables the target instead of scoping it."""
    manifest = BINDING_MANIFEST.replace('required-features = ["ir-gate"]', 'required-features = ["nope"]')
    code, output = _run(gate, capsys, _workspace(tmp_path, **{"crates/binding/Cargo.toml": manifest}))
    assert code == 1, output
    assert "[R4] binding" in output, output


def test_a_second_cdylib_crate_is_a_finding(gate, capsys, tmp_path):
    """R1: the extension is one crate, and a second cdylib makes the rule ambiguous."""
    root = _workspace(
        tmp_path,
        **{
            "Cargo.toml": '[workspace]\nmembers = ["crates/engine", "crates/binding", "crates/other"]\n',
            "crates/other/Cargo.toml": '[package]\nname = "other"\nversion = "0.1.0"\n\n[lib]\nname = "other"\ncrate-type = ["cdylib"]\n',
            "crates/other/src/lib.rs": "#[cfg(test)]\nmod tests {\n    #[test]\n    fn works() {}\n}\n",
        },
    )
    code, output = _run(gate, capsys, root)
    assert code == 1, output
    assert "[R1] workspace" in output, output


def test_an_extension_without_a_python_runtime_is_a_finding(gate, capsys, tmp_path):
    """R1 reads both ways: a crate that stopped being a binding must stop being the extension."""
    manifest = BINDING_MANIFEST.replace('pyo3 = "0.29"\n', "")
    code, output = _run(gate, capsys, _workspace(tmp_path, **{"crates/binding/Cargo.toml": manifest}))
    assert code == 1, output
    assert "[R1] binding" in output, output


# ── the reader: right on the tree, and loud when it cannot read ──────────────────────────


def _view(manifest: dict) -> dict:
    """The facts the gate reads, out of a parsed manifest."""
    lib = manifest.get("lib", {})
    crate_types = lib.get("crate-type", [])
    if isinstance(crate_types, str):
        crate_types = [crate_types]
    return {
        "package": manifest.get("package", {}).get("name"),
        "lib_crate_types": list(crate_types),
        "lib_test": lib.get("test"),
        "dependencies": sorted(manifest.get("dependencies", {})),
        "dev_dependencies": sorted(manifest.get("dev-dependencies", {})),
        "build_dependencies": sorted(manifest.get("build-dependencies", {})),
        "features": {name: list(value) for name, value in manifest.get("features", {}).items()},
        "benches": [
            (target.get("name"), list(target.get("required-features", [])), target.get("harness"))
            for target in manifest.get("bench", [])
        ],
        "tests": [target.get("name") for target in manifest.get("test", [])],
    }


def _reader_view(gate, text: str) -> dict:
    """The same facts, out of the gate's own reader."""
    manifest = gate.parse_manifest(text)
    lib = manifest.get("lib", {})
    crate_types = lib.get("crate-type", [])
    if isinstance(crate_types, str):
        crate_types = [crate_types]
    return {
        "package": manifest.get("package", {}).get("name"),
        "lib_crate_types": list(crate_types),
        "lib_test": lib.get("test"),
        "dependencies": sorted(manifest.get("dependencies", {})),
        "dev_dependencies": sorted(manifest.get("dev-dependencies", {})),
        "build_dependencies": sorted(manifest.get("build-dependencies", {})),
        "features": {name: list(value) for name, value in manifest.get("features", {}).items()},
        "benches": [
            (target.get("name"), list(target.get("required-features", [])), target.get("harness"))
            for target in manifest.get("bench", [])
        ],
        "tests": [target.get("name") for target in manifest.get("test", [])],
    }


def test_the_reader_agrees_with_the_standard_one_on_every_manifest(gate):
    """The gate carries its own reader because `tomllib` is 3.11+; this is what makes that safe.

    Every manifest in the tree is parsed by both, and the facts the gate reads are compared
    value by value. A manifest shape the reader mishandles fails here rather than in the gate.
    """
    manifests = sorted(REPO_ROOT.glob("Cargo.toml")) + sorted(REPO_ROOT.glob("crates/*/Cargo.toml"))
    manifests += sorted(REPO_ROOT.glob("fuzz/Cargo.toml"))
    assert len(manifests) >= 9, "the tree's manifests were not found; the check below would be vacuous"
    for path in manifests:
        text = path.read_text(encoding="utf-8")
        assert _reader_view(gate, text) == _view(tomllib.loads(text)), path


def test_the_reader_reads_the_constructs_the_manifests_use(gate):
    """The reader's own shapes, read out of the gate's parser rather than out of a file."""
    manifest = gate.parse_manifest(
        "\n".join(
            [
                "# a comment",
                "[package]",
                'name = "demo"',
                "",
                "[lib]",
                'name = "demo"',
                'crate-type = ["cdylib", "lib"]',
                "test = false",
                "",
                "[dependencies]",
                'other = { path = "../other", version = "0.1.0" }',
                'plain = "1.2"',
                "",
                "[features]",
                'default = ["std"]',
                "std = [",
                '    "dep:thing",',
                "]",
                "",
                "[[bench]]",
                'name = "gate"',
                "harness = false",
                'required-features = ["ir-gate"]',
            ]
        )
    )
    assert manifest["package"]["name"] == "demo"
    assert manifest["lib"]["crate-type"] == ["cdylib", "lib"]
    assert manifest["lib"]["test"] is False
    assert manifest["dependencies"]["other"]["path"] == "../other"
    assert manifest["features"]["std"] == ["dep:thing"]
    assert manifest["bench"][0]["required-features"] == ["ir-gate"]


def test_the_reader_refuses_what_it_cannot_read(gate, capsys, tmp_path):
    """A refusal is exit 2, and never a pass: the gate could not measure, so it says so."""
    manifest = ENGINE_MANIFEST + "\n[package.metadata]\nreleased = 2024-01-01\n"
    root = _workspace(tmp_path, **{"crates/engine/Cargo.toml": manifest})
    code, output = _run(gate, capsys, root)
    assert code == 2, output
    assert "cannot measure" in output, output


def test_a_manifest_that_cannot_be_read_is_not_a_pass(gate, capsys, tmp_path):
    """A missing manifest is the same class of failure: unreadable is not clean."""
    root = _workspace(tmp_path, **{"crates/engine/Cargo.toml": None})
    code, output = _run(gate, capsys, root)
    assert code == 2, output


# ── the real tree ────────────────────────────────────────────────────────────────────────


def test_the_repository_is_decoupled(gate, capsys):
    """The gate's own subject: this repository, as it stands."""
    code, output = _run(gate, capsys, REPO_ROOT)
    assert code == 0, output
    assert "free of the Python runtime" in output, output


def test_the_extension_is_the_only_cdylib_crate(gate):
    """The rule the gate's first line rests on, read out of the tree rather than asserted here."""
    crates = [gate.read_crate(path) for path in sorted((REPO_ROOT / "crates").iterdir()) if path.is_dir()]
    extensions = [crate for crate in crates if crate.is_extension]
    assert [crate.declared_name for crate in extensions] == ["pyrs-yaml"]
    assert gate.is_python_runtime("pyo3")
    assert gate.is_python_runtime("pyo3-build-config")
    assert gate.is_python_runtime("numpy")
    assert not gate.is_python_runtime("pyrs-yaml-core")
    assert not gate.is_python_runtime("granit-parser")
