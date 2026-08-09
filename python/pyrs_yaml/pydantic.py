from __future__ import annotations

from typing import Any, TypeVar

T = TypeVar("T")


def dump_pydantic(model: Any) -> str:
    """Serialize a Pydantic model to a YAML string.

    Args:
        model: A Pydantic BaseModel instance.

    Returns:
        The YAML string representation of the model.

    Raises:
        ImportError: If pydantic is not installed.
        TypeError: If model is not a Pydantic BaseModel instance.
    """
    try:
        from pydantic import BaseModel
    except ImportError:
        raise ImportError("pydantic is required for dump_pydantic. Install with: uv add pydantic") from None

    if not isinstance(model, BaseModel):
        raise TypeError("model must be a Pydantic BaseModel instance")

    from .pyrs_yaml import safe_dump

    data = model.model_dump(mode="json")
    return safe_dump(data)


def parse_as(model: type[T], src: str, **yaml_kwargs: Any) -> T:
    """Parse YAML string and validate against a Pydantic model.

    Args:
        model: A Pydantic BaseModel subclass.
        src: YAML string to parse.
        **yaml_kwargs: Keyword arguments passed to YAML() constructor.

    Returns:
        An instance of the given model.

    Raises:
        ImportError: If pydantic is not installed.
    """
    try:
        from pydantic import BaseModel
    except ImportError:
        raise ImportError("pydantic is required for parse_as. Install with: uv add pydantic") from None

    if not issubclass(model, BaseModel):
        raise TypeError("model must be a Pydantic BaseModel subclass")

    from .pyrs_yaml import YAML

    data = YAML(**yaml_kwargs).safe_load(src)
    return model.model_validate(data)
