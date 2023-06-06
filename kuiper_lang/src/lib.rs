//! # Unnamed JSON transform library
//!
//! This library defines a JSON to JSON transform and templating language. The language itself is
//! inspired by JavaScript. Expressions always terminate, as the language has no form of recursion.
//! This means that while there are loops, they only operate on input arrays. So it is possible to iterate over
//! an array, and even pairs of arrays, but it is not possible to implement recursion.
//!
//! ## Features
//!
//! - [Operators], `+`, `-`, `*`, `/`, `==`, `!=`, `>=`, `<=`, `>`, `<`, `&&`, `||` with precendence taken from the C++ standard.
//! - [Arrays], [1, 2, "test", 123.123, [123, 2]]
//! - [Objects], { "test": "123", concat("test", "test"): 321 }
//! - [Built in functions], like `map`, `float`, `concat`, etc. Either `pow(base, exp) or base.pow(exp)`
//! - [Functors], `map` is a functor, meaning it accepts a lambda: `map(arr, field => ...)` or `arr.map(field => ...)`
//! - [Selector expressions], `[1, 2, 3][1] == 2`, `input.field.value["dynamic"]`, etc.
//!
//! ## Usage
//!
//! ```
//! use kuiper_lang::compile_expression;
//! use std::collections::HashMap;
//! use serde_json::json;
//!
//! let transform = compile_expression("input.value + 5", &["input"]).unwrap();
//!
//! let input = [json!({ "value": 2 })];
//! let result = transform.run(input.iter()).unwrap();
//!
//! assert_eq!(result.as_u64().unwrap(), 7);
//! ```

mod compiler;
mod expressions;
mod lexer;
mod parse;

static NULL_CONST: Value = Value::Null;

