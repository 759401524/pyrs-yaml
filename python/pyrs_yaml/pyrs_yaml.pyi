"""
A Python module implemented in Rust.

pyrs-yaml: high-performance YAML parsing with perfect round-trip support.
"""

from _typeshed import Incomplete
from typing import Any, final

@final
class StreamIterator:
    """
    YAML event stream iterator, yielding parsed events one by one.
    """
    def __iter__(self, /) -> StreamIterator:
        """
        Return self (iterator protocol).
        """
    def __next__(self, /) -> dict |None:
        """
        Yield the next event dict; return `None` when the stream ends.
        """

@final
class YAML:
    """
    Configured YAML parser instance (rt / safe / full).
    """
    def __new__(cls, /, typ: "str" = "rt", schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = False) -> YAML:
        """
        Create a YAML instance; `typ` can be `rt`/`safe`/`full`.
        """
    def dump_file(self, /, path: "str", iterable: "Any", explicit_start: "bool" = False, explicit_end: "bool" = False, sort_keys: "bool" = False) -> "None":
        """
        Streaming writer: serialize documents to `path` (Rust File, no GIL blocking).
        """
    def dump_stream(self, /, file_obj: "Any", iterable: "Any", explicit_start: "bool" = False, explicit_end: "bool" = False, sort_keys: "bool" = False) -> "None":
        """
        Streaming writer: serialize documents to `file_obj` (write(str)), constant memory.
        """
    def load_stream(self, /, file_obj: "Any") -> "YamlStream":
        """
        Lazy event iterator: incrementally read from `file_obj` (read() returns str or bytes).
        """
    def load_stream_file(self, /, path: "str") -> "YamlStream":
        """
        Lazy event iterator: incrementally read from file path (Rust File, no GIL blocking).
        """
    def parse(self, /, yaml: "str | bytes") -> "YamlDocument":
        """
        Parse a YAML string and return a `YamlDocument`.
        """
    def parse_all_docs(self, /, yaml: "str") -> "list[YamlDocument]":
        """
        Parse multi-document YAML and return a list of `YamlDocument`.
        """
    def parse_file(self, /, path: "str") -> "YamlDocument":
        """
        Parse a YAML file and return a `YamlDocument`.
        """
    def safe_load(self, /, yaml: "str") -> "dict[str, Any] | list[Any]":
        """
        Parse YAML into a dict/list (resolves anchors and merges).
        """
    def safe_loads(self, /, yaml: "str") -> "list[dict[str, Any] | list[Any]]":
        """
        Parse multi-document YAML into a list of dicts/lists.
        """

