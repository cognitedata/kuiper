import json

import pytest

from kuiper import JsonType, KuiperCompileError, compile_expression, CustomFunction


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


test_cases: list[tuple[str, JsonType, JsonType]] = [
    ("input", {"hello": "there"}, {"hello": "there"}),
    ("input.map(i => i + 4)", [1, 2, 3, 4], [5, 6, 7, 8]),
]


@pytest.mark.parametrize("expression,input,expected_result", test_cases)
def test_run_json(expression: str, input: JsonType, expected_result: JsonType) -> None:
    result = compile_expression(expression, ["input"]).run_json(json.dumps(input))
    assert json.loads(result) == expected_result


@pytest.mark.parametrize("expression,input,expected_result", test_cases)
def test_run_values(expression: str, input: JsonType, expected_result: JsonType) -> None:
    result = compile_expression(expression, ["input"]).run(input)
    assert result == expected_result


def simple_target() -> int:
    return 42


def with_args(x: int, y: int) -> JsonType:
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

    result = exp.run({"num": 5})
    assert result == {"simple": 42, "with_args": [5, 1, 6, {"a": 1, "b": {"c": [1, 2, 3]}}]}
