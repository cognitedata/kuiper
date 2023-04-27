mod expressions;
mod lexer;
mod parse;
mod program;

pub use expressions::{TransformError, TransformErrorData};
pub use parse::{Parser, ParserError, ParserErrorData};
pub use program::{CompileError, ConfigCompileError, ParserCompileError, Program, TransformInput};

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use logos::Span;
    use serde_json::{json, Value};

    use crate::{
        program::OptimizerCompileError, CompileError, ParserError, Program, TransformError,
    };

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
                "expandOutput": true
            }, {
                "id": "parse",
                "inputs": ["input"],
                "transform": "$input.values",
                "expandOutput": true
            }, {
                "id": "finmerge",
                "inputs": ["gen", "parse"],
                "transform": r#"{
                    "val": $merge
                }"#,
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
                "expandOutput": true
            }, {
                "id": "parse",
                "inputs": ["input"],
                "transform": "$input.values",
                "expandOutput": true
            }, {
                "id": "finmerge",
                "inputs": ["gen", "parse"],
                "transform": r#"{
                    "gen": $gen,
                    "parse": $parse
                }"#,
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
                "expandOutput": true
            },
            {
                "id": "gen",
                "inputs": [],
                "transform": "[0, 1, 2, 3, 4]",
                "expandOutput": true
            },
            {
                "id": "explode1",
                "inputs": ["gen", "step1"],
                "transform": r#"{
                    "v1": $gen,
                    "v2": $step1.value
                }"#
            },
            {
                "id": "explode2",
                "inputs": ["gen", "explode1"],
                "transform": r#"{
                    "v1": $gen,
                    "v21": $explode1.v1,
                    "v22": $explode1.v2
                }"#
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
            println!("{rs}");
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
                "expandOutput": true
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
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    pub fn test_parser_error_map() {
        let err = compile_err(json!([{
            "id": "step1",
            "inputs": ["input"],
            "transform": r#"{
                "f1": pow($input.test)
            }"#
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
                        assert_eq!(d.position, Span { start: 24, end: 40 });
                    }
                    _ => panic!("Wrong type of parser error {:?}", &d.err),
                }
                assert_eq!(d.id, "step1");
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    pub fn test_illegal_id_input() {
        let err = compile_err(json!([{
            "id": "input",
            "inputs": ["input"],
            "transform": r#"{
                "f1": $input.test
            }"#
        }]));
        match err {
            CompileError::Config(d) => {
                assert_eq!(d.id, Some("input".to_string()));
                assert_eq!(d.desc, "Transform ID may not start with \"input\" or be equal to \"merge\". They are reserved for special inputs to the pipeline")
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    pub fn test_illegal_id_merge() {
        let err = compile_err(json!([{
            "id": "merge",
            "inputs": ["input"],
            "transform": r#"{
                "f1": $input.test
            }"#
        }]));
        match err {
            CompileError::Config(d) => {
                assert_eq!(d.id, Some("merge".to_string()));
                assert_eq!(d.desc, "Transform ID may not start with \"input\" or be equal to \"merge\". They are reserved for special inputs to the pipeline")
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    pub fn test_immediate_recursion() {
        let err = compile_err(json!([{
            "id": "step",
            "inputs": ["input", "step"],
            "transform": r#"{
                "f1": $step.test
            }"#
        }]));
        match err {
            CompileError::Config(d) => {
                assert_eq!(d.id, Some("step".to_string()));
                assert_eq!(
                    d.desc,
                    "Recursive transformations are not allowed, step indirectly references itself"
                )
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    pub fn test_indirect_recursion() {
        let err = compile_err(json!([{
            "id": "step",
            "inputs": ["input", "step2"],
            "transform": r#"{
                "f1": $step2.test
            }"#
        }, {
            "id": "step2",
            "inputs": ["step"],
            "transform": r#"{
                "f1": $step.test
            }"#
        }]));
        match err {
            CompileError::Config(d) => {
                assert_eq!(d.id, Some("step2".to_string()));
                assert_eq!(
                    d.desc,
                    "Recursive transformations are not allowed, step2 indirectly references itself"
                )
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    pub fn test_missing_input() {
        let err = compile_err(json!([{
            "id": "step",
            "inputs": ["input", "step2"],
            "transform": r#"{
                "f1": $step2.test
            }"#
        }]));
        match err {
            CompileError::Config(d) => {
                assert_eq!(d.id, Some("step".to_string()));
                assert_eq!(d.desc, "Input step2 to step is not defined")
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    // Numbers
    #[test]
    pub fn test_add_different_types() {
        let result = compile(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "$input.val + 5.5"
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
            "transform": "$input.val + 5"
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
            "transform": "$input.val - 10"
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
            "transform": "10 / $input.val"
        }]))
        .unwrap();
        let res = result.execute(&json!({ "val": 0 })).unwrap_err();
        match res {
            TransformError::InvalidOperation(d) => {
                assert_eq!(d.id, "step");
                assert_eq!(d.desc, "Divide by zero");
                assert_eq!(d.span, Span { start: 3, end: 4 });
            }
            _ => panic!("Wrong type of error {res:?}"),
        }
    }

    #[test]
    pub fn test_non_numeric_input() {
        let result = compile(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "10 * $input.val"
        }]))
        .unwrap();
        let res = result.execute(&json!({ "val": "test" })).unwrap_err();
        match res {
            TransformError::IncorrectTypeInField(d) => {
                assert_eq!(d.id, "step");
                assert_eq!(d.desc, "'*'. Got string, expected number");
                assert_eq!(d.span, Span { start: 3, end: 4 });
            }
            _ => panic!("Wrong type of error {res:?}"),
        }
    }

    #[test]
    pub fn test_wrong_function_input() {
        let result = compile(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "pow(10, $input.val)"
        }]))
        .unwrap();
        let res = result.execute(&json!({ "val": "test" })).unwrap_err();
        match res {
            TransformError::IncorrectTypeInField(d) => {
                assert_eq!(d.id, "step");
                assert_eq!(d.desc, "pow. Got string, expected number");
                assert_eq!(d.span, Span { start: 0, end: 19 });
            }
            _ => panic!("Wrong type of error {res:?}"),
        }
    }

    #[test]
    pub fn test_source_missing_error() {
        let result = compile_err(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "pow(10, $foo.val)"
        }]));
        match result {
            CompileError::Optimizer(OptimizerCompileError {
                err: TransformError::SourceMissingError(d),
                ..
            }) => {
                assert_eq!(d.id, "optimizer");
                assert_eq!(d.desc, "foo");
                assert_eq!(d.span, Span { start: 8, end: 17 });
            }
            _ => panic!("Wrong type of error {result:?}"),
        }
    }

    #[test]
    pub fn test_wrong_source_selector() {
        let result = compile_err(json!([{
            "id": "step",
            "inputs": ["input"],
            "transform": "$[0]"
        }]));
        match result {
            CompileError::Optimizer(OptimizerCompileError {
                err: TransformError::IncorrectTypeInField(d),
                ..
            }) => {
                assert_eq!(d.id, "optimizer");
                assert_eq!(
                    d.desc,
                    "First selector from input must be a string. Got number, expected String"
                );
                assert_eq!(d.span, Span { start: 0, end: 4 });
            }
            _ => panic!("Wrong type of error {result:?}"),
        }
    }
    // Filter

    #[test]
    pub fn test_filter() {
        let program = compile(json!([{
            "id": "gen",
            "inputs": [],
            "transform": "[1, 2, null, 3, null, 4]",
            "expandOutput": true
        }, {
            "id": "filter",
            "inputs": ["gen"],
            "transform": "$gen",
            "type": "filter"
        }]))
        .unwrap();
        let input = Value::Null;
        let res = program.execute(&input).unwrap();
        assert_eq!(res.len(), 4);
        let rs = serde_json::to_string(&Value::Array(res)).unwrap();
        println!("{}", &rs);
        assert_eq!(rs, "[1,2,3,4]");
    }

    #[test]
    pub fn test_merge_filter() {
        let program = compile(json!([{
            "id": "gen",
            "inputs": [],
            "transform": "[1, 2, null, 3, null, 4]",
            "expandOutput": true
        }, {
            "id": "gen2",
            "inputs": [],
            "transform": "[5, 6, null, 7, null, 8]",
            "expandOutput": true
        }, {
            "id": "filter",
            "inputs": ["gen", "gen2"],
            "transform": "$merge",
            "type": "filter",
            "mode": "merge"
        }]))
        .unwrap();
        let input = Value::Null;
        let res = program.execute(&input).unwrap();

        assert_eq!(res.len(), 8);
    }

    #[test]
    pub fn test_too_many_inputs_filter() {
        let err = compile_err(json!([{
            "id": "gen",
            "inputs": [],
            "transform": "[1, 2, null, 3, null, 4]",
            "expandOutput": true
        }, {
            "id": "filter",
            "inputs": ["gen", "input"],
            "transform": "$merge",
            "type": "filter"
        }]));
        match err {
            CompileError::Config(d) => {
                assert_eq!(d.id, Some("filter".to_string()));
                assert_eq!(
                    d.desc,
                    "Filter operations must have exactly one input or use input mode \"merge\""
                );
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    pub fn test_negate_op() {
        let program = compile(json!([{
            "id": "parse",
            "inputs": ["input"],
            "transform": r#"{
                "v1": !$input.v1,
                "v2": !!!$input.v2
            }"#
        }]))
        .unwrap();
        let input = json!({
            "v1": "test",
            "v2": null
        });
        let res = program.execute(&input).unwrap();
        assert_eq!(res.len(), 1);
        let res = res.first().unwrap();
        assert!(!res.get("v1").unwrap().as_bool().unwrap());
        assert!(res.get("v2").unwrap().as_bool().unwrap());
    }

    #[test]
    pub fn test_compare_operators() {
        let program = compile(json!([{
            "id": "cmp",
            "inputs": ["input"],
            "transform": r#"{
                "gt": $input.v1 > $input.v2,
                "gte": $input.v1 >= $input.v2,
                "lt": $input.v1 < $input.v2,
                "lte": $input.v1 <= $input.v2,
                "eq": $input.v1 == $input.v2,
                "neq": $input.v1 != $input.v2
            }"#
        }]))
        .unwrap();
        let input = json!({
            "v1": 1,
            "v2": 1.5
        });
        let res = program.execute(&input).unwrap();
        assert_eq!(res.len(), 1);
        let res = res.first().unwrap();
        assert!(!res.get("gt").unwrap().as_bool().unwrap());
        assert!(!res.get("gte").unwrap().as_bool().unwrap());
        assert!(res.get("lt").unwrap().as_bool().unwrap());
        assert!(res.get("lte").unwrap().as_bool().unwrap());
        assert!(!res.get("eq").unwrap().as_bool().unwrap());
        assert!(res.get("neq").unwrap().as_bool().unwrap());
    }
    #[test]
    pub fn test_compare_operators_eq() {
        let program = compile(json!([{
            "id": "cmp",
            "inputs": ["input"],
            "transform": r#"{
                "gt": $input.v1 > $input.v2,
                "gte": $input.v1 >= $input.v2,
                "lt": $input.v1 < $input.v2,
                "lte": $input.v1 <= $input.v2,
                "eq": $input.v1 == $input.v2,
                "neq": $input.v1 != $input.v2
            }"#
        }]))
        .unwrap();
        let input = json!({
            "v1": 1,
            "v2": 1.0
        });
        let res = program.execute(&input).unwrap();
        assert_eq!(res.len(), 1);
        let res = res.first().unwrap();
        assert!(!res.get("gt").unwrap().as_bool().unwrap());
        assert!(res.get("gte").unwrap().as_bool().unwrap());
        assert!(!res.get("lt").unwrap().as_bool().unwrap());
        assert!(res.get("lte").unwrap().as_bool().unwrap());
        assert!(res.get("eq").unwrap().as_bool().unwrap());
        assert!(!res.get("neq").unwrap().as_bool().unwrap());
    }

    #[test]
    pub fn test_boolean_operators() {
        let program = compile(json!([{
            "id": "cmp",
            "inputs": ["input"],
            "transform": r#"{
                "v1": $input.v1 && $input.v2 || $input.v3
            }"#
        }]))
        .unwrap();
        let input = json!({
            "v1": true,
            "v2": "test",
            "v3": null
        });
        let res = program.execute(&input).unwrap();
        assert_eq!(res.len(), 1);
        let res = res.first().unwrap();
        assert!(res.get("v1").unwrap().as_bool().unwrap());
    }

    fn compile_aliased(
        value: Value,
        alias: HashMap<usize, Vec<String>>,
    ) -> Result<Program, CompileError> {
        Program::compile_map(serde_json::from_value(value).unwrap(), &alias)
    }

    fn compile_err_aliased(value: Value, alias: HashMap<usize, Vec<String>>) -> CompileError {
        match compile_aliased(value, alias) {
            Ok(_) => panic!("Expected compilation to fail"),
            Err(x) => x,
        }
    }

    #[test]
    pub fn test_multiple_inputs() {
        let program = compile(json!([{
            "id": "test",
            "inputs": ["input", "input1", "input2"],
            "transform": r#"{
                "i1": $input,
                "i2": $input1,
                "i3": $input2
            }"#
        }]))
        .unwrap();
        let i1 = json!(123);
        let i2 = json!("test");
        let i3 = json!({ "test": 123 });
        let res = program.execute_multiple(&[&i1, &i2, &i3]).unwrap();
        assert_eq!(res.len(), 1);
        let res = res.first().unwrap();
        assert_eq!(res.get("i1").unwrap().as_i64().unwrap(), 123);
        assert_eq!(res.get("i2").unwrap().as_str().unwrap(), "test");
        assert_eq!(
            res.get("i3")
                .unwrap()
                .as_object()
                .unwrap()
                .get("test")
                .unwrap()
                .as_i64()
                .unwrap(),
            123
        );
    }

    #[test]
    pub fn test_multiple_inputs_aliased() {
        let program = compile_aliased(
            json!([{
                "id": "test2",
                "inputs": ["input", "mystery", "test"],
                "transform": r#"{
                    "i1": $input,
                    "i2": $mystery,
                    "i3": $input2
                }"#
            }]),
            HashMap::from_iter([
                (1, vec!["mystery".to_owned()]),
                (2, vec!["test".to_owned()]),
            ]),
        )
        .unwrap();

        let i1 = json!(123);
        let i2 = json!("test");
        let i3 = json!({ "test": 123 });
        let res = program.execute_multiple(&[&i1, &i2, &i3]).unwrap();
        assert_eq!(res.len(), 1);
        let res = res.first().unwrap();
        assert_eq!(res.get("i1").unwrap().as_i64().unwrap(), 123);
        assert_eq!(res.get("i2").unwrap().as_str().unwrap(), "test");
        assert_eq!(
            res.get("i3")
                .unwrap()
                .as_object()
                .unwrap()
                .get("test")
                .unwrap()
                .as_i64()
                .unwrap(),
            123
        );
    }

    #[test]
    pub fn test_multiple_inputs_aliased_err() {
        let err = compile_err_aliased(
            json!([{
                "id": "test",
                "inputs": ["input", "mystery", "test"],
                "transform": r#"{
                    "i1": $input,
                    "i2": $mystery,
                    "i3": $input2
                }"#
            }]),
            HashMap::from_iter([
                (1, vec!["mystery".to_owned()]),
                (2, vec!["test".to_owned()]),
            ]),
        );

        match err {
            CompileError::Config(d) => {
                assert_eq!(d.id, Some("test".to_string()));
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    pub fn test_object_creation() {
        let program = compile(json!([{
            "id": "test",
            "inputs": ["input"],
            "transform": r#"{
                "i1": { concat("test", "test"): 1 + 2 + 3, "val": $input.val }
            }"#
        }]))
        .unwrap();

        let inp = json!({ "val": 7 });
        let res = program.execute(&inp).unwrap();
        let res = res.into_iter().next().unwrap();

        let obj = res.as_object().unwrap();
        let obj = obj.get("i1").unwrap().as_object().unwrap();
        assert_eq!(obj.get("testtest").unwrap().as_u64().unwrap(), 6);
        assert_eq!(obj.get("val").unwrap().as_u64().unwrap(), 7);
    }

    #[test]
    pub fn test_object_indexing() {
        let program = compile(json!([{
            "id": "test",
            "inputs": ["input"],
            "transform": r#"{
                "i1": { concat("test", "test"): { "test": 8 }, "val": $input.val }["testtest"].test
            }"#
        }]))
        .unwrap();

        let inp = json!({ "val": 7 });
        let res = program.execute(&inp).unwrap();
        let res = res.into_iter().next().unwrap();

        let obj = res.as_object().unwrap();
        assert_eq!(obj.get("i1").unwrap().as_u64().unwrap(), 8);
    }

    #[test]
    pub fn test_array_indexing() {
        let program = compile(json!([{
            "id": "test",
            "inputs": ["input"],
            "transform": r#"{
                "i1": [[[1, 2, 3], [4], [5, 6], [7, [8]]]][0][3][1][0]
            }"#
        }]))
        .unwrap();

        let inp = json!({ "val": 7 });
        let res = program.execute(&inp).unwrap();
        let res = res.into_iter().next().unwrap();

        let obj = res.as_object().unwrap();
        println!("{:?}", res);
        assert_eq!(obj.get("i1").unwrap().as_u64().unwrap(), 8);
    }

    #[test]
    pub fn test_object_return() {
        let program = compile(json!([{
            "id": "test",
            "inputs": ["input"],
            "transform": r#"{ "key": "value", "key2": $input.val, "key3": { "nested": [1, 2, 3] } }"#
        }]))
        .unwrap();

        let inp = json!({ "val": 7 });
        let res = program.execute(&inp).unwrap();
        let res = res.into_iter().next().unwrap();

        let obj = res.as_object().unwrap();
        assert_eq!(obj.get("key").unwrap().as_str().unwrap(), "value");
        assert_eq!(obj.get("key2").unwrap().as_u64().unwrap(), 7);
        assert_eq!(
            obj.get("key3")
                .unwrap()
                .as_object()
                .unwrap()
                .get("nested")
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            3
        );
    }
}
