"""Tests for the INI exchange spoke (load_ini)."""

import pytest

import pyrs_yaml


class TestLoadIni:
    def test_sections_and_case(self):
        d = pyrs_yaml.load_ini("[server]\nHost = localhost\nPort = 8080\n\n[db]\nurl = sqlite://\n")
        assert d["server"] == {"Host": "localhost", "Port": "8080"}
        assert d["db"] == {"url": "sqlite://"}

    def test_comments_and_multiline(self):
        d = pyrs_yaml.load_ini("; comment\n# another\n[sec]\nkey = v1\n  continued\nother = x\n")
        assert d["sec"]["key"] == "v1\ncontinued"
        assert d["sec"]["other"] == "x"

    def test_default_section_surfaces(self):
        # configparser requires an explicit [DEFAULT] header; bare leading
        # key=value lines are rejected (documented INI strictness).
        d = pyrs_yaml.load_ini("[DEFAULT]\nshared = 1\n\n[s]\nk = v\n")
        assert d["DEFAULT"] == {"shared": "1"}
        assert d["s"]["shared"] == "1"  # configparser inheritance
        assert d["s"]["k"] == "v"

    def test_leading_keyless_section_rejected(self):
        with pytest.raises(ValueError):
            pyrs_yaml.load_ini("shared = 1\n[s]\nk = v\n")

    def test_duplicate_key_raises(self):
        with pytest.raises(ValueError):
            pyrs_yaml.load_ini("[s]\nk = 1\nk = 2\n")

    def test_malformed_raises(self):
        with pytest.raises(ValueError):
            pyrs_yaml.load_ini("no section header = 1\n[oops\n")

    def test_empty_gives_empty_dict(self):
        assert pyrs_yaml.load_ini("") == {}

    def test_convertible_to_yaml(self):
        d = pyrs_yaml.load_ini("[s]\na = 1\n")
        yaml_text = pyrs_yaml.safe_dump(d)
        assert pyrs_yaml.safe_load(yaml_text) == {"s": {"a": "1"}}