@final
class YamlDocument:
    """
    Round-trip editable YAML document with transaction support, path editing, and source preservation.
    """
    def __contains__(self, key: str, /) -> bool:
        """
        Check if a key exists in the mapping.
        """
    def __delitem__(self, key: str, /) -> None:
        """
        Delete a mapping key, `del doc['key']`.
        """
    def __enter__(self, /) -> YamlDocument:
        """
        Enter a transaction scope: snapshot AST + splice state. `with doc:` exits cleanly
        preserving edits; exceptions roll back the snapshot.
        """
    def __exit__(self, /, exc_type: "Any | None" = None, exc_value: "Any | None" = None, tb: "Any | None" = None) -> "bool": ...
    def __getitem__(self, key: Any, /) -> Any:
        """
        Access a child node by key (mapping) or index (sequence).
        """
    def __iter__(self, /) -> Any:
        """
        Return an iterator over keys (mapping) or values (sequence).
        """
    def __len__(self, /) -> int:
        """
        Return the number of mapping entries or sequence length.
        """
    def __repr__(self, /) -> str:
        """
        Return a `YamlDocument(...)` representation.
        """
    def __setitem__(self, key: str, value: Any, /) -> None:
        """
        Set the value for a mapping key, `doc['key'] = value`.
        """
    def __str__(self, /) -> str:
        """
        Return the YAML string representation.
        """
    def _append_path(self, /, segments: "list", value: "Any") -> "None":
        """
        Append a value by path (internal, appends to a sequence).
        """
    def _delete_path(self, /, segments: "list") -> "None":
        """
        Delete a node by path (internal, called by `__delitem__`).
        """
    def _get_anchor(self, /, segments: "list") -> "str | None":
        """
        Get the anchor name on the node at `segments` (internal).
        """
    def _get_chomping(self, /, segments: "list") -> "str | None":
        """
        Get the chomping indicator on the node at `segments` (internal).
        """
    def _get_comment(self, /, segments: "list") -> "str | None":
        """
        Get the comment text on the node at `segments` (internal).
        """
    def _get_flow_style(self, /, segments: "list") -> "bool | None":
        """
        Get the flow style on the node at `segments` (internal).
        """
    def _get_scalar_style(self, /, segments: "list") -> "str | None":
        """
        Get the scalar style on the node at `segments` (internal).
        """
    def _get_tag(self, /, segments: "list") -> "str | None":
        """
        Get the YAML tag on the node at `segments` (internal).
        """
    def _insert_path(self, /, segments: "list", index: "int", value: "Any") -> "None":
        """
        Insert a value by path (internal, inserts at a sequence position).
        """
    def _remove_anchor_path(self, /, segments: "list") -> "None":
        """
        Remove the anchor on the node at `segments` (internal).
        """
    def _remove_comment_path(self, /, segments: "list") -> "None":
        """
        Remove the comment on the node at `segments` (internal).
        """
    def _remove_tag_path(self, /, segments: "list") -> "None":
        """
        Remove the YAML tag on the node at `segments` (internal).
        """
    def _rename_path(self, /, segments: "list", new_key: "str") -> "None":
        """
        Rename a mapping key by path (internal).
        """
    def _revision(self, /) -> int:
        """
        Return the current revision number (incremented on each edit).
        """
    def _scalar_paths(self, /) -> list[Any]:
        """
        Walk only scalar/null nodes, returning their path tuples.
        """
    def _set_anchor_path(self, /, segments: "list", name: "str") -> "None":
        """
        Set an anchor on the node at `segments` (internal).
        """
    def _set_chomping_path(self, /, segments: "list", chomping: "str") -> "None":
        """
        Set the chomping indicator on the node at `segments` (internal).
        """
    def _set_comment_path(self, /, segments: "list", text: "str", standalone: "bool" = True) -> "None":
        """
        Set a comment on the node at `segments` (internal).
        """
    def _set_flow_style_path(self, /, segments: "list", flow: "bool") -> "None":
        """
        Set the flow style on the node at `segments` (internal).
        """
    def _set_path(self, /, segments: "list", value: "Any", create_missing: "bool" = False) -> "None":
        """
        Set a value by path (internal, called by `__setitem__`).
        """
    def _set_scalar_style_path(self, /, segments: "list", style: "str") -> "None":
        """
        Set the scalar style on the node at `segments` (internal).
        """
    def _set_tag_path(self, /, segments: "list", tag: "str") -> "None":
        """
        Set a YAML tag on the node at `segments` (internal).
        """
    def _walk_paths(self, /) -> list[Any]:
        """
        Walk the AST depth-first, returning a list of path tuples.
        Each path tuple contains strings (mapping keys) and ints (sequence indices).
        The first element is always the root node (empty path).
        """
    def flush_source(self, /) -> None:
        """
        Lazily re-serialize the AST into `source` if edits have occurred.
        """
    def get(self, /, key: "str", default: "Any" = None) -> "Any":
        """
        Access a value by top-level mapping key, returning `default` if not found.
        Path-based access is available via `find()` / `node()`.
        """
    def reparse(self, /, resolve_merges: "bool" = True, schema: "str" = "core") -> "None":
        """
        Reparse the current source, optionally changing merge behavior and schema.
        """
    def root_type(self, /) -> str:
        """
        Return the root node type: `scalar`/`mapping`/`sequence`/`null`/`alias`.
        """
    def source(self, /) -> str:
        """
        Return the current YAML source string.
        """
    def to_dict(self, /) -> Any:
        """
        Convert the document to a Python dict/list, resolving anchor references.
        """
    def to_json(self, /, indent: "int" = 2) -> "str":
        """
        Serialize to a JSON string (via Python `json.dumps`).
        """
    def to_yaml(self, /) -> str:
        """
        Serialize the document to a YAML string (default 2-space indent).
        """
    def to_yaml_with_options(self, /, indent_size: "int" = 2, explicit_start: "bool" = False, explicit_end: "bool" = False, sort_keys: "bool" = False, max_depth: "int" = 1000, width: "int" = 80, indent_mapping: "int | None" = None, indent_sequence: "int | None" = None, indent_offset: "int | None" = None) -> "str":
        """
        Serialize with customizable indent, sorting, and explicit start/end markers.
        """
    def validate(self, /, schema: "str | dict[str, Any]") -> "None":
        """
        Validate the document against a JSON Schema.
        """
    def version(self, /) -> str:
        """
        Return the YAML version string.
        """

def clear_tag_handlers() -> None:
    """
    Clear all tag handlers.
    """

def clear_type_handlers() -> None:
    """
    Clear all custom type handlers.
    """

