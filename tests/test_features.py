"""Feature tests — Markdown frontmatter, read_markdown."""

import tempfile
from pathlib import Path

import pytest

import pyrs_yaml


class TestReadMarkdown:
    """Test read_markdown and read_markdown_str functions"""

    @pytest.mark.parametrize(
        "md,has_frontmatter,title",
        [
            ("---\ntitle: My Post\ntags: [python]\n---\n# Content", True, "My Post"),
            ("# Just content\nNo frontmatter here.", False, None),
            ("---\n---\n# Content", False, None),
        ],
        ids=["with-frontmatter", "no-frontmatter", "empty-frontmatter"],
    )
    def test_read_markdown_str(self, md, has_frontmatter, title):
        frontmatter, _content = pyrs_yaml.read_markdown_str(md)
        if has_frontmatter:
            assert frontmatter is not None
            assert frontmatter["title"] == title
        else:
            assert frontmatter is None

    def test_read_markdown_file(self):
        test_file = str(Path(tempfile.gettempdir()) / "test_readme.md")
        with Path(test_file).open("w") as f:
            f.write("---\ntitle: Test\n---\nContent")
        try:
            frontmatter, _content = pyrs_yaml.read_markdown(test_file)
            assert frontmatter is not None
            assert frontmatter["title"] == "Test"
        finally:
            if Path(test_file).exists():
                Path(test_file).unlink()

    def test_read_markdown_names_its_argument_a_path(self, tmp_path):
        """The keyword a caller reaches for is the one that describes the argument.

        The Python layer renamed this parameter `content` while the function opens the file it is given, so
        `read_markdown(path=...)` raised `TypeError` and `read_markdown(content=...)` was accepted by the type
        checker and then failed on the filesystem. The pair below is the whole contract: the name that works is
        the one that is true, and the one that lies is gone.
        """
        document = tmp_path / "note.md"
        document.write_text("---\ntitle: Keyword\n---\nBody", encoding="utf-8")

        frontmatter, content = pyrs_yaml.read_markdown(path=str(document))
        assert frontmatter == {"title": "Keyword"}
        assert content == "Body"

        with pytest.raises(TypeError):
            pyrs_yaml.read_markdown(content="---\ntitle: x\n---\nbody")

    def test_read_markdown_str_names_its_argument_content(self, tmp_path):
        """The sibling really does take text, and says so."""
        document = tmp_path / "note.md"
        document.write_text("---\ntitle: Text\n---\nBody", encoding="utf-8")
        text = document.read_text(encoding="utf-8")

        frontmatter, _content = pyrs_yaml.read_markdown_str(content=text)
        assert frontmatter == {"title": "Text"}
        with pytest.raises(TypeError):
            pyrs_yaml.read_markdown_str(path=str(document))

    def test_from_dict_nested(self):
        data = {"app": {"name": "myapp", "version": "1.0"}, "features": ["auth", "logging"]}
        yaml_str = pyrs_yaml.from_dict(data)
        assert "app:" in yaml_str
        assert "name: myapp" in yaml_str
        assert "- auth" in yaml_str

    def test_from_json_nested(self):
        json_str = '{"database": {"host": "localhost", "port": 5432}}'
        yaml_str = pyrs_yaml.from_json(json_str)
        assert "database:" in yaml_str
        assert "host: localhost" in yaml_str
        assert "port: 5432" in yaml_str
