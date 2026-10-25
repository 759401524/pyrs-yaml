# AGENTS.md — pyrs-yaml

## Critical

- Use `uv` for all Python operations — never `pip` or system Python
- Use `prek` (not pre-commit) for Git hooks — run `prek install --prepare-hooks`
- Use `cargo nextest` (not `cargo test`) for Rust tests — install via `cargo install cargo-nextest`
- PyO3 v0.29 requires `#[pyo3(signature = "...")]` with double-quoted type strings
- All PyO3 custom types must implement `INPUT_TYPE`/`OUTPUT_TYPE` traits
- No `serde_yaml`, `yaml-rust2`, or any serde-based YAML library
- Must release GIL during expensive pure-Rust computation (parse, edit, AST serialize): `py.detach(|| { ... })` (pyo3 0.29). `Python::attach` re-acquires the GIL. No `with_gil`/`allow_threads` in 0.29.
- **Do NOT** cargo-cult `py.detach` when the dominant cost is Python object walking (e.g., `pyobject_to_node`). The old `dump_iterable` pattern `py.detach(|| to_yaml(&node))` released the GIL for a minority phase while the Python-object walk held the GIL throughout. When the whole operation is inherently Python-bound, the total GIL-hold time is all that matters — a faster inline path without detach is strictly better for concurrency (proven: direct_dump 6.6x faster overall, other threads get more CPU share).
- **State the assumption before the code, not after the failure.** When two readings of a request are both defensible, name both and pick out loud; when a simpler approach exists, say so even if it isn't what was asked for. "Ask First" above is the concrete list — an unclear instruction is a question, not a guess.
- **Write the minimum code that solves the named problem.** No feature, abstraction, configurability or error handling for a scenario that cannot occur. If 200 lines could be 50, rewrite it — the test is "would a reviewer call this overcomplicated?"
- **Touch only what the change must touch.** Do not improve adjacent code, comments or formatting; do not refactor what is not broken (match the surrounding style); delete only what *this* change made unused, and mention — never silently remove — unrelated dead code. Every changed line must trace to the request.
- **A claim needs a command output behind it.** Turn a task into a verifiable goal before writing code ("reject invalid inputs" → "a test for each invalid form, then make them pass"), and don't call work done on a plausible answer. The repo's existing gates are the checklist: `prek run --all-files`, `cargo clippy --all --all-targets -- -D warnings`, `uv run pytest tests/ -v`, the doc gates, and the Ir/CI performance floor — read the output, don't summarise the intent.
- **Tool selection runs in priority order: built-in tools → project custom tools → MCP → mature off-the-shelf CLI → Python/Node scripts → custom shell.** Downgrade only when a higher tier genuinely cannot do it (or its cost grows past the benefit), and say why in one line. Prefer Python/Node over shell for anything with real control flow; when a downgraded script ends up solving a repeated problem, propose it as a registered tool rather than letting it proliferate as one-off wrappers.
- **Shell stays least-privilege and sandboxed.** Read-only commands by default (`ls`, `cat`, `grep`, `awk`); no destructive or privilege-escalating forms (`rm -rf`, `curl | sh`, `sudo`, force-push) without an explicit request, and never a hook bypass (`git commit --no-verify`) — see **Never** below.

## Commands

