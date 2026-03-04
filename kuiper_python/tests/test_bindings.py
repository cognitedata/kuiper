import json
from typing import Any

import pytest

from kuiper import KuiperCompileError, compile_expression, CustomFunction


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


def simple_target():
    return 42


def with_args(x: Any, y: Any) -> Any:
    #  Include some nestedness in the response to test the recursiveness of the conversions
    return [x, y, x + y, {"a": 1, "b": {"c": [1, 2, 3]}}]


def test_custom_function() -> None:
    exp = compile_expression(
        '{"simple": simple_target(), "with_args": with_args(input.num, 1)}',
        ["input"],
        custom_functions=[
            CustomFunction("simple_target", simple_target),
            CustomFunction("with_args", with_args),
        ],
    )

    result = exp.run('{"num": 5}')
    assert json.loads(result) == {"simple": 42, "with_args": [5, 1, 6, {"a": 1, "b": {"c": [1, 2, 3]}}]}
