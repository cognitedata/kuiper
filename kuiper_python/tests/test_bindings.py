import json

import pytest

from kuiper import KuiperCompileError, compile_expression


@pytest.mark.parametrize(
    "expression,should_work",
    [
        ("input", True),
        ("fgkljh430943k", False),
        ("input.map(thing => thing.attr)", True),
        ("input2", False),
        ("input.map(thing => other_thing)", False),
    ],
)
def test_compile(expression: str, should_work: bool) -> None:
    try:
        compile_expression(expression, ["input"])
        worked = True
    except KuiperCompileError:
        worked = False

    assert should_work == worked


@pytest.mark.parametrize(
    "expression,input,expected_result",
    [
        ("input", {"hello": "there"}, {"hello": "there"}),
        ("input.map(i => i + 4)", [1, 2, 3, 4], [5, 6, 7, 8]),
    ],
)
def test_run(expression: str, input: dict, expected_result: dict) -> None:
    result = compile_expression(expression, ["input"]).run(json.dumps(input))
    assert json.loads(result) == expected_result