- Build + install: `uv run maturin develop --release`
- Run tests: `uv run pytest tests/ -v`
- Rust fmt: `cargo fmt`
- Rust lint: `cargo clippy --all --all-targets -- -D warnings` (CI runs the same scope)
- Rust tests: `cargo nextest run --all` (preferred over `cargo test`)
- Pure Rust tests (no Python runtime): `cargo test --all --no-default-features`
- Python lint: `uv run ruff check . && uv run ruff format .`
- Generate stubs: `uv run maturin generate-stubs --out target/stubs && python scripts/check_stub_drift.py --fix`
- Verify stubs: `uv run maturin generate-stubs --out target/stubs && python scripts/check_stub_drift.py` (also the `stub-drift` PR gate in `.github/workflows/validate.yml`)
- Verify the stub *type checks* (different claim than "it parses"): `uv run --with 'mypy==2.4.0' python scripts/check_stub_types.py --checker mypy` and the same with `--with 'ty==0.0.85'` `--checker ty`. Neither checker is a project dependency — they arrive as pinned overlays, the way `prek` does. Exit codes: 0 clean, 1 the stub is at fault, 2 the check could not run (including a checker that is not installed — never a pass).
- Run prek hooks: `prek run --all-files`
- Defence-matrix counts: `python scripts/quality_matrix.py --check-rows` (CI's gate) and `--write-rows` (the same measurement rewrites `QUALITY_MATRIX.md`'s tier-row counts, so growing a tier costs a flag rather than a hand-typed digit)
- Landed-claim gate: `python scripts/check_landed_claims.py` — a heading in a live design document (`docs/dev/*.md`, dated records exempt) that says something landed or shipped carries `<!-- landed: <path>[::<symbol>] -->` on the line below it, naming code the tree contains. Exit 0 means every claim is locked and every lock resolves, 1 means a claim is bare or a lock is stale, 2 means nothing was measurable (never a pass)
- Ir instruction-count gate: `python scripts/ir_gate.py` (needs valgrind; on Windows hosts run it in WSL). Pin the compiler the baseline was built with or the reading is not the gate's question: `RUSTUP_TOOLCHAIN=$(python -c "import json;print(json.load(open('.ci/ir-baseline.json'))['toolchain'].split()[1])")` — the same tree on a newer stable is a different build and moves the YAML writers by up to +9% (`docs/dev/perf.md`). Add scenarios in `crates/pyrs-yaml-core/benches/ir_gate.rs` and measure only the new ones with `ir_gate.py --update --only <name>` — it merges into `.ci/ir-baseline.json` and refuses to write an incomplete baseline. To move every number to the enforcing image, dispatch the "Ir baseline refresh" workflow and commit its artifact. When a scenario is meant to move up past the tolerance, name it in that workflow's `accept_drift` input — one flag per scenario; an empty input keeps the bare `--update`, so refusing stays the default.
- Where a scenario's instructions go: `python scripts/ir_gate.py --profile <scenario> [--top N]` (needs valgrind; WSL on this host). It ranks that scenario's functions by self instructions — full minus `--setup-only`, so the harness is not counted — and prints functions exactly as named, never grouped by a pattern over the name, because name-regex buckets manufactured a 12.46% "hash" row for TOML once (`docs/dev/perf.md`). It builds with symbols (`CARGO_PROFILE_RELEASE_STRIP=false`); a stripped binary profiles as a table of addresses. Measure before optimizing: `tests/test_ir_profile.py` guards the parser.

## Pinned Versions

- pyo3 0.29, abi3-py38 + abi3t-py315 for stable ABI
- granit-parser 2.0 (YAML 1.2 parser with native comment support) — the manifest pins `"2.0"`; the jump
  from 1.3 was a breaking major and landed as a measured migration, not a `cargo update`
- numpy (rust-numpy) 0.29 — supports free-threaded Python since v0.24.0; enabled on cp314t wheels
- maturin >=1.0,<2.0 — the lock holds 1.15.0 on purpose: `python/pyrs_yaml/pyrs_yaml.pyi` is generated by
  that version and `scripts/check_stub_drift.py`'s reconciliation rules describe it, so moving maturin is a
  deliberate change (regenerate the stub and update those rules in the same commit), never a side effect of
  an unrelated `uv lock`. `tests/test_docs_config_gate.py` fails if the lock moves it silently. The 1.14.1 →
  1.15.0 move measured no stub change at all (608 lines byte-identical, and the reconciliation counts needed
  no edit); the one thing that moved is the `--out` layout, because 1.15.0 writes
  `target/stubs/pyrs_yaml/pyrs_yaml.pyi` instead of `target/stubs/pyrs_yaml.pyi`, which is the `GENERATED`
  path in `check_stub_drift.py`.
- Docs toolchain: `zensical>=0.0.69`, `mkdocstrings-python>=2.0.9`, both marked `python_version >= '3.11'`
  (each requires it). Every key under `[project.plugins.mkdocstrings.handlers.python.options]` is checked
  against the installed handler's own option fields — an unrecognised key is ignored by the build in
  silence (measured: a bogus option changes nothing and `--strict` still passes), so the option names, not
  the exit code, are what a version bump can break.
- ruff >=0.14.0 (py39+), >=0.5.0,<0.10.0 (py38)
- prek 0.1.0+

## Non-Obvious

