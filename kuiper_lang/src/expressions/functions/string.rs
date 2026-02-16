use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;
use serde_json::Value;

use crate::{
    expressions::{Expression, ResolveResult},
    types::Type,
};

// Example function definition

// Define a variadic function with type ConcatFunction, name "concat" in code, at least 2 arguments, any number of max arguments.
function_def!(ConcatFunction, "concat", 2, None);

// You need to implement Expression.
// There are three lifetimes here. 'a refers to the transform itself, 'b is the current expression execution, 'c is the current program execution.
// The result must outlive the execution, but does not need to outlive the program execution.
// The expression itself of course lives longer than the data, so 'a: 'c, and the return type has lifetime equal to the
// input data, which is 'c. So we can return data with lifetime 'c or 'a.
//
// For functions, we usually compute a new result based on the child expressions, so we either return a reference with lifetime 'c,
// or an owned value, like we do here.
// In theory we could have a function like `pi()`, which returns the constant PI, this could return a reference with lifetime 'a.
// Typically returning references with lifetime 'a is used for constants, see Constant in base.rs.
impl Expression for ConcatFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        // Create a mutable string we can write to, in rust this is fast, a string is just Vec<u8>
        let mut res = String::new();
        // Iterate over the arguments to the function
        for expr in self.args.iter() {
            // Resolve each argument by passing the state, then return any errors if they occur.
            let resolved = expr.resolve(state)?;
            // Convert the value to string
            let dat = resolved.try_into_string("concat", &self.span)?;
            // Push the resulting string to the result vector.
            res.push_str(&dat);
        }
        // Since we own the data we want to return here, return ResolveResult::Value. If we had built a reference
        // to a previous result (which itself might be a reference to input data!), we could have returned a reference here instead.
        Ok(ResolveResult::Owned(Value::String(res)))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        // First, check that all arguments can be stringified using try_into_string
        for arg in &self.args {
            let res = arg.resolve_types(state)?;
            res.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        }

        // The result of concat is always a string, so we can just return that.
        Ok(crate::types::Type::String)
    }
}

// other functions follow... This function converts the input to a string.
function_def!(StringFunction, "string", 1);

impl Expression for StringFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let dat = self.args[0].resolve(state)?;
        Ok(ResolveResult::Owned(
            dat.try_into_string("string", &self.span)?
                .into_owned()
                .into(),
        ))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let res = self.args[0].resolve_types(state)?;
        res.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        Ok(Type::String)
    }
}

function_def!(ReplaceFunction, "replace", 3);

impl Expression for ReplaceFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let res = self.args[0]
            .resolve(state)?
            .try_into_string("replace", &self.span)?;
        let from = self.args[1]
            .resolve(state)?
            .try_into_string("replace", &self.span)?;
        let to = self.args[2]
            .resolve(state)?
            .try_into_string("replace", &self.span)?;
        let replaced = res.replace(from.as_ref(), to.as_ref());
        Ok(ResolveResult::Owned(Value::String(replaced)))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        for arg in &self.args {
            let res = arg.resolve_types(state)?;
            res.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        }
        Ok(Type::String)
    }
}

function_def!(SubstringFunction, "substring", 2, Some(3));

