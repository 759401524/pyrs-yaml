import pyrs_yaml
import pytest

from tests.data import yaml_samples as yaml

pytest.importorskip("pydantic", reason="pydantic not installed")


class TestPydantic:
    """Test Pydantic integration (Phase 5 of v0.9.0)."""

    def test_parse_as_returns_model_instance(self):
        from pydantic import BaseModel

        class UserModel(BaseModel):
            name: str
            age: int

        result = pyrs_yaml.parse_as(UserModel, yaml.USER_MODEL)
        assert isinstance(result, UserModel)
        assert result.name == "Alice"
        assert result.age == 30

    def test_parse_as_with_kwargs(self):
        from pydantic import BaseModel

        class Config(BaseModel):
            name: str

        yaml_str = "name: first\nname: second"
        result = pyrs_yaml.parse_as(Config, yaml_str, allow_duplicate_keys=True)
        assert result.name == "second"

    def test_parse_as_no_pydantic(self, monkeypatch):
        import sys

        monkeypatch.setitem(sys.modules, "pydantic", None)
        with pytest.raises(ImportError, match="pydantic is required"):
            pyrs_yaml.parse_as(dict, "key: value")

    def test_parse_as_validation_error(self):
        from pydantic import BaseModel, ValidationError

        class UserModel(BaseModel):
            name: str
            age: int

        with pytest.raises(ValidationError):
            pyrs_yaml.parse_as(UserModel, yaml.USER_MODEL_INVALID)

    def test_parse_as_non_base_model(self):
        with pytest.raises(TypeError, match="BaseModel"):
            pyrs_yaml.parse_as(dict, "key: value")
