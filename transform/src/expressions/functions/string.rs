use serde_json::Value;

use crate::expressions::{
    base::{get_string_from_value, ReferenceOrValue},
    Expression,
};

// Example function definition

// Define a variadic function with type ConcatFunction, name "concat" in code, at least 2 arguments, any number of max arguments.
function_def!(ConcatFunction, "concat", 2, None);

// You need to implement Expression. The lifetime 'a refers to the source data we are working with.
impl<'a> Expression<'a> for ConcatFunction {
    fn resolve(
        &'a self,
        state: &'a crate::expressions::ExpressionExecutionState,
    ) -> Result<crate::expressions::ResolveResult<'a>, crate::TransformError> {
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
        Ok(ReferenceOrValue::Value(Value::String(res)))
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
}