impl Expression for SubstringFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let inp_string = self.args[0]
            .resolve(state)?
            .try_into_string("substring", &self.span)?;
        let input = inp_string.as_ref();

        let start = self.args[1]
            .resolve(state)?
            .try_as_number("substring", &self.span)?
            .try_as_i64(&self.span)?;

        let end_value: Option<Result<i64, crate::TransformError>> = self.args.get(2).map(|c| {
            c.resolve(state)?
                .try_as_number("substring", &self.span)?
                .try_as_i64(&self.span)
        });
        let end = end_value.transpose()?;
        if end.is_some_and(|v| v == start) {
            return Ok(ResolveResult::Owned(Value::String(String::new())));
        }

        // Translate indices to proper byte indices
        let start = match get_byte_index(input, start) {
            Some(idx) => idx,
            None => {
                if start < 0 {
                    0
                } else {
                    return Ok(ResolveResult::Owned(Value::String(String::new())));
                }
            }
        };

        if let Some(end) = end.and_then(|end| get_byte_index(input, end)) {
            if end <= start {
                return Ok(ResolveResult::Owned(Value::String(String::new())));
            }

            Ok(ResolveResult::Owned(Value::String(
                input[start..end].to_string(),
            )))
        } else {
            Ok(ResolveResult::Owned(Value::String(
                input[start..].to_string(),
            )))
        }
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let input = self.args[0].resolve_types(state)?;
        input.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        let start = self.args[1].resolve_types(state)?;
        start.assert_assignable_to(&Type::number(), &self.span)?;

        if let Some(end) = self.args.get(2) {
            let end = end.resolve_types(state)?;
            end.assert_assignable_to(&Type::number(), &self.span)?;
        }

        Ok(Type::String)
    }
}
fn get_byte_index(str: &str, idx: i64) -> Option<usize> {
    if idx < 0 {
        str.char_indices()
            .rev()
            .nth((-idx - 1) as usize)
            .map(|v| v.0)
    } else {
        str.char_indices().nth(idx as usize).map(|v| v.0)
    }
}

function_def!(SplitFunction, "split", 2);

impl Expression for SplitFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let inp_string = self.args[0]
            .resolve(state)?
            .try_into_string("split", &self.span)?;

        let pat_string = self.args[1]
            .resolve(state)?
            .try_into_string("split", &self.span)?;

        Ok(ResolveResult::Owned(Value::Array(
            inp_string
                .split(pat_string.as_ref())
                .map(|p| Value::String(p.to_string()))
                .collect(),
        )))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        for arg in &self.args {
            let res = arg.resolve_types(state)?;
            res.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        }

        Ok(Type::array_of_type(Type::String))
    }
}

function_def!(TrimWhitespace, "trim_whitespace", 1);

impl Expression for TrimWhitespace {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let inp_string = self.args[0]
            .resolve(state)?
            .try_into_string("trim_whitespace", &self.span)?;

        Ok(ResolveResult::Owned(inp_string.trim().to_string().into()))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let input = self.args[0].resolve_types(state)?;
        input.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        Ok(Type::String)
    }
}

function_def!(CharsFunction, "chars", 1);

impl Expression for CharsFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let inp_string = self.args[0]
            .resolve(state)?
            .try_into_string("chars", &self.span)?;

        let res = inp_string
            .chars()
            .map(|c| Value::String(c.to_string()))
            .collect();
        Ok(ResolveResult::Owned(res))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let input = self.args[0].resolve_types(state)?;
        input.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        Ok(Type::array_of_type(Type::String))
    }
}

function_def!(StringJoinFunction, "string_join", 1, Some(2));

impl Expression for StringJoinFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let list = self.args[0].resolve(state)?;

        let sep = match self.args.get(1) {
            Some(s) => s
                .resolve(state)?
                .try_into_string("string_join", &self.span)?,
            None => Cow::Borrowed(""),
        };

        match list.as_ref() {
            Value::Array(arr) => Ok(ResolveResult::Owned(Value::String(
                arr.iter()
                    .map(|s| match s {
                        Value::String(val) => val.to_owned(),
                        _ => s.to_string(),
                    })
                    .join(sep.as_ref()),
            ))),

            wrong => Err(crate::TransformError::new_incorrect_type(
                "Incorrect input to string_join",
                "array",
                crate::TransformError::value_desc(wrong),
                &self.span,
            )),
        }
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let input = self.args[0].resolve_types(state)?;
        input.assert_assignable_to(&Type::array_of_type(Type::stringifyable()), &self.span)?;

        if let Some(sep) = self.args.get(1) {
            let sep = sep.resolve_types(state)?;
            sep.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        }

        Ok(Type::String)
    }
}

function_def!(StartsWithFunction, "starts_with", 2);

