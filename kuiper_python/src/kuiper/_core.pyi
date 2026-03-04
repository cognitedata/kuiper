from collections.abc import Callable
from typing import Any

class KuiperExpression:
    def run(self, input: str) -> str: ...
    def run_multiple_inputs(self, inputs: list[str]) -> str: ...

class CustomFunction:
    def __init__(self, name: str, target: Callable[..., Any]) -> None: ...

def compile_expression(
    expression: str,
    inputs: list[str],
    optimizer_operation_limit: int = 100000,
    max_macro_expansions: int = 20,
    custom_functions: list[CustomFunction] | None = None,
) -> KuiperExpression: ...
