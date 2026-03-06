from collections.abc import Callable
from typing import Any

type JsonType = str | int | float | bool | None | list["JsonType"] | dict[str, "JsonType"]

class KuiperExpression:
    def run(self, *inputs: JsonType, max_operations: int | None = None) -> JsonType: ...
    def run_json(self, *inputs: str, max_operations: int | None = None) -> str: ...

class CustomFunction:
    def __init__(self, name: str, target: Callable[..., Any]) -> None: ...

def compile_expression(
    expression: str,
    inputs: list[str],
    optimizer_operation_limit: int = 100000,
    max_macro_expansions: int = 20,
    custom_functions: list[CustomFunction] | None = None,
) -> KuiperExpression: ...
