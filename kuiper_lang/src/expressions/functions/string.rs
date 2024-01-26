use serde_json::Value;

use crate::expressions::{
    base::{get_number_from_value, get_string_from_cow_value},
    Expression, ResolveResult,
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
impl<'a: 'c, 'c> Expression<'a, 'c> for ConcatFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        // Create a mutable string we can write to, in rust this is fast, a string is just Vec<u8>
        let mut res = String::new();
        // Iterate over the arguments to the function
        for expr in self.args.iter() {
            // Resolve each argument by passing the state, then return any errors if they occur.
            let resolved = expr.resolve(state)?;
            // Convert the value to string
            let dat = get_string_from_cow_value("concat", resolved, &self.span)?;
            // Push the resulting string to the result vector.
            res.push_str(&dat);
        }
        // Since we own the data we want to return here, return ResolveResult::Value. If we had built a reference
        // to a previous result (which itself might be a reference to input data!), we could have returned a reference here instead.
        Ok(ResolveResult::Owned(Value::String(res)))
    }
}

// other functions follow... This function converts the input to a string.
function_def!(StringFunction, "string", 1);

impl<'a: 'c, 'c> Expression<'a, 'c> for StringFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let dat = self.args[0].resolve(state)?;
        // let val = dat.as_ref();
        let res = match dat {
            ResolveResult::Owned(Value::Null) | ResolveResult::Borrowed(Value::Null) => {
                "".to_string()
            }
            ResolveResult::Owned(Value::Bool(ref x))
            | ResolveResult::Borrowed(Value::Bool(ref x)) => {
                if *x {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            ResolveResult::Owned(Value::Number(ref x))
            | ResolveResult::Borrowed(Value::Number(ref x)) => x.to_string(),
            ResolveResult::Owned(Value::String(s)) => s,
            ResolveResult::Borrowed(Value::String(s)) => s.to_owned(),
            x => x.as_ref().to_string(),
        };
        Ok(ResolveResult::Owned(Value::String(res)))
    }
}

function_def!(SubstringFunction, "substring", 2, Some(3));

impl<'a: 'c, 'c> Expression<'a, 'c> for SubstringFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let inp_value = self.args[0].resolve(state)?;
        let inp_string = get_string_from_cow_value("substring", inp_value, &self.span)?;
        let input = inp_string.as_ref();

        let start_value = self.args[1].resolve(state)?;
        let start =
            get_number_from_value("substring", &start_value, &self.span)?.try_as_i64(&self.span)?;

        let end_value: Option<Result<i64, crate::TransformError>> = self.args.get(2).map(|c| {
            let val = c.resolve(state)?;
            let end =
                get_number_from_value("substring", &val, &self.span)?.try_as_i64(&self.span)?;
            Ok(end)
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

impl<'a: 'c, 'c> Expression<'a, 'c> for SplitFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let inp_value = self.args[0].resolve(state)?;
        let inp_string = get_string_from_cow_value("split", inp_value, &self.span)?;

        let pat_value = self.args[1].resolve(state)?;
        let pat_string = get_string_from_cow_value("split", pat_value, &self.span)?;

        Ok(ResolveResult::Owned(Value::Array(
            inp_string
                .split(pat_string.as_ref())
                .map(|p| Value::String(p.to_string()))
                .collect(),
        )))
    }
}

function_def!(TrimWhitespace, "trim_whitespace", 1);

impl<'a: 'c, 'c> Expression<'a, 'c> for TrimWhitespace {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let inp_value = self.args[0].resolve(state)?;
        let inp_string = get_string_from_cow_value("split", inp_value, &self.span)?;

        Ok(ResolveResult::Owned(inp_string.trim().to_string().into()))
    }
}

function_def!(CharsFunction, "chars", 1);

impl<'a: 'c, 'c> Expression<'a, 'c> for CharsFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let inp_value = self.args[0].resolve(state)?;
        let inp_string = get_string_from_cow_value("split", inp_value, &self.span)?;

        let res = inp_string
            .chars()
            .map(|c| Value::String(c.to_string()))
            .collect();
        Ok(ResolveResult::Owned(res))
    }
}

// Once the function is defined it should be added to the main function enum in expressions/base.rs, and to the get_function_expression function.
// We can just add a test in this file:
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::compile_expression;

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
            .into_iter()
            .map(|v| v.as_str().unwrap())
            .collect();

        assert_eq!(vec!["t", "e", "s", "t", " ", "æ", "ø", "å"], arr);
    }
}