def detect_language() -> "str":
    """
    Detect the system default language.
    """

def dump_file(data: "Any", path: "str") -> "None":
    """
    Serialize a Python object to YAML and write to a file.
    """

def from_dict(data: "dict[str, Any] | list[Any]") -> "str":
    """
    Convert a Python dict/list to a YAML string (auto-selects block/flow style).
    """

def from_json(json_str: "str") -> "str":
    """
    Convert a JSON string to a YAML string.
    """

def get_language() -> "str":
    """
    Return the current error message language.
    """

def list_languages() -> "list[str]":
    """
    List supported language codes.
    """

def list_schemas() -> "list[str]":
    """
    List all registered schema names (built-in + custom).

    Returns the four built-in schemas (`failsafe`, `json`, `core`, `yaml1.1`)
    plus any schemas registered via `register_schema()` / `load_schema()`.
    """

def load_schema(name: "str", path: "str") -> "None":
    """
    Register a YAML Schema Language schema from a file.

    Reads the schema definition from `path` (a YAML file with `name`/`extends`/`rules`
    structure) and registers it under `name`. Equivalent to calling
    `register_schema(name, open(path).read())` but handles file I/O in Rust.
    """

def negotiate_language(user_locales: "list[str]", default: "str" = "en") -> "str":
    """
    Negotiate a language from user locale list and default.
    """

def parse(yaml: "str | bytes", resolve_merges: "bool" = True, schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = False) -> "YamlDocument":
    """
    Parse a YAML string (str or bytes) and return an editable `YamlDocument`.
    """

def parse_all_docs(yaml: "str", resolve_merges: "bool" = True, schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = False) -> "list[YamlDocument]":
    """
    Parse a multi-document YAML stream and return all `YamlDocument` objects.
    """

def parse_file(path: "str", schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = False) -> "YamlDocument":
    """
    Parse a YAML file and return an editable `YamlDocument`.
    """

def parse_stream(yaml: "str | bytes", on_event: "Callable[[dict[str, Any]], bool] | None" = None, max_depth: "int" = 1000) -> "StreamIterator | None":
    """
    Event-stream parsing. With `on_event` callback, consumes events and returns `None`. Otherwise returns a lazy `StreamIterator`.
    """

def read_markdown(path: "str", schema: "str" = "core", max_depth: "int" = 1000) -> "tuple[dict[str, Any] | None, str]":
    """
    Read a Markdown file and extract YAML front matter, returning `(frontmatter, body)`.
    """

def read_markdown_str(content: "str", schema: "str" = "core", max_depth: "int" = 1000) -> "tuple[dict[str, Any] | None, str]":
    """
    Extract YAML front matter from a Markdown string, returning `(frontmatter, body)`.
    """

def register_schema(name: "str", schema_yaml: "str") -> None:
    """
    Register a YAML Schema Language schema under a name.

    `schema_yaml` is a schema definition in YAML format:
    ```yaml
    name: myapp
    extends: core
    rules:
      - pattern: "^0x[0-9a-fA-F]+$"
        type: int
    ```
    Once registered, the schema can be used as `YAML(schema="myapp")`.
    """

def register_tag(name: "str", handler: "Py<PyAny>", priority: "u32" = 0) -> None:
    """
    Register a custom tag handler.
    """

def register_type(name: "str", handler: "Py<PyAny>") -> None:
    """
    Register a custom type handler (Community Plugins).
    """

def remove_tag(name: "str") -> None:
    """
    Remove a specific tag handler.
    """

def remove_type(name: "str") -> None:
    """
    Remove a specific custom type handler.
    """

def safe_dump(data: "dict[str, Any] | list[Any]") -> "str":
    """
    Serialize a Python dict/list to a YAML string.
    """

def safe_load(yaml: "str", schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = False) -> "dict[str, Any] | list[Any]":
    """
    Parse YAML into a Python dict/list, resolving anchors and merges.
    """

def safe_loads(yaml: "str", schema: "str" = "core", max_depth: "int" = 1000, allow_duplicate_keys: "bool" = False) -> "list[dict[str, Any] | list[Any]]":
    """
    Parse a multi-document YAML stream into a list of dicts/lists.
    """

def set_language(lang: "str") -> "None":
    """
    Set the error message language.
    """

def validate_custom_types(obj: "Py<PyAny>") -> None:
    """
    Validate a Python object against all registered CustomType validators.

    Recursively walks dicts and lists. For each value that matches a
    registered type's `python_type`, calls the handler's `validate` method.
    Raises `ValueError` if any value fails validation.
    """

def __getattr__(name: str) -> Incomplete: ...
