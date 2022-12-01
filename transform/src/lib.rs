mod expressions;
mod lexer;
mod parse;
mod program;

pub use expressions::{TransformError, TransformErrorData};
pub use parse::{Parser, ParserError, ParserErrorData};
pub use program::{CompileError, ConfigCompileError, ParserCompileError, Program, TransformInput};

#[cfg(test)]
mod tests {
    use logos::Span;
    use serde_json::{json, Value};

    use crate::{CompileError, ParserError, Program, TransformError};

    fn compile(value: Value) -> Result<Program, CompileError> {
        Program::compile(serde_json::from_value(value).unwrap())
    }

    fn compile_err(value: Value) -> CompileError {
        match compile(value) {
            Ok(_) => panic!("Expected compilation to fail"),
            Err(x) => x,
        }
    }

    #[test]
    pub fn test_merge() {
        let program = compile(json!([
            {
                "id": "gen",
                "inputs": [],
                "transform": "[1, 2]",
                "type": "flatten"
            }, {
                "id": "parse",
                "inputs": ["input"],
                "transform": "$input.values",
                "type": "flatten"
            }, {
                "id": "finmerge",
                "inputs": ["gen", "parse"],
                "transform": {
                    "val": "$merge",
                },
                "type": "map",
                "mode": "merge"
            }
        ]))
        .unwrap();
        let input = json!({
            "values": [3, 4, 5]
        });
        let res = program.execute(&input).unwrap();
        assert_eq!(res.len(), 5);
        let mut vals: Vec<_> = res
            .into_iter()
            .map(|e| e.get("val").unwrap().as_u64().unwrap())
            .collect();
        // No guarantee of ordering
        vals.sort();
        for i in 0..5 {
            assert_eq!(*vals.get(i).unwrap(), (i + 1) as u64);
        }
    }

    #[test]
    pub fn test_zip() {
        let program = compile(json!([
            {
                "id": "gen",
                "inputs": [],
                "transform": "[1, 2]",
                "type": "flatten"
            }, {
                "id": "parse",
                "inputs": ["input"],
                "transform": "$input.values",
                "type": "flatten"
            }, {
                "id": "finmerge",
                "inputs": ["gen", "parse"],
                "transform": {
                    "gen": "$gen",
                    "parse": "$parse"
                },
                "type": "map",
                "mode": "zip"
            }
        ]))
        .unwrap();
        let input = json!({
            "values": [3, 4, 5]
        });
        let res = program.execute(&input).unwrap();
        assert_eq!(res.len(), 3);
        let val = res.get(0).unwrap();
        assert_eq!(val.get("gen").unwrap(), 1);
        assert_eq!(val.get("parse").unwrap(), 3);
        let val = res.get(1).unwrap();
        assert_eq!(val.get("gen").unwrap(), 2);
        assert_eq!(val.get("parse").unwrap(), 4);
        let val = res.get(2).unwrap();
        assert_eq!(val.get("gen").unwrap(), &Value::Null);
        assert_eq!(val.get("parse").unwrap(), 5);
    }

    #[test]
    pub fn test_exponential_flatten() {
        let program = compile(json!([
            {
                "id": "step1",
                "inputs": ["input"],
                "transform": "$input.values",
                "type": "flatten"
            },
            {
                "id": "gen",
                "inputs": [],
                "transform": "[0, 1, 2, 3, 4]",
                "type": "flatten"
            },
            {
                "id": "explode1",
                "inputs": ["gen", "step1"],
                "transform": {
                    "v1": "$gen",
                    "v2": "$step1.value"
                },
                "type": "map"
            },
            {
                "id": "explode2",
                "inputs": ["gen", "explode1"],
                "transform": {
                    "v1": "$gen",
                    "v21": "$explode1.v1",
                    "v22": "$explode1.v2"
                },
                "type": "map"
            }
        ]))
        .unwrap();
        let input = json!({
            "id": "my-id",
            "values": [{
                "value": 123.123,
                "time": 123142812824u64
            }, {
                "value": 321.321,
                "time": 123901591231u64
            }]
        });
        let res = program.execute(&input).unwrap();
        assert_eq!(res.len(), 50);
        for rs in res {
            println!("{}", rs);
        }
    }

