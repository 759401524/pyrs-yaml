import pyyaml_rs


def test_escape_sequences():
    """Test escape sequences in double-quoted scalars"""
    yaml_str = 'text: "hello\\nworld"'
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("text") == "hello\nworld"


def test_unicode_escape():
    """Test unicode escape sequences"""
    yaml_str = 'text: "\\u0041\\u0042\\u0043"'
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("text") == "ABC"


def test_special_escapes():
    """Test special escape sequences"""
    yaml_str = 'text: "tab\\there\\nnewline"'
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("text") == "tab\there\nnewline"


def test_backslash_escape():
    """Test backslash escape"""
    yaml_str = 'text: "back\\\\slash"'
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("text") == "back\\slash"


def test_quote_escape():
    """Test quote escape"""
    yaml_str = 'text: "say \\"hello\\""'
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("text") == 'say "hello"'


def test_null_escape():
    """Test null escape"""
    yaml_str = 'text: "null\\0char"'
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("text") == "null\x00char"


def test_tab_escape():
    """Test tab escape"""
    yaml_str = 'text: "tab\\tchar"'
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("text") == "tab\tchar"


def test_newline_escape():
    """Test newline escape"""
    yaml_str = 'text: "newline\\nchar"'
    doc = pyyaml_rs.parse(yaml_str)
    assert doc.get("text") == "newline\nchar"