/// A failed compilation, contains sub-errors for each stage of the compilation.
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Compilation failed: {0}")]
    Build(#[from] BuildError),
    #[error("Compilation failed: {0}")]
    Parser(#[from] ParseError),
    #[error("Compilation failed: {0}")]
    Config(String),
    #[error("Compilation failed: {0}")]
    Optimizer(#[from] TransformError),
}

pub use compiler::{compile_expression, BuildError, DebugInfo, ExpressionDebugInfo};
pub use expressions::{ExpressionType, TransformError, TransformErrorData};
pub use lexer::ParseError;
use serde_json::Value;
use thiserror::Error;

#[cfg(test)]
mod tests {
    use logos::Span;
    use serde_json::json;

    use crate::{compile_expression, compiler::BuildError, CompileError, TransformError};

    fn compile_err(data: &str, inputs: &[&str]) -> CompileError {
        match compile_expression(data, inputs) {
            Ok(_) => panic!("Expected compilation to fail"),
            Err(x) => x,
        }
    }

    // Compile errors
    #[test]
    pub fn test_build_error() {
        let err = compile_err("pow(input.test)", &["input"]);
        match err {
            CompileError::Build(BuildError::NFunctionArgs(d)) => {
                assert_eq!(
                    d.detail,
                    Some(
                        "Incorrect number of function args: function pow takes 2 arguments"
                            .to_string()
                    )
                );
                assert_eq!(d.position, Span { start: 0, end: 15 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    // Numbers
    #[test]
    pub fn test_add_different_types() {
        let expr = compile_expression("input.val + 5.5", &["input"]).unwrap();
        let inp = json!({ "val": 5 });
        let res = expr.run([&inp]).unwrap();
        assert_eq!(10.5, res.as_f64().unwrap());
    }

    #[test]
    pub fn test_add_keeps_type() {
        let expr = compile_expression("input.val + 5", &["input"]).unwrap();
        let inp = json!({ "val": 5 });
        let res = expr.run([&inp]).unwrap();
        assert_eq!(10, res.as_u64().unwrap());
    }

    #[test]
    pub fn test_negative_result() {
        let expr = compile_expression("input.val - 10", &["input"]).unwrap();
        let inp = json!({ "val": 5 });
        let res = expr.run([&inp]).unwrap();
        assert_eq!(-5, res.as_i64().unwrap());
    }

    #[test]
    pub fn test_divide_by_zero() {
        let expr = compile_expression("10 / input.val", &["input"]).unwrap();
        let res = expr.run([&json!({ "val": 0 })]).unwrap_err();
        match res {
            TransformError::InvalidOperation(d) => {
                assert_eq!(d.desc, "Divide by zero");
                assert_eq!(d.span, Span { start: 0, end: 14 });
            }
            _ => panic!("Wrong type of error {res:?}"),
        }
    }

    #[test]
    pub fn test_non_numeric_input() {
        let expr = compile_expression("10 * input.val", &["input"]).unwrap();
        let res = expr.run([&json!({ "val": "test" })]).unwrap_err();
        match res {
            TransformError::IncorrectTypeInField(d) => {
                assert_eq!(d.desc, "'*'. Got string, expected number");
                assert_eq!(d.span, Span { start: 0, end: 14 });
            }
            _ => panic!("Wrong type of error {res:?}"),
        }
    }

    #[test]
    pub fn test_wrong_function_input() {
        let expr = compile_expression("pow(10, input.val)", &["input"]).unwrap();
        let res = expr.run([&json!({ "val": "test" })]).unwrap_err();
        match res {
            TransformError::IncorrectTypeInField(d) => {
                assert_eq!(d.desc, "pow. Got string, expected number");
                assert_eq!(d.span, Span { start: 0, end: 18 });
            }
            _ => panic!("Wrong type of error {res:?}"),
        }
    }

    #[test]
    pub fn test_source_missing_error() {
        let result = compile_err("pow(10, foo.val)", &[]);
        match result {
            CompileError::Optimizer(TransformError::SourceMissingError(d)) => {
                assert_eq!(d.desc, "foo");
                assert_eq!(d.span, Span { start: 8, end: 15 });
            }
            _ => panic!("Wrong type of error {result:?}"),
        }
    }

    #[test]
    pub fn test_negate_op() {
        let expr = compile_expression(
            r#"{
            "v1": !input.v1,
            "v2": !!!input.v2
        }"#,
            &["input"],
        )
        .unwrap();
        let input = json!({
            "v1": "test",
            "v2": null
        });
        let res = expr.run([&input]).unwrap();
        assert!(!res.get("v1").unwrap().as_bool().unwrap());
        assert!(res.get("v2").unwrap().as_bool().unwrap());
    }

    #[test]
    pub fn test_compare_operators() {
        let expr = compile_expression(
            r#"{
            "gt": input.v1 > input.v2,
            "gte": input.v1 >= input.v2,
            "lt": input.v1 < input.v2,
            "lte": input.v1 <= input.v2,
            "eq": input.v1 == input.v2,
            "neq": input.v1 != input.v2
        }"#,
            &["input"],
        )
        .unwrap();
        let input = json!({
            "v1": 1,
            "v2": 1.5
        });
        let res = expr.run([&input]).unwrap();
        assert!(!res.get("gt").unwrap().as_bool().unwrap());
        assert!(!res.get("gte").unwrap().as_bool().unwrap());
        assert!(res.get("lt").unwrap().as_bool().unwrap());
        assert!(res.get("lte").unwrap().as_bool().unwrap());
        assert!(!res.get("eq").unwrap().as_bool().unwrap());
        assert!(res.get("neq").unwrap().as_bool().unwrap());
    }
    #[test]
    pub fn test_compare_operators_eq() {
        let expr = compile_expression(
            r#"{
            "gt": input.v1 > input.v2,
            "gte": input.v1 >= input.v2,
            "lt": input.v1 < input.v2,
            "lte": input.v1 <= input.v2,
            "eq": input.v1 == input.v2,
            "neq": input.v1 != input.v2
        }"#,
            &["input"],
        )
        .unwrap();
        let input = json!({
            "v1": 1,
            "v2": 1.0
        });
        let res = expr.run([&input]).unwrap();
        assert!(!res.get("gt").unwrap().as_bool().unwrap());
        assert!(res.get("gte").unwrap().as_bool().unwrap());
        assert!(!res.get("lt").unwrap().as_bool().unwrap());
        assert!(res.get("lte").unwrap().as_bool().unwrap());
        assert!(res.get("eq").unwrap().as_bool().unwrap());
        assert!(!res.get("neq").unwrap().as_bool().unwrap());
    }

    #[test]
    pub fn test_boolean_operators() {
        let expr = compile_expression(
            r#"{
            "v1": input.v1 && input.v2 || input.v3
        }"#,
            &["input"],
        )
        .unwrap();
        let input = json!({
            "v1": true,
            "v2": "test",
            "v3": null
        });
        let res = expr.run([&input]).unwrap();
        assert!(res.get("v1").unwrap().as_bool().unwrap());
    }

    #[test]
    pub fn test_multiple_inputs() {
        let expr = compile_expression(
            r#"{
            "i1": input,
            "i2": input1,
            "i3": input2
        }"#,
            &["input", "input1", "input2"],
        )
        .unwrap();
        let i1 = json!(123);
        let i2 = json!("test");
        let i3 = json!({ "test": 123 });
        let res = expr.run([&i1, &i2, &i3]).unwrap();
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
    pub fn test_object_creation() {
        let expr = compile_expression(
            r#"{
            "i1": { concat("test", "test"): 1 + 2 + 3, "val": input.val }
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({ "val": 7 });
        let res = expr.run([&inp]).unwrap();
        let obj = res.as_object().unwrap();
        let obj = obj.get("i1").unwrap().as_object().unwrap();
        assert_eq!(obj.get("testtest").unwrap().as_u64().unwrap(), 6);
        assert_eq!(obj.get("val").unwrap().as_u64().unwrap(), 7);
    }

    #[test]
    pub fn test_object_indexing() {
        let expr = compile_expression(
            r#"{
            "i1": { concat("test", "test"): { "test": 8 }, "val": input.val }["testtest"].test
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({ "val": 7 });
        let res = expr.run([&inp]).unwrap();
        let obj = res.as_object().unwrap();
        assert_eq!(obj.get("i1").unwrap().as_u64().unwrap(), 8);
    }

    #[test]
    pub fn test_array_indexing() {
        let expr = compile_expression(
            r#"{
            "i1": [[[1, 2, 3], [4], [5, 6], [7, [8]]]][0][3][1][0]
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({ "val": 7 });
        let res = expr.run([&inp]).unwrap();

        let obj = res.as_object().unwrap();
        println!("{:?}", res);
        assert_eq!(obj.get("i1").unwrap().as_u64().unwrap(), 8);
    }

    #[test]
    pub fn test_object_return() {
        let expr = compile_expression(
            r#"{ "key": "value", "key2": input.val, "key3": { "nested": [1, 2, 3] } }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({ "val": 7 });
        let res = expr.run([&inp]).unwrap();
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
    #[test]
    pub fn test_nested_postfix_function() {
        let expr = compile_expression(
            r#"{ "test": [1, 2, 3, 4] }.test.map((a) => a * 2)[0].pow(2)"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();
        assert_eq!(res.as_f64().unwrap(), 4.0);
    }
    #[test]
    pub fn test_modulo_operator() {
        let expr = compile_expression("[1, 2, 3, 4].filter((a) => a % 2 == 1)", &[]).unwrap();

        let res = expr.run([]).unwrap();
        let val = res.as_array().unwrap();
        assert_eq!(2, val.len());
        assert_eq!(val[0].as_u64().unwrap(), 1);
        assert_eq!(val[1].as_u64().unwrap(), 3);
    }
    #[test]
    pub fn test_complicated_operator_precedence() {
        let expr = compile_expression("1 == 1 && 2 == 2 || (2 + 2) != 4", &[]).unwrap();

        let res = expr.run([]).unwrap();
        assert!(res.as_bool().unwrap());
    }
    #[test]
    pub fn test_variable_ordering() {
        let expr = compile_expression("input.map([1].map(a => a + 1))", &["input"]).unwrap();

        let inp = json!([1, 2, 3]);
        let res = expr.run([&inp]).unwrap();
        let res_arr = res.as_array().unwrap();
        assert_eq!(res_arr.len(), 3);
        for el in res_arr {
            let el_arr = el.as_array().unwrap();
            assert_eq!(1, el_arr.len());
            assert_eq!(el_arr.first().unwrap().as_u64().unwrap(), 2);
        }
    }
    #[test]
    pub fn test_is_operator() {
        let expr = compile_expression(
            r#"{
            "v1": "test" is "string",
            "v2": "test" is "number",
            "v3": 123 is "number",
            "v4": 123.0 is "int",
            "v5": true is "bool",
            "v6": [1, 2, 3] is "object",
            "v7": [1, 2, 3] is "array"
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();
        let res_obj = res.as_object().unwrap();

        assert!(res_obj.get("v1").unwrap().as_bool().unwrap());
        assert!(!res_obj.get("v2").unwrap().as_bool().unwrap());
        assert!(res_obj.get("v3").unwrap().as_bool().unwrap());
        assert!(!res_obj.get("v4").unwrap().as_bool().unwrap());
        assert!(res_obj.get("v5").unwrap().as_bool().unwrap());
        assert!(!res_obj.get("v6").unwrap().as_bool().unwrap());
        assert!(res_obj.get("v7").unwrap().as_bool().unwrap());
    }
}
