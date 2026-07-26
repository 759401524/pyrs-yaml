"""
Type stubs for pyyaml-rs.

This module provides type hints for the pyyaml_rs native module.
"""

from typing import Any, Dict, Iterator, List, Optional, Tuple, Union, overload

__version__: str

class YamlParseError(ValueError):
    """YAML parsing error (inherits from ValueError)."""
    ...

class YamlSerializeError(ValueError):
    """YAML serialization error (inherits from ValueError)."""
    ...

class YamlTypeError(TypeError):
    """YAML type conversion error (inherits from TypeError)."""
    ...

class YamlDocument:
    """
    A parsed YAML document.

    This class provides methods to access and manipulate the parsed YAML data.

    Example:
        >>> doc = pyyaml_rs.parse("key: value")
        >>> print(doc.to_yaml())
        key: value
        >>> print(doc.get("key"))
        value
    """

    def to_yaml(self) -> str:
        """Convert the AST back to YAML string."""
        ...

    def to_yaml_with_options(
        self,
        indent_size: int = 2,
        explicit_start: bool = False,
        explicit_end: bool = False,
        sort_keys: bool = False,
    ) -> str:
        """Convert the AST back to YAML string with custom options."""
        ...

    def to_dict(self) -> Union[Dict[str, Any], List[Any]]:
        """Convert the AST to a Python dict/list."""
        ...

    def get(self, key: str, default: Any = None) -> Any:
        """
        Get a value by key (for mapping root).

        Args:
            key: The key to look up
            default: Value to return if key is not found (default: None)

        Returns:
            The value associated with the key, or default if not found.
        """
        ...

    def root_type(self) -> str:
        """
        Get the root node type as string.

        Returns:
            One of: "scalar", "mapping", "sequence", "null", "alias"
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __contains__(self, key: str) -> bool: ...
    def __len__(self) -> int: ...
    def __iter__(self) -> Iterator[Any]: ...

    @overload
    def __getitem__(self, key: str) -> Any: ...
    @overload
    def __getitem__(self, key: int) -> Any: ...
    def __getitem__(self, key: Union[str, int]) -> Any:
        """
        Get a value by key (mapping) or index (sequence).

        Raises:
            KeyError: If key is not found in a mapping.
            IndexError: If index is out of range for a sequence.
            TypeError: If the document is not subscriptable.
        """
        ...

def parse(yaml: Union[str, bytes], resolve_merges: bool = True) -> YamlDocument:
    """
    Parse a YAML string or bytes into a YamlDocument.

    Args:
        yaml: A string or bytes containing YAML content
        resolve_merges: Whether to resolve merge keys (<<) after parsing (default: True)

    Returns:
        A YamlDocument containing the parsed YAML.

    Raises:
        ValueError: If the YAML is invalid.
        TypeError: If input is not str or bytes.
    """
    ...

def parse_file(path: str) -> YamlDocument:
    """
    Parse a YAML file.

    Args:
        path: Path to the YAML file

    Returns:
        A YamlDocument containing the parsed YAML.

    Raises:
        IOError: If the file cannot be read.
        ValueError: If the YAML is invalid.
    """
    ...

def parse_all_docs(yaml: str) -> List[YamlDocument]:
    """
    Parse multiple YAML documents from a string.

    Args:
        yaml: A string containing multiple YAML documents separated by ---

    Returns:
        A list of YamlDocument objects.
    """
    ...

def safe_load(yaml: str) -> Union[Dict[str, Any], List[Any]]:
    """
    Parse a YAML string and return Python dict/list (PyYAML compatible).

    Args:
        yaml: A string containing YAML content

    Returns:
        A Python object (dict for mappings, list for sequences, etc.)

    Example:
        >>> data = pyyaml_rs.safe_load("key: value")
        >>> print(data)
        {'key': 'value'}
    """
    ...

def safe_loads(yaml: str) -> List[Union[Dict[str, Any], List[Any]]]:
    """
    Parse multiple YAML documents (PyYAML compatible).

    Args:
        yaml: A string containing multiple YAML documents separated by ---

    Returns:
        A list of Python objects.

    Example:
        >>> docs = pyyaml_rs.safe_loads("a: 1\\n---\\nb: 2")
        >>> print(len(docs))
        2
    """
    ...

def safe_dump(data: Union[Dict[str, Any], List[Any]]) -> str:
    """
    Serialize a Python dict/list to YAML string (PyYAML compatible).

    Args:
        data: A Python dict or list to serialize

    Returns:
        A YAML string representation.

    Example:
        >>> yaml_str = pyyaml_rs.safe_dump({"key": "value"})
        >>> print(yaml_str)
        key: value
    """
    ...

def safe_dumps(data: Union[Dict[str, Any], List[Any]]) -> str:
    """
    Alias for safe_dump (deprecated, use safe_dump instead).
    """
    ...

def from_dict(data: Dict[str, Any]) -> str:
    """
    Convert a Python dict to YAML string (yamlium compatible).

    Args:
        data: A Python dict to convert

    Returns:
        A YAML string representation.
    """
    ...

def from_json(json_str: str) -> str:
    """
    Convert a JSON string to YAML string (yamlium compatible).

    Args:
        json_str: A JSON string to convert

    Returns:
        A YAML string representation.
    """
    ...

def dump_file(data: Any, path: str) -> None:
    """
    Dump (serialize) a Python object to YAML and write to a file.

    Args:
        data: A Python dict, list, or scalar to serialize
        path: File path to write the YAML output to

    Raises:
        IOError: If the file cannot be written.
    """
    ...

def read_markdown(path: str) -> Tuple[Optional[Dict[str, Any]], str]:
    """
    Read YAML frontmatter from a markdown file.

    Args:
        path: Path to the markdown file

    Returns:
        A tuple of (frontmatter_dict, content_string).
        If no frontmatter is found, frontmatter is None.
    """
    ...

def read_markdown_str(content: str) -> Tuple[Optional[Dict[str, Any]], str]:
    """
    Read YAML frontmatter from a markdown string.

    Args:
        content: A markdown string

    Returns:
        A tuple of (frontmatter_dict, content_string).
        If no frontmatter is found, frontmatter is None.
    """
    ...

def set_language(lang: str) -> None:
    """
    Set the language for error messages.

    Args:
        lang: Language code, supports "en" and "zh-CN"

    Raises:
        ValueError: If the language is not supported.
    """
    ...

def get_language() -> str:
    """
    Get the current language for error messages.

    Returns:
        The current language code (default: "en").
    """
    ...

def list_languages() -> list[str]:
    """
    List all supported languages.

    Returns:
        List of language codes (e.g., ["en", "zh-CN"]).
    """
    ...
