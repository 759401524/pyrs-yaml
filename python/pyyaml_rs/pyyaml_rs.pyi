"""
Type stubs for pyyaml-rs.

This module provides type hints for the pyyaml_rs native module.
"""

from typing import Any, Dict, List, Optional, Tuple, Union

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
        """
        Convert the AST back to YAML string.
        
        Returns:
            A YAML string representation of the document.
        """
        ...
    
    def to_dict(self) -> Union[Dict[str, Any], List[Any]]:
        """
        Convert the AST to a Python dict/list.
        
        Returns:
            A Python object (dict for mappings, list for sequences, etc.)
        """
        ...
    
    def get(self, key: str) -> Optional[Any]:
        """
        Get a value by key (for mapping root).
        
        Args:
            key: The key to look up
            
        Returns:
            The value associated with the key, or None if not found.
        """
        ...
    
    def root_type(self) -> str:
        """
        Get the root node type as string.
        
        Returns:
            One of: "scalar", "mapping", "sequence", "null", "alias"
        """
        ...

def parse(yaml: str) -> YamlDocument:
    """
    Parse a YAML string into a YamlDocument.
    
    Args:
        yaml: A string containing YAML content
        
    Returns:
        A YamlDocument containing the parsed YAML.
        
    Raises:
        ValueError: If the YAML is invalid.
        
    Example:
        >>> doc = pyyaml_rs.parse("key: value")
        >>> print(doc.to_yaml())
        key: value
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
        
    Example:
        >>> doc = pyyaml_rs.parse_file("config.yaml")
        >>> print(doc.to_yaml())
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
        >>> docs = pyyaml_rs.safe_loads("a: 1\n---\nb: 2")
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
    Alias for safe_dump.
    
    Args:
        data: A Python dict or list to serialize
        
    Returns:
        A YAML string representation.
    """
    ...

def from_dict(data: Dict[str, Any]) -> str:
    """
    Convert a Python dict to YAML string (yamlium compatible).
    
    Args:
        data: A Python dict to convert
        
    Returns:
        A YAML string representation.
        
    Example:
        >>> yaml_str = pyyaml_rs.from_dict({"name": "Alice"})
        >>> print(yaml_str)
        name: Alice
    """
    ...

def from_json(json_str: str) -> str:
    """
    Convert a JSON string to YAML string (yamlium compatible).
    
    Args:
        json_str: A JSON string to convert
        
    Returns:
        A YAML string representation.
        
    Example:
        >>> yaml_str = pyyaml_rs.from_json('{"name": "Alice"}')
        >>> print(yaml_str)
        name: Alice
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
        
    Example:
        >>> frontmatter, content = pyyaml_rs.read_markdown("post.md")
        >>> if frontmatter:
        ...     print(frontmatter["title"])
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
        
    Example:
        >>> text = "---\\ntitle: Post\\n---\\n# Content"
        >>> frontmatter, content = pyyaml_rs.read_markdown_str(text)
        >>> print(frontmatter["title"])
        Post
    """
    ...
