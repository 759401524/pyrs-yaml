import pytest

import pyrs_yaml

pytest.importorskip("pydantic_settings", reason="pydantic-settings not installed")
pytest.importorskip("pydantic", reason="pydantic not installed")


def _yaml_source(settings_cls, **kwargs):
    return (pyrs_yaml.PyrsYamlConfigSettingsSource(settings_cls, **kwargs),)


class TestPyrsYamlConfigSettingsSource:
    """Test pydantic-settings integration (PyrsYamlConfigSettingsSource)."""

    def test_basic_load_from_yaml_file(self, tmp_path):
        from pydantic import BaseModel
        from pydantic_settings import BaseSettings, SettingsConfigDict

        cfg = tmp_path / "config.yaml"
        cfg.write_text("app_name: my-service\nnested:\n  host: localhost\n  port: 5432\n")

        class Nested(BaseModel):
            host: str
            port: int

        class Settings(BaseSettings):
            app_name: str
            nested: Nested

            model_config = SettingsConfigDict(yaml_file=str(cfg))

            @classmethod
            def settings_customise_sources(
                cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
            ):
                return (
                    init_settings,
                    env_settings,
                    dotenv_settings,
                    file_secret_settings,
                    _yaml_source(settings_cls)[0],
                )

        settings = Settings()
        assert settings.app_name == "my-service"
        assert settings.nested.host == "localhost"
        assert settings.nested.port == 5432

    def test_yaml_config_section(self, tmp_path):
        from pydantic_settings import BaseSettings, SettingsConfigDict

        cfg = tmp_path / "config.yaml"
        cfg.write_text("app:\n  settings:\n    api_key: secret\n    timeout: 30\n")

        class Settings(BaseSettings):
            api_key: str
            timeout: int

            model_config = SettingsConfigDict(yaml_file=str(cfg), yaml_config_section="app.settings")

            @classmethod
            def settings_customise_sources(
                cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
            ):
                return (
                    init_settings,
                    env_settings,
                    dotenv_settings,
                    file_secret_settings,
                    _yaml_source(settings_cls)[0],
                )

        settings = Settings()
        assert settings.api_key == "secret"
        assert settings.timeout == 30

    def test_yaml_config_section_top_level_key(self, tmp_path):
        from pydantic_settings import BaseSettings, SettingsConfigDict

        cfg = tmp_path / "config.yaml"
        cfg.write_text("app:\n  api_key: secret\n  timeout: 30\n")

        class Settings(BaseSettings):
            api_key: str
            timeout: int

            model_config = SettingsConfigDict(yaml_file=str(cfg), yaml_config_section="app")

            @classmethod
            def settings_customise_sources(
                cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
            ):
                return (
                    init_settings,
                    env_settings,
                    dotenv_settings,
                    file_secret_settings,
                    _yaml_source(settings_cls)[0],
                )

        settings = Settings()
        assert settings.api_key == "secret"
        assert settings.timeout == 30

    def test_yaml_config_section_not_mapping_raises(self, tmp_path):
        from pydantic_settings import BaseSettings, SettingsConfigDict

        cfg = tmp_path / "config.yaml"
        cfg.write_text("app: just-a-string\n")

        class Settings(BaseSettings):
            api_key: str

            model_config = SettingsConfigDict(yaml_file=str(cfg), yaml_config_section="app")

            @classmethod
            def settings_customise_sources(
                cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
            ):
                return (
                    init_settings,
                    env_settings,
                    dotenv_settings,
                    file_secret_settings,
                    _yaml_source(settings_cls)[0],
                )

        with pytest.raises(TypeError, match="must be a mapping"):
            Settings()

    def test_env_var_overrides_yaml(self, tmp_path, monkeypatch):
        from pydantic_settings import BaseSettings, SettingsConfigDict

        cfg = tmp_path / "config.yaml"
        cfg.write_text("app_name: from-yaml\n")

        monkeypatch.setenv("APP_NAME", "from-env")

        class Settings(BaseSettings):
            app_name: str

            model_config = SettingsConfigDict(yaml_file=str(cfg))

            @classmethod
            def settings_customise_sources(
                cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
            ):
                return (
                    init_settings,
                    env_settings,
                    dotenv_settings,
                    file_secret_settings,
                    _yaml_source(settings_cls)[0],
                )

        settings = Settings()
        assert settings.app_name == "from-env"

    def test_deep_merge(self, tmp_path):
        from pydantic import BaseModel
        from pydantic_settings import BaseSettings

        default = tmp_path / "default.yaml"
        custom = tmp_path / "custom.yaml"
        default.write_text("nested:\n  foo: 1\n  bar: 2\n")
        custom.write_text("nested:\n  foo: 3\n")

        class Nested(BaseModel):
            foo: int
            bar: int = 0

        class Settings(BaseSettings):
            nested: Nested

            @classmethod
            def settings_customise_sources(
                cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
            ):
                return (
                    init_settings,
                    env_settings,
                    dotenv_settings,
                    file_secret_settings,
                    _yaml_source(settings_cls, yaml_file=[str(default), str(custom)], deep_merge=True)[0],
                )

        settings = Settings()
        assert settings.nested.foo == 3
        assert settings.nested.bar == 2

    def test_deep_merge_false_overrides(self, tmp_path):
        from pydantic import BaseModel
        from pydantic_settings import BaseSettings

        default = tmp_path / "default.yaml"
        custom = tmp_path / "custom.yaml"
        default.write_text("nested:\n  foo: 1\n  bar: 2\n")
        custom.write_text("nested:\n  foo: 3\n")

        class Nested(BaseModel):
            foo: int
            bar: int = 0

        class Settings(BaseSettings):
            nested: Nested

            @classmethod
            def settings_customise_sources(
                cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
            ):
                return (
                    init_settings,
                    env_settings,
                    dotenv_settings,
                    file_secret_settings,
                    _yaml_source(settings_cls, yaml_file=[str(default), str(custom)], deep_merge=False)[0],
                )

        settings = Settings()
        assert settings.nested.foo == 3
        assert settings.nested.bar == 0

    def test_yaml_file_encoding(self, tmp_path):
        from pydantic_settings import BaseSettings, SettingsConfigDict

        cfg = tmp_path / "config.yaml"
        cfg.write_text("app_name: サービス\n", encoding="utf-8")

        class Settings(BaseSettings):
            app_name: str

            model_config = SettingsConfigDict(yaml_file=str(cfg), yaml_file_encoding="utf-8")

            @classmethod
            def settings_customise_sources(
                cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
            ):
                return (
                    init_settings,
                    env_settings,
                    dotenv_settings,
                    file_secret_settings,
                    _yaml_source(settings_cls)[0],
                )

        settings = Settings()
        assert settings.app_name == "サービス"

    def test_yaml_12_scalars_stay_strings(self, tmp_path):
        """YAML 1.2 core schema: 'on'/'off' are strings, not bools (vs PyYAML 1.1)."""
        from pydantic_settings import BaseSettings, SettingsConfigDict

        cfg = tmp_path / "config.yaml"
        cfg.write_text("feature_flag: on\n")

        class Settings(BaseSettings):
            feature_flag: str

            model_config = SettingsConfigDict(yaml_file=str(cfg))

            @classmethod
            def settings_customise_sources(
                cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
            ):
                return (
                    init_settings,
                    env_settings,
                    dotenv_settings,
                    file_secret_settings,
                    _yaml_source(settings_cls)[0],
                )

        settings = Settings()
        assert settings.feature_flag == "on"

    def test_drop_in_parity_with_yaml_settings_source(self, tmp_path):
        """Same YAML via PyYAML-based source and pyrs-yaml source give equal results."""
        from pydantic_settings import BaseSettings, SettingsConfigDict, YamlConfigSettingsSource

        cfg = tmp_path / "config.yaml"
        cfg.write_text("app_name: my-service\ncount: 42\n")

        class PyYAMLSettings(BaseSettings):
            app_name: str
            count: int

            model_config = SettingsConfigDict(yaml_file=str(cfg))

            @classmethod
            def settings_customise_sources(
                cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
            ):
                return (
                    init_settings,
                    env_settings,
                    dotenv_settings,
                    file_secret_settings,
                    YamlConfigSettingsSource(settings_cls),
                )

        class PyrsYAMLSettings(BaseSettings):
            app_name: str
            count: int

            model_config = SettingsConfigDict(yaml_file=str(cfg))

            @classmethod
            def settings_customise_sources(
                cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
            ):
                return (
                    init_settings,
                    env_settings,
                    dotenv_settings,
                    file_secret_settings,
                    _yaml_source(settings_cls)[0],
                )

        assert (
            PyYAMLSettings().model_dump() == PyrsYAMLSettings().model_dump() == {"app_name": "my-service", "count": 42}
        )

    def test_missing_pydantic_settings_raises_helpful_error(self, monkeypatch):
        import sys

        from pyrs_yaml import settings as settings_module

        settings_module._settings_source_class.cache_clear()
        monkeypatch.setitem(sys.modules, "pydantic_settings", None)
        with pytest.raises(ImportError, match="pydantic-settings is required"):
            hasattr(pyrs_yaml, "PyrsYamlConfigSettingsSource")

    def test_missing_pydantic_settings_class_import(self, monkeypatch):
        import sys

        from pyrs_yaml import settings as settings_module

        settings_module._settings_source_class.cache_clear()
        monkeypatch.setitem(sys.modules, "pydantic_settings", None)
        with pytest.raises(ImportError, match="pydantic-settings is required"):
            hasattr(settings_module, "PyrsYamlConfigSettingsSource")

    def test_dump_pydantic_works_with_settings(self, tmp_path):
        """dump_pydantic round-trips a BaseSettings instance like any BaseModel."""
        from pydantic_settings import BaseSettings, SettingsConfigDict

        cfg = tmp_path / "config.yaml"
        cfg.write_text("app_name: my-service\n")

        class Settings(BaseSettings):
            app_name: str

            model_config = SettingsConfigDict(yaml_file=str(cfg))

            @classmethod
            def settings_customise_sources(
                cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
            ):
                return (
                    init_settings,
                    env_settings,
                    dotenv_settings,
                    file_secret_settings,
                    _yaml_source(settings_cls)[0],
                )

        settings = Settings()
        yaml_str = pyrs_yaml.dump_pydantic(settings)
        result = pyrs_yaml.parse_as(Settings, yaml_str)
        assert result.app_name == "my-service"
