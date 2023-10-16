use serde_json::Value;

use crate::expressions::{base::get_string_from_cow_value, Expression, ResolveResult};

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
        state: &crate::expressions::ExpressionExecutionState<'c, '_>,
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
        state: &crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let dat = self.args[0].resolve(state)?;
        let val = dat.as_ref();
        let res = match val {
            Value::Null => "".to_string(),
            Value::Bool(x) => {
                if *x {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            Value::Number(x) => x.to_string(),
            Value::String(s) => s.to_string(),
            Value::Array(_) | Value::Object(_) => val.to_string(),
        };
        Ok(ResolveResult::Owned(Value::String(res)))
    }
}

// Once the function is defined it should be added to the main function enum in expressions/base.rs, and to the get_function_expression function.
// We can just add a test in this file:
#[cfg(test)]
mod tests {
    use serde_json::json;

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
}
