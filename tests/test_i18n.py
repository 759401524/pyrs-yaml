"""Tests for i18n language switching and error messages."""

import pytest

import pyrs_yaml


class TestSetLanguage:
    def test_set_language_en(self):
        pyrs_yaml.set_language("en")
        assert pyrs_yaml.get_language() == "en"

    def test_set_language_zh(self):
        pyrs_yaml.set_language("zh-CN")
        assert pyrs_yaml.get_language() == "zh-CN"

    def test_set_language_ja(self):
        pyrs_yaml.set_language("ja-JP")
        assert pyrs_yaml.get_language() == "ja-JP"

    def test_set_language_ko(self):
        pyrs_yaml.set_language("ko-KR")
        assert pyrs_yaml.get_language() == "ko-KR"

    def test_set_language_unsupported_raises(self):
        with pytest.raises(ValueError):
            pyrs_yaml.set_language("xx-YY")

    def test_set_language_invalid_default_raises(self):
        with pytest.raises(ValueError):
            pyrs_yaml.set_language("")


class TestListLanguages:
    def test_list_languages_returns_all(self):
        langs = pyrs_yaml.list_languages()
        assert langs == ["en", "zh-CN", "ja-JP", "ko-KR"]

    def test_list_languages_is_list(self):
        langs = pyrs_yaml.list_languages()
        assert isinstance(langs, list)
        assert all(isinstance(lang, str) for lang in langs)


class TestErrorMessages:
    """Parse errors always use English detail text (from the Rust parser).
    Only wrapper templates are translated (e.g. duplicate-key).
    """

    def test_en_parse_error_contains_english(self):
        pyrs_yaml.set_language("en")
        with pytest.raises(pyrs_yaml.YamlParseError) as exc_info:
            pyrs_yaml.parse("key: [unclosed")
        msg = str(exc_info.value)
        assert "parse error" in msg.lower()

    def test_zh_parse_error_same_detail(self):
        """Parse error detail is always in English (Rust parser output)."""
        pyrs_yaml.set_language("zh-CN")
        with pytest.raises(pyrs_yaml.YamlParseError) as exc_info:
            pyrs_yaml.parse("key: [unclosed")
        msg = str(exc_info.value)
        assert "parse error" in msg.lower()

    def test_duplicate_key_error_is_translated(self):
        pyrs_yaml.set_language("en")
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError) as exc_en:
            pyrs_yaml.parse("key: 1\nkey: 2")
        msg_en = str(exc_en.value)

        pyrs_yaml.set_language("zh-CN")
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError) as exc_zh:
            pyrs_yaml.parse("key: 1\nkey: 2")
        msg_zh = str(exc_zh.value)

        assert msg_en == "duplicate key: key"
        assert msg_zh == "重复的键：key"  # noqa: RUF001
        assert msg_en != msg_zh

    def test_duplicate_key_error_ja(self):
        pyrs_yaml.set_language("ja-JP")
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError) as exc_info:
            pyrs_yaml.parse("key: 1\nkey: 2")
        msg = str(exc_info.value)
        assert "key" in msg

    def test_duplicate_key_error_ko(self):
        pyrs_yaml.set_language("ko-KR")
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError) as exc_info:
            pyrs_yaml.parse("key: 1\nkey: 2")
        msg = str(exc_info.value)
        assert "key" in msg


class TestNegotiateLanguage:
    def test_exact_match(self):
        result = pyrs_yaml.negotiate_language(["zh-CN", "en"], "en")
        assert result == "zh-CN"

    def test_prefix_match(self):
        result = pyrs_yaml.negotiate_language(["zh-TW", "en"], "en")
        assert result == "zh-CN"

    def test_fallback_to_default(self):
        result = pyrs_yaml.negotiate_language(["xx-YY"], "en")
        assert result == "en"

    def test_empty_list_fallback(self):
        result = pyrs_yaml.negotiate_language([], "ja-JP")
        assert result == "ja-JP"

    def test_invalid_default_falls_back_to_en(self):
        """Invalid default falls back to 'en', does not raise."""
        result = pyrs_yaml.negotiate_language(["en"], "xx")
        assert result == "en"


class TestDetectLanguage:
    def test_detect_language_returns_str(self):
        result = pyrs_yaml.detect_language()
        assert isinstance(result, str)
        assert result in ["en", "zh-CN", "ja-JP", "ko-KR"]