- `docs/<locale>/api/*.md` are generated: each page carries `:::` directives and mkdocstrings renders the
  extension's signatures and docstrings into it. That gap closed with #321, so the obligation is now the
  inverse of what this bullet used to describe: a `:::` block added to an `en` page needs its zh/ja/ko twins,
  and "the build was green" is not evidence the content arrived — `scripts/build-docs.py` output is measured
  by counting rendered signature/class markers, not by the exit code (a page missing its directives renders
  perfectly while showing nothing).
- This file is gated. `scripts/check_hole_claims.py` asserts that every quality-hole id named here or in
  `CLAUDE.md`, and every one named inside the `[Unreleased]` block of the five changelog mirrors, is an id the
  matrix measures right now, because for a milestone this file described a closed gap as open and nothing read
  it: the matrix compared the registry with `QUALITY_MATRIX.md` in both directions and stopped there. The
  release notes joined that scope after one closed gap was still described as open in five mirrors, a ledger
  row and the Rust doc comment of the seam that closed it, across two pull requests with every gate green.
  Keep the rule honest rather than working around it — state the defence as it is now, name the pull request
  that closed a gap instead of the id of it, and leave histories to the dated records
  (`docs/dev/quality-ledger.md`, a released `CHANGELOG.md` section), where naming a closed hole is the point.
- Switching jj changes does not rebuild the extension. `python/pyrs_yaml/pyrs_yaml.pyd` belongs to whichever
  sources were checked out when it was last built, so a Python run after `jj edit`/`jj rebase` can silently
  test another branch's binary — which has now produced three misleading results in one session (a JSON5 test
  failing against a pre-#312 build, a schema test failing against a pre-#327 build, a passing run that had
  proved nothing). Rebuild after moving the working copy. That obligation is a gate now:
  `tests/test_extension_freshness.py` compares the loaded extension against every file compiled into it — the
  crates the binding reaches transitively through its `path = "..."` dependencies, their `src` trees and
  manifests — and fails by naming them, so a stale binary costs a red run rather than a wrong conclusion.
  The crate set is derived, not listed: the standalone `pyq` crate is linked by nothing in the binding, so
  counting it asked every CLI edit for a rebuild that cannot change the loaded module. Modules a crate root
  declares under `#[cfg(test)]` are excluded — derived from those declarations, never a hand-written list —
  because editing a Rust property test and re-running Python needs no rebuild, and an extension installed
  outside the working copy is skipped rather than compared. When
  `maturin develop` cannot install its dependency
  groups (a PyPI outage), `cargo build --release -p pyrs-yaml` and copying `target/release/pyrs_yaml.dll` over
  the `.pyd` produces the same binary without touching the network; note it is a *build* substitute, not a
  wheel install, so the packaging steps still need maturin.
- Local `cargo fuzz` on this tree needs two things the error messages do not say: `RUSTUP_TOOLCHAIN=nightly`
  (a bare `cargo fuzz build` under stable reports a rustc-looking failure because `-Zsanitizer` is unstable),
  and on WSL a `CARGO_TARGET_DIR` on the Linux filesystem — building with the target directory under `/mnt/d`
  dies mid-compile with "Cannot allocate memory" while writing `.rmeta`, which is a 9p limit and looks exactly
  like a broken target. New targets go in `fuzz/Cargo.toml` **and** the `fuzz.yml` matrix: the quality matrix
  derives from both, and `tests/test_quality_matrix.py` reddens on either half missing.
- An upgrade applied with a bare `uv run --group docs` re-resolves **the whole project** and rewrites
  `pyproject.toml` and `uv.lock` — that is how a documentation change arrived at the stub-drift job with
  `maturin` moved from 1.14.1 to 1.15.0. Use `uv lock --upgrade-package <name>` for a scoped bump.