    // Compile errors
    #[test]
    pub fn test_parser_error() {
        let err = compile_err(json!([
            {
                "id": "step1",
                "inputs": ["input"],
                "transform": "pow($input.test)",
                "type": "flatten"
            }
        ]));
        match err {
            CompileError::Parser(d) => {
                match &d.err {
                    ParserError::NFunctionArgs(d) => {
                        assert_eq!(
                            d.detail,
                            Some(
                                "Incorrect number of function args: function pow takes 2 arguments"
                                    .to_string()
                            )
                        );
                        assert_eq!(d.position, Span { start: 0, end: 16 });
                    }
                    _ => panic!("Wrong type of parser error {:?}", &d.err),
                }
                assert_eq!(d.field, None);
                assert_eq!(d.id, "step1");
            }
            _ => panic!("Wrong type of error {:?}", err),
        }
    }

    #[test]
    pub fn test_parser_error_map() {
        let err = compile_err(json!([{
            "id": "step1",
            "inputs": ["input"],
            "transform": {
                "f1": "pow($input.test)"
            },
            "type": "map"
        }]));
        match err {
            CompileError::Parser(d) => {
                match &d.err {
                    ParserError::NFunctionArgs(d) => {
                        assert_eq!(
                            d.detail,
                            Some(
                                "Incorrect number of function args: function pow takes 2 arguments"
                                    .to_string()
                            )
                        );
                        assert_eq!(d.position, Span { start: 0, end: 16 });
                    }
                    _ => panic!("Wrong type of parser error {:?}", &d.err),
                }
                assert_eq!(d.field, Some("f1".to_string()));
                assert_eq!(d.id, "step1");
            }
            _ => panic!("Wrong type of error {:?}", err),
        }
    }

    #[test]
    pub fn test_illegal_id_input() {
        let err = compile_err(json!([{
            "id": "input",
            "inputs": ["input"],
            "transform": {
                "f1": "$input.test"
            },
            "type": "map"
        }]));
        match err {
            CompileError::Config(d) => {
                assert_eq!(d.id, Some("input".to_string()));
                assert_eq!(d.desc, "Transform ID may not be \"input\" or \"merge\". They are reserved for special inputs to the pipeline")
            }
            _ => panic!("Wrong type of error {:?}", err),
        }
    }

    #[test]
    pub fn test_illegal_id_merge() {
        let err = compile_err(json!([{
            "id": "merge",
            "inputs": ["input"],
            "transform": {
                "f1": "$input.test"
            },
            "type": "map"
        }]));
        match err {
            CompileError::Config(d) => {
                assert_eq!(d.id, Some("merge".to_string()));
                assert_eq!(d.desc, "Transform ID may not be \"input\" or \"merge\". They are reserved for special inputs to the pipeline")
            }
            _ => panic!("Wrong type of error {:?}", err),
        }
    }

    #[test]
    pub fn test_immediate_recursion() {
        let err = compile_err(json!([{
            "id": "step",
            "inputs": ["input", "step"],
            "transform": {
                "f1": "$step.test"
            },
            "type": "map"
        }]));
        match err {
            CompileError::Config(d) => {
                assert_eq!(d.id, Some("step".to_string()));
                assert_eq!(
                    d.desc,
                    "Recursive transformations are not allowed, step indirectly references itself"
                )
            }
            _ => panic!("Wrong type of error {:?}", err),
        }
    }

    #[test]
    pub fn test_indirect_recursion() {
        let err = compile_err(json!([{
            "id": "step",
            "inputs": ["input", "step2"],
            "transform": {
                "f1": "$step2.test"
            },
            "type": "map"
        }, {
            "id": "step2",
            "inputs": ["step"],
            "transform": {
                "f1": "$step.test"
            },
            "type": "map"
        }]));
        match err {
            CompileError::Config(d) => {
                assert_eq!(d.id, Some("step2".to_string()));
                assert_eq!(
                    d.desc,
                    "Recursive transformations are not allowed, step2 indirectly references itself"
                )
            }
            _ => panic!("Wrong type of error {:?}", err),
        }
    }

