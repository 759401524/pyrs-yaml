"""
Feature tests — Markdown frontmatter, read_markdown.
"""

import tempfile
from pathlib import Path

import pyyaml_rs


class TestReadMarkdown:
    """Test read_markdown and read_markdown_str functions"""

    def test_read_markdown_with_frontmatter(self):
        md = "---\ntitle: My Post\ntags: [python]\n---\n# Content"
        frontmatter, content = pyyaml_rs.read_markdown_str(md)
        assert frontmatter is not None
        assert frontmatter["title"] == "My Post"
        assert "# Content" in content

    def test_read_markdown_no_frontmatter(self):
        md = "# Just content\nNo frontmatter here."
        frontmatter, content = pyyaml_rs.read_markdown_str(md)
        assert frontmatter is None
        assert content == md

    def test_read_markdown_empty_frontmatter(self):
        md = "---\n---\n# Content"
        frontmatter, _content = pyyaml_rs.read_markdown_str(md)
        assert frontmatter is None

    def test_read_markdown_file(self):
        test_file = str(Path(tempfile.gettempdir()) / "test_readme.md")
        with Path(test_file).open("w") as f:
            f.write("---\ntitle: Test\n---\nContent")
        try:
            frontmatter, _content = pyyaml_rs.read_markdown(test_file)
            assert frontmatter is not None
            assert frontmatter["title"] == "Test"
        finally:
            if Path(test_file).exists():
                Path(test_file).unlink()

    def test_from_dict_nested(self):
        """Test from_dict with deeply nested structures."""
        data = {"app": {"name": "myapp", "version": "1.0"}, "features": ["auth", "logging"]}
        yaml_str = pyyaml_rs.from_dict(data)
        assert "app:" in yaml_str
        assert "name: myapp" in yaml_str
        assert "- auth" in yaml_str

    def test_from_json_nested(self):
        """Test from_json with nested database config."""
        json_str = '{"database": {"host": "localhost", "port": 5432}}'
        yaml_str = pyyaml_rs.from_json(json_str)
        assert "database:" in yaml_str
        assert "host: localhost" in yaml_str
        assert "port: 5432" in yaml_str
