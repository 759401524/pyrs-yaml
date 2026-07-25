import pyyaml_rs


def test_from_dict():
    """Test from_dict function"""
    data = {'name': 'John', 'age': 30, 'items': [1, 2, 3]}
    yaml_str = pyyaml_rs.from_dict(data)
    assert 'name: John' in yaml_str
    assert 'age: 30' in yaml_str


def test_from_json():
    """Test from_json function"""
    json_str = '{"name": "Alice", "active": true}'
    yaml_str = pyyaml_rs.from_json(json_str)
    assert 'name: Alice' in yaml_str
    assert 'active: true' in yaml_str


def test_read_markdown_str():
    """Test read_markdown_str function"""
    md_content = """---
title: My Post
tags: [python, yaml]
---
# Hello World
This is the body.
"""
    frontmatter, content = pyyaml_rs.read_markdown_str(md_content)
    assert frontmatter is not None
    assert frontmatter['title'] == 'My Post'
    assert '# Hello World' in content


def test_read_markdown_no_frontmatter():
    """Test read_markdown_str with no frontmatter"""
    md_content = "# Hello World\nThis is the body."
    frontmatter, content = pyyaml_rs.read_markdown_str(md_content)
    assert frontmatter is None
    assert content == md_content


def test_from_dict_nested():
    """Test from_dict with nested structure"""
    data = {
        'app': {
            'name': 'myapp',
            'version': '1.0'
        },
        'features': ['auth', 'logging']
    }
    yaml_str = pyyaml_rs.from_dict(data)
    assert 'app:' in yaml_str
    assert 'name: myapp' in yaml_str
    assert '- auth' in yaml_str


def test_from_json_nested():
    """Test from_json with nested structure"""
    json_str = '{"database": {"host": "localhost", "port": 5432}}'
    yaml_str = pyyaml_rs.from_json(json_str)
    assert 'database:' in yaml_str
    assert 'host: localhost' in yaml_str
    assert 'port: 5432' in yaml_str