    #[test]
    pub fn test_missing_input() {
        let err = compile_err(json!([{
            "id": "step",
            "inputs": ["input", "step2"],
            "transform": {
                "f1": "$step2.test"
            },
            "type": "map"
        }]));
        match err {
            CompileError::Config(d) => {
                assert_eq!(d.id, Some("step".to_string()));
                assert_eq!(d.desc, "Input step2 to step is not defined")
            }
            _ => panic!("Wrong type of error {:?}", err),
        }
    }

    // Numbers
    #[test]
    pub fn test_add_different_types() {
        let result = compile(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "$input.val + 5.5",
            "type": "flatten"
        }]))
        .unwrap();
        let res = result.execute(&json!({ "val": 5 })).unwrap();
        let res = res.get(0).unwrap();
        assert_eq!(10.5, res.as_f64().unwrap());
    }

    #[test]
    pub fn test_add_keeps_type() {
        let result = compile(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "$input.val + 5",
            "type": "flatten"
        }]))
        .unwrap();
        let res = result.execute(&json!({ "val": 5 })).unwrap();
        let res = res.get(0).unwrap();
        assert_eq!(10, res.as_u64().unwrap());
    }

    #[test]
    pub fn test_negative_result() {
        let result = compile(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "$input.val - 10",
            "type": "flatten"
        }]))
        .unwrap();
        let res = result.execute(&json!({ "val": 5 })).unwrap();
        let res = res.get(0).unwrap();
        assert_eq!(-5, res.as_i64().unwrap());
    }

    #[test]
    pub fn test_divide_by_zero() {
        let result = compile(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "10 / $input.val",
            "type": "flatten"
        }]))
        .unwrap();
        let res = result.execute(&json!({ "val": 0 })).unwrap_err();
        match res {
            TransformError::InvalidOperation(d) => {
                assert_eq!(d.id, "step");
                assert_eq!(d.desc, "Divide by zero");
                assert_eq!(d.span, Span { start: 3, end: 4 });
            }
            _ => panic!("Wrong type of error {:?}", res),
        }
    }

    #[test]
    pub fn test_non_numeric_input() {
        let result = compile(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "10 * $input.val",
            "type": "flatten"
        }]))
        .unwrap();
        let res = result.execute(&json!({ "val": "test" })).unwrap_err();
        match res {
            TransformError::IncorrectTypeInField(d) => {
                assert_eq!(d.id, "step");
                assert_eq!(d.desc, "'*'. Got string, expected number");
                assert_eq!(d.span, Span { start: 3, end: 4 });
            }
            _ => panic!("Wrong type of error {:?}", res),
        }
    }

    #[test]
    pub fn test_wrong_function_input() {
        let result = compile(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "pow(10, $input.val)",
            "type": "flatten"
        }]))
        .unwrap();
        let res = result.execute(&json!({ "val": "test" })).unwrap_err();
        match res {
            TransformError::IncorrectTypeInField(d) => {
                assert_eq!(d.id, "step");
                assert_eq!(d.desc, "pow argument 2. Got string, expected number");
                assert_eq!(d.span, Span { start: 0, end: 19 });
            }
            _ => panic!("Wrong type of error {:?}", res),
        }
    }

    #[test]
    pub fn test_source_missing_error() {
        let result = compile(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "pow(10, $foo.val)",
            "type": "flatten"
        }]))
        .unwrap();
        let res = result.execute(&json!({ "val": "test" })).unwrap_err();
        match res {
            TransformError::SourceMissingError(d) => {
                assert_eq!(d.id, "step");
                assert_eq!(d.desc, "foo");
                assert_eq!(d.span, Span { start: 8, end: 17 });
            }
            _ => panic!("Wrong type of error {:?}", res),
        }
    }

    #[test]
    pub fn test_wrong_source_selector() {
        let result = compile(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "$[0]",
            "type": "flatten"
        }]))
        .unwrap();
        let res = result.execute(&json!({ "val": "test" })).unwrap_err();
        match res {
            TransformError::InvalidOperation(d) => {
                assert_eq!(d.id, "step");
                assert_eq!(d.desc, "Root selector must be string");
                assert_eq!(d.span, Span { start: 0, end: 4 });
            }
            _ => panic!("Wrong type of error {:?}", res),
        }
    }
}
