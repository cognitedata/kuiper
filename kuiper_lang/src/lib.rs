//! # The Kuiper language
//!
//! This library defines a JSON to JSON transform and templating language. The language itself is
//! inspired by JavaScript. Expressions always terminate, as the language has no form of recursion.
//! This means that while there are loops, they only operate on input arrays. So it is possible to iterate over
//! an array, and even pairs of arrays, but it is not possible to implement recursion.
//!
//! The language itself is documented [here](https://docs.cognite.com/cdf/integration/guides/extraction/hosted_extractors/kuiper_concepts).
//!
//! ## Features
//!
//! - [Operators], `+`, `-`, `*`, `/`, `==`, `!=`, `>=`, `<=`, `>`, `<`, `&&`, `||` with precendence taken from the C++ standard.
//! - [Arrays], [1, 2, "test", 123.123, [123, 2]]
//! - [Objects], { "test": "123", concat("test", "test"): 321 }
//! - [Built in functions], like `map`, `float`, `concat`, etc. Either `pow(base, exp) or base.pow(exp)`
//! - [Functors], `map` is a functor, meaning it accepts a lambda: `map(arr, field => ...)` or `arr.map(field => ...)`
//! - [Selector expressions], `[1, 2, 3][1] == 2`, `input.field.value["dynamic"]`, etc.
//! - **Macros**, `#my_macro := (a, b) => a + b; my_macro(1, 2)`
//!
//! ## Usage
//!
//! ```
//! use kuiper_lang::compile_expression;
//! use serde_json::json;
//!
//! let expr = compile_expression("input.test + 5", &["input"]).unwrap();
//! let value = json!({ "test": 3 });
//! let result = expr.run([&value]).unwrap();
//! assert_eq!(result.as_ref(), &json!(8));
//! ```

#![warn(missing_docs)]

mod compiler;
mod expressions;
mod lexer;
mod parse;
mod pretty;
pub mod types;

pub use pretty::{format_expression, PrettyError};

/// A constant null value, which can be handy when implementing SourceData, as a fallback
/// if a key is not found.
pub static NULL_CONST: Value = Value::Null;