- `zensical` versions disagree about page metadata: the locked 0.0.56 publishes a page whose front matter is
  not a mapping (its description silently becomes the site's), while 0.0.69 fails the build outright. A
  green `Docs` run is therefore not evidence about a page — see `scripts/check_doc_metadata.py`.
- `numpy` is optional (feature `numpy`); compiled into all wheels including cp314t — runtime probe `py.import("numpy").is_ok()` auto-detects availability
- `Cargo.toml` uses `abi3t` base feature; py315 added separately
- Do not add pyo3's `extension-module`, and do not split a cdylib shell crate to avoid it: pyo3 0.29
  deprecates the feature because it drops the libpython link for tests and benches as well, maturin
  (pinned 1.15.0) sets `PYO3_BUILD_EXTENSION_MODULE` for the packaging build alone, and measured on
  this tree enabling it leaves the built cdylib's import table byte-for-byte the same — `abi3`+`abi3t`
  already resolve Python through `python3.dll`, the stable-ABI forwarder
- `python/pyrs_yaml/py.typed` must always exist (PEP 561 marker for mypy)
- `.pyi` stubs are auto-generated by maturin; never edit manually. The declared route is checked three ways, not one: `ast.parse` (it is Python) and `scripts/check_stub_types.py` under both `mypy` and `ty` (it is *typing*). The generator emits Rust spellings such as `u32` and `Py<PyAny>` into annotations and forgets `from typing import Callable`; those are reconciled in `check_stub_drift.py`, counted, and a name that maps to no `typing` export fails the run rather than gaining a guessed import. `ty` is a second opinion, never the only one: `ty check <directory>` prints `All checks passed!` with `WARN No python files found` for a package holding only a `.pyi`, so the script names the file and treats a run that examined nothing as failure (exit 2, distinct from 1 = the stub is at fault).
- `scripts/check_stub_drift.py` is the only sanctioned place to reconcile the committed stub with generator output: it strips the trailing whitespace that prek's hook removes at commit, escapes backslashes that the pinned maturin copies into a docstring unescaped (which made the shipped `.pyi` unparsable — `EXPECTED_DOCSTRING_ESCAPES` is the tripwire and the derived text must pass `ast.parse`), appends the ten extension exception types the generator omits (`EXPECTED_EXCEPTION_CLASSES`, read from the live classes — an unimportable extension exits 2 rather than shipping a thinner contract), declares the twelve members `python/pyrs_yaml/__init__.py` attaches to the extension's classes after import (`EXPECTED_ATTACHED_MEMBERS`, read from the compiled module because maturin introspects it before the package runs), and declares the two `__next__` returns where that version drops the binding's `Option`. Add a rule there (with its expected match count) — never a hand edit of the stub.
- The Ir gate's tolerance is not all signal — the distance between the committed floor and the current tree is the number that matters. The 2026-10-14 refresh measured −45.95%…+0.10% over the 21 scenarios: six writer scenarios sat 22–46% below their floor and four readers 4.7–11.2% below, so the gate had been waving through a regression the size of the whole TOML writer win. That gap is drift accumulated while four optimization PRs landed between refreshes, and it widens the accepted band every time a PR makes the engine faster — refresh on the enforcing image rather than re-tuning the line. The same refresh, measured on all 22 scenarios on 2026-10-16 (one tree, one WSL host, only the toolchain
changing — `docs/dev/perf.md`), says the term a local reading moves is the compiler, not the host: on
the baseline's pinned `rustc 1.97.1` nothing is past the line (worst `to_python_medium` +0.1639%, the
nineteen non-binding scenarios inside ±0.003%, so the pinned-ISA recipe did close the +1.45% figure),
while the box's default `rustc 1.99.0` puts five past it — `serialize_medium` +9.08%,
`serialize_block_scalars` −7.71% in the other direction, and different binary digests. CI is immune
(`ci.yml`'s `ir` job installs the baseline's `toolchain` before measuring); a local run is not, so pin it —
`RUSTUP_TOOLCHAIN=$(python -c "import json;print(json.load(open('.ci/ir-baseline.json'))['toolchain'].split()[1])")`
— and note that `ir_gate.py --only` takes exactly one complete scenario name (it filters by
`s in args.only`, so a comma list silently matches nothing). The lower edge is a verdict too now: a
scenario more than 2% below its floor warns, more than 5% fails as `FLOOR_STALE`, and the remedy is the
dispatch job above rather than a local `--update`, which would move the floor to this host.
- `#[pymodule_init]` is prohibited — use inline `mod` for module declaration
- Branch protection requires exactly one check name, `Test matrix (all legs)`, and `enforce_admins` is on. `ci.yml` therefore triggers on **every** pull request: a `changes` job classifies it (`git diff` excluding `*.md` and `docs/**`), the heavy legs run only when that says code changed, and `docs-gates` always runs the documentation gates plus `scripts/build-docs.py --strict`. A required check that never reports is a deadlock, not a pass — measured on PR #319, a prose-only pull request that could not be merged by any contributor route. Never re-add `paths-ignore` for prose to `ci.yml`; if a leg must be conditional, gate it on `needs.changes.outputs.code` **and** declare `changes` in that job's `needs`, or GitHub resolves the reference to an empty string and skips it silently (`tests/test_matrix_verdict_gate.py` fails on that).
- The `no_std` foundation is six crates — `pyrs-ast`, `pyrs-event`, `pyrs-schema`, `pyrs-json`,
  `pyrs-toml` and `pyrs-yaml-core` — and the set is derived rather than written down here: a crate
  joins by declaring `#![no_std]` in its own root, and `tests/test_no_std_gate.py` compares that
  derived set against the three `no-std-check` steps in `ci.yml` (the `thumbv7em-none-eabi` build,
  the host unit tests under `--no-default-features`, the std re-check) *and* against the crates the
  live documents name — this file and `docs/dev/boundaries.md` — so a crate that joins the group
  without joining the gate, and a document that keeps naming the old group, are both red runs
  rather than stale paragraphs. The dated records (`CHANGELOG.md` and its four mirrors,
  `docs/dev/quality-ledger.md`) are history and are exempt: they may name a five-crate foundation,
  because that is what was true when they were written.
