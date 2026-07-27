from collections.abc import Iterator
from typing import Any, Callable

class StreamIterator(Iterator[dict[str, Any]]):
    def __iter__(self) -> StreamIterator: ...
    def __next__(self) -> dict[str, Any] | None: ...

def parse_stream(
    yaml: str | bytes,
    resolve_merges: bool = ...,
    on_event: Callable[[dict[str, Any]], bool] | None = ...,
) -> StreamIterator | None: ...

__all__ = ["StreamIterator", "parse_stream"]
