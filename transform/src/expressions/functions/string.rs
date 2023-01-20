use serde_json::Value;

use crate::expressions::{base::get_string_from_value, Expression, ResolveResult};

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
impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for ConcatFunction {
    fn resolve(
        &'a self,
        state: &'b crate::expressions::ExpressionExecutionState<'c, 'b>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        // Create a mutable string we can write to, in rust this is fast, a string is just Vec<u8>
        let mut res = "".to_string();
        // Iterate over the arguments to the function
        for expr in self.args.iter() {
            // Resolve each argument by passing the state, then return any errors if they occur.
            let resolved = expr.resolve(state)?;
            // Convert the value to string
            let dat = get_string_from_value("concat", resolved.as_ref(), &self.span, state.id)?;
            // Push the resulting string to the result vector.
            res.push_str(dat.as_ref());
        }
        // Since we own the data we want to return here, return ResolveResult::Value. If we had built a reference
        // to a previous result (which itself might be a reference to input data!), we could have returned a reference here instead.
        Ok(ResolveResult::Value(Value::String(res)))
    }
}

// other functions follow... This function converts the input to a string.
function_def!(StringFunction, "string", 1);

impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for StringFunction {
    fn resolve(
        &'a self,
        state: &'b crate::expressions::ExpressionExecutionState<'c, 'b>,
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
        Ok(ResolveResult::Value(Value::String(res)))
    }
}

// Once the function is defined it should be added to the main function enum in expressions/base.rs, and to the get_function_expression function.
// We can just add a test in this file:
#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::Program;

    #[test]
    pub fn test_concat() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "concat",
                "inputs": ["input"],
                "transform": {
                    "concat2": "concat('foo', 'bar')",
                    "concat3": "concat('foo', $input.val1 + $input.val2, 'bar')"
                },
                "type": "map"
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program
            .execute(&json!({
                "val1": 100,
                "val2": 23
            }))
            .unwrap();
        assert_eq!(res.len(), 1);
        let val = res.get(0).unwrap();
        assert_eq!("foobar", val.get("concat2").unwrap().as_str().unwrap());
        assert_eq!("foo123bar", val.get("concat3").unwrap().as_str().unwrap());
    }

    #[test]
    pub fn test_string_function() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "tostring",
                "inputs": ["input"],
                "transform": {
                    "s1": "string('foo')",
                    "s2": "string(123)",
                    "s3": "string(null)",
                    "s4": "string($input.val)"
                },
                "type": "map"
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program
            .execute(&json!({
                "val": 123.123
            }))
            .unwrap();

        assert_eq!(res.len(), 1);
        let val = res.first().unwrap();
        assert_eq!("foo", val.get("s1").unwrap().as_str().unwrap());
        assert_eq!("123", val.get("s2").unwrap().as_str().unwrap());
        assert_eq!("", val.get("s3").unwrap().as_str().unwrap());
        assert_eq!("123.123", val.get("s4").unwrap().as_str().unwrap());
    }
}