- Mapping order requires `IndexMap<CustomNode, CustomNode>`, not `HashMap`
- Comment/anchor preservation needs `comment.rs` scanner — do not skip it

## Code Style

```rust
// ✅ correct — constructor with no metadata
CustomNode::plain_scalar("value")

// ❌ wrong — manual 6-field boilerplate
CustomNode { style: ScalarStyle::Plain, ..Default::default() }
```

```rust
// ✅ correct — type annotations in pyo3 signature
#[pyo3(signature = (yaml: "str", resolve_merges: "bool" = true) -> "YamlDocument")]
fn parse(yaml: &str, resolve_merges: bool) -> PyResult<YamlDocument> { ... }

// ❌ wrong — missing signature or unquoted types
#[pyfunction]
fn parse(yaml: &str) -> PyResult<YamlDocument> { ... }
```

## Testing

- Round-Trip assertion is the primary test pattern
- The defence matrix is itself a gate: `python scripts/quality_matrix.py` measures which tiers exist per format and what they are blind to, and `tests/test_quality_matrix.py` fails if the measured blind spots differ from `.ci/quality-holes.json` in either direction. See [`QUALITY_MATRIX.md`](QUALITY_MATRIX.md) — closing a hole means deleting its registry entry, and accepting one means adding an entry with an exit criterion, in the same change.
- Use `uv run pytest tests/ -v` for normal test runs
- Free-threaded tests exclude numpy: `pytest tests/ -k "not numpy"`
- Compliance check via `tests/test_yaml_suite.py` on PRs to main

## Benchmark Standards

- All benchmarks use CodSpeed (`pytest-codspeed`); CI runs in `simulation` mode (instruction counts), not WallTime
- Cross-library comparison in `tests/test_benchmark_crosslib.py` (pyrs-yaml vs PyYAML vs ruamel vs ryaml vs yaml_edit vs yaml_rs)
- API benchmarks in `tests/test_benchmark_api.py` (safe_load, parse_stream, validate, etc.)
- Run benchmarks: `uv run pytest tests/test_benchmark_crosslib.py tests/test_benchmark_api.py --codspeed`
- Optional dependencies (yaml_rs, ryaml, yaml_edit) use `try/except` + `skipif`
- Bench files are excluded from normal `pytest` runs (require `--codspeed` flag)
- **Documented wall-clock numbers come only from the committed snapshot** — `scripts/bench_snapshot.py --update` measures the Python tables in batches (nine batches of at least 0.1 s, fastest reported) and records the environment with the reading in `.ci/perf-snapshot.json`; `--docs` rewrites all twelve locale tables from it. `tests/test_perf_docs.py` compares every cell back to the snapshot; it also derives every speedup range that appears as prose, on all 24 pages that carry one, and rejects the withdrawn 2026-08-17 date and ranges, so a hand-edited number is a red run, not a docs edit.
- **Instruction-count gate** (`.ci/ir-baseline.json`) — `python scripts/ir_gate.py` measures the hot paths in counted instructions and fails past the tolerance recorded in the baseline (its `tolerance_hint`, currently 0.5%; the 2% this file used to name was the WSL-vs-GitHub-runner drift, which the refresh channel now closes by measuring on the enforcing image instead of absorbing it in the line); `--update` re-baselines deliberately. Prefer it over CodSpeed numbers when judging whether *your own change* made the hot path do more work: CodSpeed compares against a baseline recorded on different hardware and is not reproducible below ~10% (three pushes doing strictly less work once read −7.7%, −10.5%, −9.8%). Needs valgrind — on this Windows box run it in WSL: `wsl -e bash -lc "cd /mnt/d/PycharmProjects/pyrs-yaml && python3 scripts/ir_gate.py"`. Both harnesses read the same documents from `pyrs_yaml_core::bench_inputs` — add a scenario there, never a second copy of a fixture. Growing that set is not a neutral act: adding scenarios reshapes what the release binary contains, so a scenario the harness never enters can move — PR #599 took the set from 22 to 26 and `stream_medium` read +0.01% then +0.63% on the enforcing image with identical engine code. A change that adds a scenario therefore dispatches the refresh on its own head and lets that run decide whether anything needs naming (the 26-to-30 growth crossed the line for nobody, so it was written with no name given); a move past the tolerance is met by naming it there, never by widening `tolerance_hint`.

