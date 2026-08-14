"""Concurrency / thread-safety smoke tests.

The Rust extension releases the GIL during pure-Rust compute (parse/serialize),
so running many operations on Python threads exercises the parallel Rust paths.
These tests assert correctness and absence of crashes/data races under
concurrency; they do not require the free-threaded build.
"""

from concurrent.futures import ThreadPoolExecutor

import pyrs_yaml

# A small set of representative documents whose load/dump must stay stable.
DOCUMENTS = [
    "name: alice\nage: 30\nroles:\n  - admin\n  - user\n",
    "items:\n  - {id: 1, name: x}\n  - {id: 2, name: y}\n",
    "a: 1\nb: 2\nc: [3, 4, 5]\n",
    "'quoted key': value\n",
    "nested:\n  one:\n    two:\n      three: deep\n",
]

EXPECTED = {doc: pyrs_yaml.parse(doc).to_dict() for doc in DOCUMENTS}


def test_concurrent_parse_is_stable():
    # Parsing the same source on many threads must yield identical results.
    def work(_):
        return pyrs_yaml.parse(DOCUMENTS[0]).to_dict()

    with ThreadPoolExecutor(max_workers=8) as ex:
        results = list(ex.map(work, range(200)))
    assert results == [EXPECTED[DOCUMENTS[0]]] * 200


def test_concurrent_roundtrip_is_correct():
    # Independent load/dump operations across threads must stay correct.
    def work(i):
        doc = DOCUMENTS[i % len(DOCUMENTS)]
        loaded = pyrs_yaml.safe_load(doc)
        assert loaded == EXPECTED[doc]
        dumped = pyrs_yaml.safe_dump(loaded)
        assert pyrs_yaml.safe_load(dumped) == loaded
        return True

    with ThreadPoolExecutor(max_workers=8) as ex:
        assert all(ex.map(work, range(400)))


def test_concurrent_documents_independent():
    # Building/parsing separate documents concurrently must not share state.
    def work(i):
        src = f"idx: {i}\nlist:\n  - {i}\n  - {i + 1}\n"
        d = pyrs_yaml.parse(src)
        assert d.get("idx") == i
        assert d.get("list") == [i, i + 1]
        return True

    with ThreadPoolExecutor(max_workers=8) as ex:
        assert all(ex.map(work, range(300)))


def test_concurrent_edits_are_isolated():
    # Editing one YamlDocument instance must not affect another instance.
    def work(i):
        d = pyrs_yaml.parse(DOCUMENTS[2])
        d._set_path(["c", 0], i)
        assert d.to_dict()["c"][0] == i
        assert d.to_dict()["a"] == 1
        return True

    with ThreadPoolExecutor(max_workers=8) as ex:
        assert all(ex.map(work, range(200)))


def test_concurrent_tag_registry_register_clear():
    # tag_registry uses a Mutex; concurrent register/clear must not crash and
    # must end in a consistent state.
    def work(_):
        @pyrs_yaml.register_tag("!c")
        def h(node):
            return f"h:{node}"

        pyrs_yaml.clear_tag_handlers()
        return True

    def read():
        return pyrs_yaml.parse("x: !c v\n").get("x")

    with ThreadPoolExecutor(max_workers=8) as ex:
        assert all(ex.map(work, range(80)))

    # After all workers clear, a fresh register still resolves.
    @pyrs_yaml.register_tag("!c")
    def h2(node):
        return f"ok:{node}"

    assert pyrs_yaml.parse("x: !c v\n").get("x") == "ok:v"
    pyrs_yaml.clear_tag_handlers()


def test_concurrent_type_registry_register_remove():
    # type_registry Mutex under concurrent register/remove; no crash, consistent end state.
    class T(pyrs_yaml.CustomType):
        python_type = str

        def can_parse(self, value):
            return True

        def from_yaml(self, value):
            return "T:" + value

    def work(_):
        pyrs_yaml.register_type("!t", T())
        pyrs_yaml.remove_type("!t")
        return True

    with ThreadPoolExecutor(max_workers=8) as ex:
        assert all(ex.map(work, range(80)))
    assert pyrs_yaml.safe_load("x: !t v\n")["x"] == "v"
    pyrs_yaml.clear_type_handlers()