/// A failed compilation, contains sub-errors for each stage of the compilation.
#[derive(Debug, Error)]
pub enum CompileError {
    /// An error during the build phase of compilation.
    #[error("Compilation failed: {0}")]
    Build(#[from] BuildError),
    /// An error during parsing.
    #[error("Compilation failed: {0}")]
    Parser(#[from] ParseError),
    /// An error during optimization.
    #[error("Compilation failed: {0}")]
    Optimizer(#[from] TransformError),
    /// An error during type checking.
    #[error("Type checking failed: {0}")]
    TypeChecker(#[from] TypeError),
}

impl CompileError {
    /// Get the span of code that caused the error, if available.
    pub fn span(&self) -> Option<Span> {
        match self {
            CompileError::Build(x) => match x {
                BuildError::NFunctionArgs(x) => Some(x.position.clone()),
                BuildError::UnexpectedLambda(x) => Some(x.position.clone()),
                BuildError::UnrecognizedFunction(x) => Some(x.position.clone()),
                BuildError::UnknownVariable(x) => Some(x.position.clone()),
                BuildError::VariableConflict(x) => Some(x.position.clone()),
                BuildError::Other(x) => Some(x.position.clone()),
            },
            CompileError::Parser(x) => match x {
                lalrpop_util::ParseError::InvalidToken { location } => Some(Span {
                    start: *location,
                    end: *location,
                }),
                lalrpop_util::ParseError::UnrecognizedEof {
                    location,
                    expected: _,
                } => Some(Span {
                    start: *location,
                    end: *location,
                }),
                lalrpop_util::ParseError::UnrecognizedToken { token, expected: _ } => Some(Span {
                    start: token.0,
                    end: token.2,
                }),
                lalrpop_util::ParseError::ExtraToken { token } => Some(Span {
                    start: token.0,
                    end: token.2,
                }),
                lalrpop_util::ParseError::User { error } => match error {
                    lexer::LexerError::UnknownToken => None,
                    lexer::LexerError::InvalidToken(x) => Some(x.clone()),
                    lexer::LexerError::ParseInt(x) => Some(x.1.clone()),
                    lexer::LexerError::ParseFloat(x) => Some(x.1.clone()),
                    lexer::LexerError::InvalidEscapeChar(x) => Some(x.1.clone()),
                },
            },
            CompileError::Optimizer(t) => t.span(),
            CompileError::TypeChecker(t) => Some(t.span().clone()),
        }
    }

    /// Get a human readable message describing the error.
    pub fn message(&self) -> String {
        match self {
            CompileError::Build(build_error) => match build_error {
                BuildError::NFunctionArgs(compile_error_data) => {
                    compile_error_data.detail.to_string()
                }
                BuildError::UnexpectedLambda(compile_error_data) => {
                    compile_error_data.detail.to_string()
                }
                BuildError::UnrecognizedFunction(compile_error_data) => {
                    format!("Unrecognized function {}", compile_error_data.detail)
                }
                BuildError::UnknownVariable(compile_error_data) => {
                    format!("Unknown variable {}", compile_error_data.detail)
                }
                BuildError::VariableConflict(compile_error_data) => {
                    format!("Variable {} already defined", compile_error_data.detail)
                }
                BuildError::Other(compile_error_data) => compile_error_data.detail.clone(),
            },
            CompileError::Parser(parse_error) => parse_error.to_string(),
            CompileError::Optimizer(transform_error) => transform_error.message(),
            CompileError::TypeChecker(type_error) => type_error.to_string(),
        }
    }
}

pub use compiler::{
    compile_expression, compile_expression_with_config, BuildError, CompilerConfig, DebugInfo,
    ExpressionDebugInfo,
};
#[cfg(feature = "completions")]
pub use expressions::Completions;
pub use expressions::{
    ExpressionRunBuilder, ExpressionType, ResolveResult, TransformError, TransformErrorData,
};
pub use lexer::ParseError;
pub use logos::Span;

/// Module for utilties for working with input data as a stream of tokens,
/// rather than a string.
pub mod lex {
    pub use super::compiler::compile_from_tokens;
    pub use super::expressions::Operator;
    pub use super::expressions::TypeLiteral;
    pub use super::expressions::UnaryOperator;
    pub use super::lexer::LexerError;
    pub use super::lexer::Token;
}

/// Module containing the SourceData trait and related types,
/// used for creating custom input data sources for expressions.
pub mod source {
    pub use super::expressions::{LazySourceData, LazySourceDataJson, SourceData};
    #[doc(inline)]
    pub use kuiper_lang_macros::SourceData;
}

use serde_json::Value;
use thiserror::Error;

macro_rules! write_list {
    ($f:ident, $iter:expr) => {
        let mut needs_comma = false;
        for it in $iter {
            if needs_comma {
                write!($f, ", ")?;
            }
            needs_comma = true;
            write!($f, "{it}")?;
        }
    };
}

pub(crate) use write_list;

use crate::types::TypeError;

#[cfg(test)]
fn compile_expression_test(
    data: &str,
    known_inputs: &[&str],
) -> Result<ExpressionType, CompileError> {
    compile_expression_with_config(
        data,
        known_inputs,
        &CompilerConfig::new().type_checker_mode(crate::compiler::TypeCheckerMode::Early),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use logos::{Logos, Span};
    use serde_json::json;
    use std::path::PathBuf;

    use crate::{
        compile_expression, compile_expression_test, compile_expression_with_config,
        compiler::BuildError, format_expression, lex::Token, CompileError, CompilerConfig,
        ExpressionDebugInfo, TransformError,
    };

    pub(crate) fn compile_err(data: &str, inputs: &[&str]) -> CompileError {
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
                    "Incorrect number of function args: function pow takes 2 arguments".to_string()
                );
                assert_eq!(d.position, Span { start: 0, end: 15 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    // Numbers
    #[test]
    pub fn test_add_different_types() {
        let expr = compile_expression_test("input.val + 5.5", &["input"]).unwrap();
        let inp = json!({ "val": 5 });
        let res = expr.run([&inp]).unwrap();
        assert_eq!(10.5, res.as_f64().unwrap());
    }

    #[test]
    pub fn test_add_keeps_type() {
        let expr = compile_expression_test("input.val + 5", &["input"]).unwrap();
        let inp = json!({ "val": 5 });
        let res = expr.run([&inp]).unwrap();
        assert_eq!(10, res.as_u64().unwrap());
    }

    #[test]
    pub fn test_negative_result() {
        let expr = compile_expression_test("input.val - 10", &["input"]).unwrap();
        let inp = json!({ "val": 5 });
        let res = expr.run([&inp]).unwrap();
        assert_eq!(-5, res.as_i64().unwrap());
    }

    #[test]
    pub fn test_divide_by_zero() {
        let expr = compile_expression_test("10 / input.val", &["input"]).unwrap();
        let res = expr.run([&json!({ "val": 0 })]).unwrap_err();
        match res {
            TransformError::InvalidOperation(d) => {
                assert_eq!(d.desc, "Divide by zero");
                assert_eq!(d.span, Span { start: 3, end: 4 });
            }
            _ => panic!("Wrong type of error {res:?}"),
        }
    }

    #[test]
    pub fn test_non_numeric_input() {
        let expr = compile_expression_test("10 * input.val", &["input"]).unwrap();
        let res = expr.run([&json!({ "val": "test" })]).unwrap_err();
        match res {
            TransformError::IncorrectTypeInField(d) => {
                assert_eq!(d.desc, "'*'. Got string, expected number");
                assert_eq!(d.span, Span { start: 3, end: 4 });
            }
            _ => panic!("Wrong type of error {res:?}"),
        }
    }

    #[test]
    pub fn test_wrong_function_input() {
        let expr = compile_expression_test("pow(10, input.val)", &["input"]).unwrap();
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
            CompileError::Build(BuildError::UnknownVariable(d)) => {
                assert_eq!(d.detail, "foo".to_string());
                assert_eq!(d.position, Span { start: 8, end: 11 });
            }
            _ => panic!("Wrong type of error {result:?}"),
        }
    }

    #[test]
    pub fn test_source_conflict_error() {
        let result = compile_err("a.map(a => a.foo)", &["a"]);
        match result {
            CompileError::Build(BuildError::VariableConflict(d)) => {
                assert_eq!("a".to_string(), d.detail);
                assert_eq!(d.position, Span { start: 6, end: 16 });
            }
            _ => panic!("Wrong type of error {result:?}"),
        }
    }

    #[test]
    pub fn test_negate_op() {
        let expr = compile_expression_test(
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
        let expr = compile_expression_test(
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
        let expr = compile_expression_test(
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
    pub fn test_equality_cross_type() {
        let expr = compile_expression_test(
            r#"
            {
                "v1": "foo" == 123,
                "v2": 123 == "foo",
                "v3": 123.0 == 123
            }
        "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        assert!(!res.get("v1").unwrap().as_bool().unwrap());
        assert!(!res.get("v2").unwrap().as_bool().unwrap());
        assert!(res.get("v3").unwrap().as_bool().unwrap());
    }

    #[test]
    pub fn test_boolean_operators() {
        let expr = compile_expression_test(
            r#"{
            "v1": input.v1 && input.v2 || input.v3,
            "v2": 1 && 2 && 3,
            "v3": null && "wow"
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
        assert!(res.get("v2").unwrap().as_bool().unwrap());
        assert!(!res.get("v3").unwrap().as_bool().unwrap());
    }

    #[test]
    pub fn test_multiple_inputs() {
        let expr = compile_expression_test(
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
        let expr = compile_expression_test(
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
        let expr = compile_expression_test(
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
        let expr = compile_expression_test(
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
        let expr = compile_expression_test(
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
        let expr = compile_expression_test(
            r#"{ "test": [1, 2, 3, 4] }.test.map((a) => a * 2)[0].pow(2)"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();
        assert_eq!(res.as_f64().unwrap(), 4.0);
    }
    #[test]
    pub fn test_modulo_operator() {
        let expr = compile_expression_test("[1, 2, 3, 4].filter((a) => a % 2 == 1)", &[]).unwrap();

        let res = expr.run([]).unwrap();
        let val = res.as_array().unwrap();
        assert_eq!(2, val.len());
        assert_eq!(val[0].as_u64().unwrap(), 1);
        assert_eq!(val[1].as_u64().unwrap(), 3);
    }
    #[test]
    pub fn test_complicated_operator_precedence() {
        let expr = compile_expression_test("1 == 1 && 2 == 2 || (2 + 2) != 4", &[]).unwrap();

        let res = expr.run([]).unwrap();
        assert!(res.as_bool());
    }
    #[test]
    pub fn test_variable_ordering() {
        let expr = compile_expression_test("input.map([1].map(a => a + 1))", &["input"]).unwrap();

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
        let expr = compile_expression_test(
            r#"{
            "v1": "test" is string,
            "v2": "test" is number,
            "v3": 123 is number,
            "v4": 123.0 is int,
            "v5": true is bool,
            "v6": [1, 2, 3] is object,
            "v7": [1, 2, 3] is array
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

    #[cfg(feature = "completions")]
    #[test]
    pub fn test_completions() {
        let expr = compile_expression_test("input.test.foo", &["input"]).unwrap();

        let data = json! {{
            "test": {
                "wow": 123,
                "foo": {
                    "ho-boy": [1, 2, 3]
                }
            },
            "dos": 5
        }};

        let (_, comp) = expr.run_get_completions([&data]).unwrap();
        for c in &comp {
            println!("{c:?}");
        }
        assert_eq!(2, comp.get(&Span { start: 6, end: 10 }).unwrap().len());
        assert_eq!(2, comp.get(&Span { start: 11, end: 14 }).unwrap().len());
    }

    #[test]
    pub fn test_op_limit() {
        let expr =
            compile_expression_test("[input, input, input, input, input]", &["input"]).unwrap();

        let data = json! { 1 };

        assert!(expr.run_limited([&data], 5).is_err());
        assert!(expr.run_limited([&data], 6).is_ok());
    }

    #[test]
    pub fn test_object_concat() {
        let expr = compile_expression_test(
            r#"
        {
            "foo": "bar",
            ...{
                "s1": "v1",
                ...{
                    "s2": "v2"
                }
            },
            ...input
        }"#,
            &["input"],
        )
        .unwrap();

        let data = json!({ "s3": "v3" });
        let r = expr.run(&[data]).unwrap().into_owned();
        let obj = r.as_object().unwrap();

        assert_eq!(4, obj.len());
        assert_eq!("v1", obj.get("s1").unwrap().as_str().unwrap());
        assert_eq!("v2", obj.get("s2").unwrap().as_str().unwrap());
        assert_eq!("v3", obj.get("s3").unwrap().as_str().unwrap());
        assert_eq!("bar", obj.get("foo").unwrap().as_str().unwrap());
    }

    #[test]
    pub fn test_array_concat() {
        let expr = compile_expression_test(
            r#"
            [1, 2, ...[3, 4], ...[5], ...input]
        "#,
            &["input"],
        )
        .unwrap();

        let data = json!([6, 7]);
        let r = expr.run(&[data]).unwrap().into_owned();
        let arr = r.as_array().unwrap();

        assert_eq!(7, arr.len());
        for (idx, it) in arr.iter().enumerate() {
            assert_eq!(idx as u64 + 1, it.as_u64().unwrap());
        }
    }

    #[test]
    pub fn test_comments() {
        let expr = compile_expression_test(
            r#"
        1 + /* hello there, this is a comment */ - 5
        + 3
        // hello block comment here, no math going on 1 + 1
        + 2
        "#,
            &[],
        )
        .unwrap();
        let r = expr.run(&[]).unwrap().into_owned();
        assert_eq!(1, r.as_i64().unwrap());
    }

    #[test]
    pub fn test_comments_2() {
        let expr = compile_expression_test(r#"/* some comment */ {}"#, &[]).unwrap();
        let r = expr.run(&[]).unwrap().into_owned();
        assert_eq!(0, r.as_object().unwrap().len());
    }

    #[test]
    pub fn test_is_2() {
        let expr = compile_expression_test(
            r#"{
                1: 1 is number,
                2: 2 is not string,
                3: null is null,
                4: null is not null,
                5: "test" is string,
                6: 123.123 is int,
                7: true is float,
                8: false is bool,
                9: 123 is not null
            }
            "#,
            &[],
        )
        .unwrap();
        let r = expr.run(&[]).unwrap().into_owned();
        let o = r.as_object().unwrap();

        assert!(o.get("1").unwrap().as_bool().unwrap());
        assert!(o.get("2").unwrap().as_bool().unwrap());
        assert!(o.get("3").unwrap().as_bool().unwrap());
        assert!(!o.get("4").unwrap().as_bool().unwrap());
        assert!(o.get("5").unwrap().as_bool().unwrap());
        assert!(!o.get("6").unwrap().as_bool().unwrap());
        assert!(!o.get("7").unwrap().as_bool().unwrap());
        assert!(o.get("8").unwrap().as_bool().unwrap());
        assert!(o.get("9").unwrap().as_bool().unwrap());
    }

    #[test]
    pub fn test_get_opcount() {
        let expr = compile_expression_test("input.map(x => x + 1)", &["input"]).unwrap();
        let data = json!([1, 2, 3, 4, 5]);
        let (res, opcount) = expr.run_get_opcount([&data]).unwrap();
        assert_eq!(5, res.as_array().unwrap().len());
        // Lookup input once, For each iteration: Call the lambda passed to map, lookup x,
        //resolve the constant `1` and resolve the `+` operator. 1 + 4 * 5 = 21.
        assert_eq!(21, opcount);
    }

    #[test]
    fn test_optimizer_operation_limit() {
        let err = compile_expression_with_config(
            "1 + 1 + 1",
            &[],
            &CompilerConfig::new().optimizer_operation_limit(2),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            CompileError::Optimizer(TransformError::OperationLimitExceeded)
        ));
    }

    #[test]
    fn test_max_macro_expansions() {
        let err = compile_expression_with_config(
            r#"
            #my_macro := (a) => a + 1;
            my_macro(1) + my_macro(2) + my_macro(3)
        "#,
            &[],
            &CompilerConfig::new().max_macro_expansions(2),
        )
        .unwrap_err();

        match err {
            CompileError::Build(BuildError::Other(d)) => {
                assert_eq!(d.detail, "Too many macro expansions, maximum is 2");
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    fn test_compile_from_tokens() {
        use crate::lex::compile_from_tokens;
        let tokens = vec![
            Token::Integer(2),
            Token::Operator(crate::lex::Operator::Plus),
            Token::Integer(1),
        ];

        let expr = compile_from_tokens(tokens.into_iter(), &[], &Default::default()).unwrap();
        let res = expr.run(&[]).unwrap().into_owned();
        assert_eq!(res.as_u64().unwrap(), 3);
    }

    #[test]
    fn test_debug_info_ok() {
        let info = ExpressionDebugInfo::new("1 + 1 + input.test", &["input"], &Default::default())
            .unwrap();
        assert_eq!(info.lexer.to_string(), "1+1+`input`.`test`");
        assert_eq!(info.ast.to_string(), "((1 + 1) + input.test)");
        assert_eq!(info.exec_tree.to_string(), "((1 + 1) + $0.test)");
        assert_eq!(info.optimized.to_string(), "(2 + $0.test)");

        assert_eq!(
            info.to_string(),
            r#"{
    lexer: 1+1+`input`.`test`
    ast: ((1 + 1) + input.test)
    exec_tree: ((1 + 1) + $0.test)
    optimized: (2 + $0.test)
}"#
        );
    }

    #[test]
    fn test_unexpected_lambda() {
        let err = compile_err("float(a => a)", &[]);
        match err {
            CompileError::Build(BuildError::UnexpectedLambda(d)) => {
                assert_eq!(d.detail, "Expected expression, got lambda");
                assert_eq!(d.position, Span { start: 6, end: 12 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    fn test_unrecognized_function() {
        let err = compile_err("unknown_func(1)", &[]);
        match err {
            CompileError::Build(BuildError::UnrecognizedFunction(d)) => {
                assert_eq!(d.detail, "unknown_func");
                assert_eq!(d.position, Span { start: 0, end: 15 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    fn test_wrong_number_of_macro_args() {
        let err = compile_err(
            r#"
            #my_macro := (a, b) => a + b;
            my_macro(1)
        "#,
            &[],
        );
        match err {
            CompileError::Build(BuildError::NFunctionArgs(d)) => {
                assert_eq!(
                    d.detail,
                    "Incorrect number of function args: Expected 2 arguments to macro"
                );
                assert_eq!(d.position, Span { start: 55, end: 66 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    fn test_unexpected_lambda_in_macro() {
        let err = compile_err(
            r#"
            #my_macro := (a) => a + b;
            my_macro(a => a)
        "#,
            &[],
        );
        match err {
            CompileError::Build(BuildError::UnexpectedLambda(d)) => {
                assert_eq!(d.detail, "Expected expression, got lambda");
                assert_eq!(d.position, Span { start: 61, end: 67 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    fn test_recursion_depth() {
        let bad = "1 +".repeat(500) + " 1";
        let err = compile_err(&bad, &[]);
        match err {
            CompileError::Build(BuildError::Other(d)) => {
                assert_eq!(
                    d.detail,
                    "Recursion depth limit exceeded during compilation"
                );
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    fn test_duplicate_macro() {
        let err = compile_err(
            r#"
            #my_macro := (a) => a + 1;
            #my_macro := (a) => a * 2;
            my_macro(1)
        "#,
            &[],
        );
        match err {
            CompileError::Build(BuildError::Other(d)) => {
                assert_eq!(d.detail, "Duplicate macro definition");
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    fn test_invalid_input_variable() {
        let err = compile_err("1 + foo", &[]);
        match err {
            CompileError::Build(BuildError::UnknownVariable(d)) => {
                assert_eq!(d.detail, "foo");
                assert_eq!(d.position, Span { start: 4, end: 7 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    fn test_display_expression() {
        fn expr_matches(expr: &str, expected: &str) {
            let compiled = compile_expression(expr, &["input"]).unwrap();
            assert_eq!(
                compiled.to_string(),
                expected,
                "Expression did not format as expected. Got '{}', expected '{}'",
                compiled,
                expected
            );
        }

        expr_matches("1 + input", "(1 + $0)");
        expr_matches("[1, 2, ...input]", "[1, 2, ...$0]");
        expr_matches("{\"foo\": \"bar\", ...input}", "{\"foo\": \"bar\", ...$0}");
        expr_matches(
            "if input > 5 { 0 } else if input > 3 { 1 } else { 2 }",
            "if ($0 > 5) { 0 } else if ($0 > 3) { 1 } else { 2 }",
        );
        expr_matches("input is null", "$0 is null");
        expr_matches("input is not array", "$0 is not array");
        expr_matches("input is object", "$0 is object");
        expr_matches("input is number", "$0 is number");
        expr_matches("input is bool", "$0 is bool");
        expr_matches("input.map(a => a + 1)", "map($0, (a) => ($1 + 1))");
        expr_matches(
            r#"
        #foo := (a) => a + 1;
        foo(input)
        "#,
            "((a) => ($1 + 1))($0)",
        );
    }

    #[derive(Debug, serde::Deserialize)]
    struct TestRunConfig {
        /// List of input parameters for this test run
        inputs: Vec<serde_json::Value>,
        /// The expected output
        expected: serde_json::Value,
    }

    #[derive(Debug, serde::Deserialize, Default)]
    struct TestCaseConfig {
        /// List of input variable names
        pub inputs: Vec<String>,
        /// List of input/output pairs to test with
        pub cases: Option<Vec<TestRunConfig>>,
    }

    #[test]
    fn run_compile_tests() {
        let root_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let src_path = root_path.join("test_files");

        walkdir::WalkDir::new(src_path)
            .into_iter()
            .map(|f| f.expect("Failed to read directory entry"))
            .filter(|f| f.file_type().is_file())
            .map(|f| f.path().to_path_buf())
            .filter(|f| f.extension().is_some_and(|ext| ext == "kp"))
            .for_each(|test_case| {
                let raw_expression = std::fs::read_to_string(&test_case).expect(&format!(
                    "Failed to read test case file {}",
                    test_case.display()
                ));

                // Get first token, check if it is a comment. If it is, parse it as JSON and store as the run config.

                let mut tokens = Token::lexer(&raw_expression).spanned();
                let first = tokens.next();
                let config = first
                    .map(|res| match res {
                        (Ok(Token::Comment), span) => {
                            Some(raw_expression[span.start..span.end].to_string())
                        }
                        _ => None,
                    })
                    .flatten()
                    .map(|raw_config| {
                        // Remove comment markers before parsing
                        let config = raw_config
                            .trim_start_matches("//")
                            .trim_start_matches("/*")
                            .trim_end_matches("*/");

                        serde_json::from_str::<TestCaseConfig>(config).expect(&format!(
                            "Failed to parse config in file {}",
                            test_case.display()
                        ))
                    })
                    .unwrap_or_default();

                let inputs: Vec<&str> = config.inputs.iter().map(|s| s.as_str()).collect();
                let expression =
                    compile_expression_test(&raw_expression, &inputs).expect(&format!(
                        "Failed to compile expression in file {}",
                        test_case.display()
                    ));

                for (i, run_case) in config.cases.unwrap_or_default().iter().enumerate() {
                    let result = expression
                        .run(run_case.inputs.iter())
                        .expect(&format!(
                            "Failed to run expression in file {}",
                            test_case.display()
                        ))
                        .into_owned();

                    assert_eq!(
                        result,
                        run_case.expected,
                        "Test case {i} failed for file {}",
                        test_case.display(),
                    );
                }
                // Check that the expression is formatted correctly.
                let formatted = format_expression(&raw_expression).expect(&format!(
                    "Failed to format expression in file {}",
                    test_case.display()
                ));
                assert_eq!(
                    formatted,
                    raw_expression,
                    "Formatted expression does not match original in file {}",
                    test_case.display()
                );
            });
    }
}
