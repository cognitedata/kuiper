class KuiperExpression:
    def run(self, input: str) -> str: ...
    def run_multiple_inputs(self, inputs: list[str]) -> str: ...

def compile_expression(
    expression: str,
    inputs: list[str],
    optimizer_operation_limit: int = 100000,
    max_macro_expansions: int = 20,
) -> KuiperExpression: ...