impl Expression for StartsWithFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let lh = self.args[0].resolve(state)?;
        let lh = lh.try_as_string("starts_with", &self.span)?;
        let rh = self.args[1].resolve(state)?;
        let rh = rh.try_as_string("starts_with", &self.span)?;

        Ok(lh.starts_with(rh.as_ref()).into())
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        for arg in &self.args {
            let res = arg.resolve_types(state)?;
            res.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        }
        Ok(Type::Boolean)
    }
}

function_def!(EndsWithFunction, "ends_with", 2);

impl Expression for EndsWithFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let lh = self.args[0].resolve(state)?;
        let lh = lh.try_as_string("ends_with", &self.span)?;
        let rh = self.args[1].resolve(state)?;
        let rh = rh.try_as_string("ends_with", &self.span)?;

        Ok(lh.ends_with(rh.as_ref()).into())
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        for arg in &self.args {
            let res = arg.resolve_types(state)?;
            res.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        }
        Ok(Type::Boolean)
    }
}

function_def!(LowerFunction, "lower", 1);

impl Expression for LowerFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let inp_string = self.args[0]
            .resolve(state)?
            .try_into_string("lower", &self.span)?;

        Ok(ResolveResult::Owned(Value::String(
            inp_string.to_lowercase(),
        )))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let input = self.args[0].resolve_types(state)?;
        input.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        Ok(Type::String)
    }
}

function_def!(UpperFunction, "upper", 1);

impl Expression for UpperFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let inp_string = self.args[0]
            .resolve(state)?
            .try_into_string("upper", &self.span)?;

        Ok(ResolveResult::Owned(Value::String(
            inp_string.to_uppercase(),
        )))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let input = self.args[0].resolve_types(state)?;
        input.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        Ok(Type::String)
    }
}

function_def!(TranslateFunction, "translate", 3);

impl Expression for TranslateFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let inp_string = self.args[0].resolve(state)?;
        let inp_string = inp_string.try_as_string("translate", &self.span)?;

        let from = self.args[1].resolve(state)?;
        let from = from.try_as_string("translate", &self.span)?;

        let to = self.args[2].resolve(state)?;
        let to = to.try_as_string("translate", &self.span)?;

        let mut map = HashMap::new();
        for pair in from.chars().zip_longest(to.chars()) {
            match pair {
                itertools::EitherOrBoth::Both(from, to) => {
                    map.entry(from).or_insert(to);
                }
                _ => {
                    return Err(crate::TransformError::new_invalid_operation(
                        "In translate, 'from' and 'to' must have the same number of characters"
                            .to_owned(),
                        &self.span,
                    ))
                }
            }
        }

        let result = inp_string
            .chars()
            .map(|c| map.get(&c).cloned().unwrap_or(c))
            .collect();

        Ok(ResolveResult::Owned(Value::String(result)))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        for arg in &self.args {
            let res = arg.resolve_types(state)?;
            res.assert_assignable_to(&Type::stringifyable(), &self.span)?;
        }
        Ok(Type::String)
    }
}