## CLI (`pyrs-yaml` command)

Learnings from the initial CLI implementation (2026-08):

- **Architecture invariant**: the CLI lives behind the `cli` extra (`cyclopts>=4,<5`, Python 3.10+). The base install must stay zero-extra-dependency and Python 3.8 compatible — `python/pyrs_yaml/cli/__init__.py` guards the import and matches on `exc.name == "cyclopts"` so unrelated ImportErrors are not masked.
- **Cyclopts in-process gotchas** (all bit us once):
    - Call as `app(["fmt", "--indent", "4"])` — a token list, not splat args.
    - Every successful command still raises `SystemExit(0)` under the default `result_action`; tests must swallow exit 0 and propagate nonzero.
    - Do NOT use `cyclopts.types.StdioPath` — it requires Python 3.12+ (Path subclassing) while the `cli` extra targets 3.10+. Use plain `str` params with `Parameter(allow_leading_hyphen=True)` so the bare `-` token parses positionally, and route through `_io.is_stdio()`.
    - Parameters whose value may start with `-` (e.g. negative numbers in `set`) need `Parameter(allow_leading_hyphen=True)`.
- **CLI surface is complete** (verified 2026-09, supersedes the 2026-08 "known boundaries" list): multi-document is exposed via `-A/--all-docs` on fmt/get/set/delete/rename/validate/to-json; `sort-keys`, `move`, `frontmatter`, and `compliance` are first-class subcommands; `validate` disambiguates schema source with mutually exclusive `--schema` / `--schema-file` flags (no filename probe). Remaining boundaries: `move`'s destination must already exist (no auto-create); `python -m pyrs_yaml.compliance` still exists as a legacy entry point alongside the subcommand. All edit/query commands now accept `-A/--all-docs` for multi-document streams.
- **Docs cross-page anchors**: localized pages translate headings, so slug fragments like `editing.md#path-syntax` only resolve in `docs/en`. Link to the bare page for zh/ja/ko, or add an explicit `{ #id }` to the target heading first (per `DOCS_STANDARDS.md`; strict builds fail on missing anchors).

## Boundaries

✅ Always:

- Use `uv` for Python, `cargo` for Rust
- Preserve comments/anchors/order in Round-Trip
- Use `IndexMap` for Mapping order
- Implement `INPUT_TYPE`/`OUTPUT_TYPE` for all PyO3 types
- Use `#[pyo3(signature = "...")]` with double-quoted types
- Run prek hooks before committing — and fold their edits into the commit you push: `prek run --all-files` fixes files in the *working copy*, so if you squashed first, the pushed commit still carries what the hook set rejects (measured: CI's `Hygiene` job reddened on a `rumdl fmt` rewrite that sat unsquashed beside it).
- Before declaring a CI check enforced, ask what merge decision consumes it. A matrix leg that nobody requires is a report: `Test matrix (all legs)` (`scripts/check_matrix_verdict.py`) is the single name to require, and it reads `success` only - `skipped`/`cancelled` legs are refusals.
- **Bench design**: Include setup costs (e.g. `ast.clone()`) in both sides of the comparison, or document them as external
- **pyo3 visibility**: `#[pyclass]` methods may need `#[allow(dead_code)]` even when `pub` — Rust visibility != Python visibility

⚠️ Ask First:

- Adding a new external dependency (Rust or Python)
- Changing the AST node structure
- Modifying `.pyi` stubs manually
- Using `#[pymodule_init]` or `#[pymodule]` function form

🚫 Never:

- Use `serde_yaml` or `yaml-rust2` `YamlLoader::load_from_str()`
- Use `.unwrap()` or `.expect()` in business logic
- Skip the GIL release in hot pure-Rust paths (parse, edit, AST serialize — use `py.detach`)
- Use `pip` or system Python — always `uv`
- Use `git commit --no-verify` to bypass prek hooks
- Edit `.pyi` files manually instead of regenerating via maturin

⚠️ 架构决策（不重构，防过拟合）：

- 不统一 `_parse_jsonpath` 与 `parse_path_segments`：查询（find/walk 多节点）与编辑（set/insert 单节点）路径语义天然不同，重写 find/walk 风险高 ROI 低。
- 不消除 direct_dump 与 serializer 的近似镜像：输入对象不同（Python 对象 vs CustomNode），细节必然不同，强行抽象引入过度泛化。
- 不重写绑定层 `py/editing/mod.rs` 与 core editing 的边界：编辑语义直接操作 AST，分层边界在当前版本合理，需未来版本级设计。

## Release Process

Learnings from v0.10.0 (2026-08-01) — three pitfalls, each is a separate regression:

1. **`publish.yml` must never use `--generate-stubs`.** Maturin's introspection helper is built for the runner host glibc and cannot load the extension inside old manylinux/musllinux containers or under qemu → every linux/musllinux wheel build fails with `Failed to introspect the built libraries to generate type stubs`. macos/windows pass — the failure is silent until Release.
2. **The committed `.pyi` must NOT appear in `.gitignore`.** Maturin bundles `python-source` files but skips anything gitignored → the wheel silently ships without `pyrs_yaml.pyi`. Regenerate via `uv run maturin build --release --generate-stubs`, extract, commit with `git add -f`.
3. **Publish runs only on tag push** (it validates at release time, never on PRs/pushes). Verify the tag target before pushing: `git ls-remote --tags origin`. Force-pushing a tag re-triggers the workflow; a tag on a bad commit must be canceled + re-tagged.

Release checklist: bump `Cargo.toml` + `Cargo.lock`, rename `[Unreleased]` → `[X.Y.Z]` (with date) in `CHANGELOG.md` + `docs/{en,ja,ko,zh}/changelog.md` (do NOT skip `docs/en/changelog.md` — it is the canonical English mirror and feeds the deployed docs site; v0.7.0–v0.10.0 were silently missed and only caught during post-release site verification), add a `ROADMAP.md` table row, then PR → CI → rebase merge → lightweight tag → watch Publish run (Release job publishes to PyPI only when all build jobs pass). Confirm before tagging that the release build's Rust compiler is still the version `.ci/ir-baseline.json` records: `publish.yml` passes it to `maturin-action` as `rust-toolchain` (and the `pyq` job installs the same version), `ci.yml` builds the same wheel recipe with it, and `tests/test_publish_toolchain.py` reddens when the pin and the baseline disagree. Moving that compiler is a measured decision — the floors and the shipped artifact describe the same build only while the two stay together.

Post-release verification (mandatory): confirm the deployed docs changelog shows the new version (`https://759401524.github.io/pyrs-yaml/en/changelog/`) — a stale page means a docs mirror was missed; smoke-test the PyPI wheel in a clean venv (`__version__`, parse/edit/round-trip, `py.typed` + `.pyi` present). When interpreting CodSpeed comparisons, check for runner hardware differences first (Simulation-mode flags that ran across a CPU change are hardware-attributable, not code regressions).

## Changelog Mirrors

- **Root `CHANGELOG.md` is canonical English**. The 4 locale mirrors (`docs/{en,ja,ko,zh}/changelog.md`) share structural parity (same version headers, `[Unreleased]` present) but the Unreleased entry is translated per locale. Historical entries remain English in all mirrors.
- **Workflow**: update root first, then translate into each locale. Never commit partial updates.
- **Guard**: `scripts/check_changelog_mirrors.py` checks structural parity (not verbatim equality) — use it before committing.

## Version Control & Commits

See [`CONTRIBUTING.md`](CONTRIBUTING.md#version-control--commits) for the canonical commit and PR conventions — staging rules, quality gates, push/merge policy, and commit message format.

## Docs Standards

When writing or editing Markdown documentation under `docs/`, follow [`DOCS_STANDARDS.md`](DOCS_STANDARDS.md) for frontmatter, admonitions, content tabs, snippet references, code block, and link/path rules.
