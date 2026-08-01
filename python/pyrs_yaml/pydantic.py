from __future__ import annotations

from typing import Any, TypeVar

T = TypeVar("T")


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