// Once the function is defined it should be added to the main function enum in expressions/base.rs, and to the get_function_expression function.
// We can just add a test in this file:
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::{compile_expression, types::Type};

    #[test]
    pub fn test_concat() {
        let expr = compile_expression(
            r#"{
            "concat2": concat('foo', 'bar'),
            "concat3": concat('foo', input.val1 + input.val2, 'bar')
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({
            "val1": 100,
            "val2": 23
        });
        let res = expr.run([&inp]).unwrap();
        assert_eq!("foobar", res.get("concat2").unwrap().as_str().unwrap());
        assert_eq!("foo123bar", res.get("concat3").unwrap().as_str().unwrap());
    }

    #[test]
    pub fn test_string_function() {
        let expr = compile_expression(
            r#"{
            "s1": string('foo'),
            "s2": string(123),
            "s3": string(null),
            "s4": string(input.val)
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({
            "val": 123.123
        });
        let res = expr.run([&inp]).unwrap();

        assert_eq!("foo", res.get("s1").unwrap().as_str().unwrap());
        assert_eq!("123", res.get("s2").unwrap().as_str().unwrap());
        assert_eq!("", res.get("s3").unwrap().as_str().unwrap());
        assert_eq!("123.123", res.get("s4").unwrap().as_str().unwrap());
    }

    #[test]
    pub fn test_substring_function() {
        let expr = compile_expression(
            r#"{
            "s1": "test".substring(2),
            "s2": "test".substring(2, 3),
            "s3": "string".substring(15, 16),
            "s4": "string".substring(2, 2),
            "s5": "string".substring(2, 0),
            "s6": "string".substring(15),
            "s7": "æææææææ".substring(3),
            "s8": "string".substring(2, 15),
            "s9": "string".substring(-3, -2),
            "s10": "string".substring(-4),
            "s11": "string".substring(0, -2),
            "s12": "string".substring(0, -1),
            "s13": "string".substring(-15)
            }"#,
            &[],
        )
        .unwrap();

        let res = expr.run(&[]).unwrap();

        assert_eq!("st", res.get("s1").unwrap().as_str().unwrap());
        assert_eq!("s", res.get("s2").unwrap().as_str().unwrap());
        assert_eq!("", res.get("s3").unwrap().as_str().unwrap());
        assert_eq!("", res.get("s4").unwrap().as_str().unwrap());
        assert_eq!("", res.get("s5").unwrap().as_str().unwrap());
        assert_eq!("", res.get("s6").unwrap().as_str().unwrap());
        assert_eq!("ææææ", res.get("s7").unwrap().as_str().unwrap());
        assert_eq!("ring", res.get("s8").unwrap().as_str().unwrap());
        assert_eq!("i", res.get("s9").unwrap().as_str().unwrap());
        assert_eq!("ring", res.get("s10").unwrap().as_str().unwrap());
        assert_eq!("stri", res.get("s11").unwrap().as_str().unwrap());
        assert_eq!("strin", res.get("s12").unwrap().as_str().unwrap());
        assert_eq!("string", res.get("s13").unwrap().as_str().unwrap());
    }

    #[test]
    pub fn test_split_function() {
        let expr = compile_expression(
            r#"{
            "s1": "test some words".split(" "),
            "s2": "test".split(""),
            "s3": "testwowtestwowtest".split("wow"),
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run(&[]).unwrap();

        assert_eq!(
            &Value::Array(vec![
                "test".to_string().into(),
                "some".to_string().into(),
                "words".to_string().into()
            ]),
            res.get("s1").unwrap()
        );
        assert_eq!(
            &Value::Array(vec![
                "".to_string().into(),
                "t".to_string().into(),
                "e".to_string().into(),
                "s".to_string().into(),
                "t".to_string().into(),
                "".to_string().into(),
            ]),
            res.get("s2").unwrap()
        );
        assert_eq!(
            &Value::Array(vec![
                "test".to_string().into(),
                "test".to_string().into(),
                "test".to_string().into()
            ]),
            res.get("s3").unwrap()
        );
    }

    #[test]
    pub fn test_trim_function() {
        let expr = compile_expression(
            r#"{
            "s1": "test".trim_whitespace(),
            "s2": "   test   ".trim_whitespace(),
            "s3": "

            test

            ".trim_whitespace()
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run(&[]).unwrap();

        assert_eq!("test", res.get("s1").unwrap().as_str().unwrap());
        assert_eq!("test", res.get("s2").unwrap().as_str().unwrap());
        assert_eq!("test", res.get("s3").unwrap().as_str().unwrap());
    }

    #[test]
    pub fn test_replace_function() {
        let expr = compile_expression(
            r#"{
            "s1": "test_potato".replace("potato","tomato"),
            "s2": " ".replace(" ","tomato"),
            "s3": replace("potato","o","a")
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run(&[]).unwrap();

        assert_eq!("test_tomato", res.get("s1").unwrap().as_str().unwrap());
        assert_eq!("tomato", res.get("s2").unwrap().as_str().unwrap());
        assert_eq!("patata", res.get("s3").unwrap().as_str().unwrap());
    }

    #[test]
    pub fn test_chars_function() {
        let expr = compile_expression(
            r#"
        "test æøå".chars()
        "#,
            &[],
        )
        .unwrap();

        let res = expr.run(&[]).unwrap();

        let arr: Vec<_> = res
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();

        assert_eq!(vec!["t", "e", "s", "t", " ", "æ", "ø", "å"], arr);
    }

    #[test]
    pub fn test_join() {
        let expr = compile_expression(
            r#"
        {
            "test1": ["hello", "there"].string_join(" "),
            "test2": [1, 2, 3].string_join()
        }
        "#,
            &[],
        )
        .unwrap();

        let res = expr.run(&[]).unwrap();

        assert_eq!(res.get("test1").unwrap().as_str().unwrap(), "hello there");
        assert_eq!(res.get("test2").unwrap().as_str().unwrap(), "123");
    }

    #[test]
    pub fn test_starts_with() {
        let expr = compile_expression(
            r#"
            {
                "t1": "test".starts_with("tes"),
                "t2": "foo".starts_with("bar"),
                "t3": "test".starts_with("test")
            }
            "#,
            &[],
        )
        .unwrap();

        let res = expr.run(&[]).unwrap();
        assert_eq!(res.get("t1").unwrap().as_bool().unwrap(), true);
        assert_eq!(res.get("t2").unwrap().as_bool().unwrap(), false);
        assert_eq!(res.get("t3").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    pub fn test_ends_with() {
        let expr = compile_expression(
            r#"
            {
                "t1": "test".ends_with("est"),
                "t2": "foo".ends_with("bar"),
                "t3": "test".ends_with("test")
            }
            "#,
            &[],
        )
        .unwrap();

        let res = expr.run(&[]).unwrap();
        assert_eq!(res.get("t1").unwrap().as_bool().unwrap(), true);
        assert_eq!(res.get("t2").unwrap().as_bool().unwrap(), false);
        assert_eq!(res.get("t3").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    fn test_lower() {
        let expr = compile_expression(
            r#"
        {
            "t1": "TEST".lower(),
            "t2": "TeSt123".lower(),
            "t3": "test".lower(),
            "t4": "tëßt".lower(),
            "t5": 123.lower(),
            "t6": true.lower(),
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run(&[]).unwrap();
        assert_eq!(res.get("t1").unwrap().as_str().unwrap(), "test");
        assert_eq!(res.get("t2").unwrap().as_str().unwrap(), "test123");
        assert_eq!(res.get("t3").unwrap().as_str().unwrap(), "test");
        assert_eq!(res.get("t4").unwrap().as_str().unwrap(), "tëßt");
        assert_eq!(res.get("t5").unwrap().as_str().unwrap(), "123");
        assert_eq!(res.get("t6").unwrap().as_str().unwrap(), "true");
    }

    #[test]
    fn test_upper() {
        let expr = compile_expression(
            r#"
        {
            "t1": "TEST".upper(),
            "t2": "TeSt123".upper(),
            "t3": "test".upper(),
            "t4": "tëßt".upper(),
            "t5": 123.upper(),
            "t6": true.upper(),
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run(&[]).unwrap();
        assert_eq!(res.get("t1").unwrap().as_str().unwrap(), "TEST");
        assert_eq!(res.get("t2").unwrap().as_str().unwrap(), "TEST123");
        assert_eq!(res.get("t3").unwrap().as_str().unwrap(), "TEST");
        assert_eq!(res.get("t4").unwrap().as_str().unwrap(), "TËSST");
        assert_eq!(res.get("t5").unwrap().as_str().unwrap(), "123");
        assert_eq!(res.get("t6").unwrap().as_str().unwrap(), "TRUE");
    }

    #[test]
    fn test_translate() {
        let expr = compile_expression(
            r#"
        {
            "t1": "hello".translate("ho","jy"),
            "t2": "hello world".translate("helowr","HELOWR"),
            "t3": "háøøææ".translate("áõøæ","aooa"),
            "t4": "hello".translate("hee", "hij"), // first match is prefered.
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run(&[]).unwrap();
        assert_eq!(res.get("t1").unwrap().as_str().unwrap(), "jelly");
        assert_eq!(res.get("t2").unwrap().as_str().unwrap(), "HELLO WORLd");
        assert_eq!(res.get("t3").unwrap().as_str().unwrap(), "haooaa");
        assert_eq!(res.get("t4").unwrap().as_str().unwrap(), "hillo");
    }

    #[test]
    fn test_concat_types() {
        let expr = compile_expression("concat(input1, input2)", &["input1", "input2"]).unwrap();
        let ty = expr.run_types([Type::Integer, Type::String]).unwrap();
        assert_eq!(ty, Type::String);

        assert!(expr
            .run_types([Type::String, Type::array_of_type(Type::Boolean)])
            .is_err());
    }

    #[test]
    fn test_string_types() {
        let expr = compile_expression("string(input)", &["input"]).unwrap();
        let ty = expr.run_types([Type::Integer]).unwrap();
        assert_eq!(ty, Type::String);

        let ty = expr.run_types([Type::null()]).unwrap();
        assert_eq!(ty, Type::String);

        let ty = expr.run_types([Type::Boolean]).unwrap();
        assert_eq!(ty, Type::String);

        assert!(expr
            .run_types([Type::array_of_type(Type::Boolean)])
            .is_err());
    }

    #[test]
    fn test_replace_types() {
        let expr = compile_expression("replace(input, 'a', 'b')", &["input"]).unwrap();
        let ty = expr.run_types([Type::Integer]).unwrap();
        assert_eq!(ty, Type::String);

        assert!(expr
            .run_types([Type::array_of_type(Type::Boolean)])
            .is_err());
    }

    #[test]
    fn test_substring_types() {
        let expr = compile_expression(
            "substring(input, inpu2, input3)",
            &["input", "inpu2", "input3"],
        )
        .unwrap();
        let ty = expr
            .run_types([Type::Integer, Type::Integer, Type::Integer])
            .unwrap();
        assert_eq!(ty, Type::String);

        assert!(expr
            .run_types([
                Type::array_of_type(Type::Boolean),
                Type::Integer,
                Type::Integer
            ])
            .is_err());
        assert!(expr
            .run_types([Type::String, Type::String, Type::Integer])
            .is_err());
    }

    #[test]
    fn test_split_types() {
        let expr = compile_expression("split(input, ' ')", &["input"]).unwrap();
        let ty = expr.run_types([Type::String]).unwrap();
        assert_eq!(ty, Type::array_of_type(Type::String));
    }

    #[test]
    fn test_trim_whitespace_types() {
        let expr = compile_expression("trim_whitespace(input)", &["input"]).unwrap();
        let ty = expr.run_types([Type::String]).unwrap();
        assert_eq!(ty, Type::String);
    }

    #[test]
    fn test_chars_types() {
        let expr = compile_expression("chars(input)", &["input"]).unwrap();
        let ty = expr.run_types([Type::String]).unwrap();
        assert_eq!(ty, Type::array_of_type(Type::String));
    }

    #[test]
    fn test_string_join_types() {
        let expr = compile_expression("string_join(input, ' ')", &["input"]).unwrap();
        let ty = expr.run_types([Type::array_of_type(Type::String)]).unwrap();
        assert_eq!(ty, Type::String);
    }

    #[test]
    fn test_starts_ends_with_types() {
        for func in ["starts_with", "ends_with"] {
            let expr = compile_expression(&format!("{}(input, 'test')", func), &["input"]).unwrap();
            let ty = expr.run_types([Type::String]).unwrap();
            assert_eq!(ty, Type::Boolean);
        }
    }

    #[test]
    fn test_lower_upper_types() {
        for func in ["lower", "upper"] {
            let expr = compile_expression(&format!("{}(input)", func), &["input"]).unwrap();
            let ty = expr.run_types([Type::String]).unwrap();
            assert_eq!(ty, Type::String);
        }
    }

    #[test]
    fn test_translate_types() {
        let expr = compile_expression("translate(input, 'abc', 'def')", &["input"]).unwrap();
        let ty = expr.run_types([Type::String]).unwrap();
        assert_eq!(ty, Type::String);
    }
}
