use serde_json::Value;

use crate::{
    expressions::{
        base::get_boolean_from_value, functions::LambdaAcceptFunction, Expression, ResolveResult,
    },
    ParserError, TransformError,
};

function_def!(FilterFunction, "filter", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for FilterFunction {
    fn resolve(
        &'a self,
        state: &crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.into_owned() {
            Value::Array(x) => {
                let mut res = Vec::with_capacity(x.len());
                for item in x {
                    let should_add = {
                        let chunk = &[&item];
                        let inner = state.get_temporary_clone_inner(chunk.iter().copied(), 1);
                        let inner_state = inner.get_temp_state();
                        get_boolean_from_value(self.args[1].resolve(&inner_state)?.as_ref())
                    };

                    if should_add {
                        res.push(item);
                    }
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to filter",
                "array",
                TransformError::value_desc(&x),
                &self.span,
                state.id,
            )),
        }
    }
}

impl LambdaAcceptFunction for FilterFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
        _num_args: usize,
    ) -> Result<(), crate::ParserError> {
        if idx != 1 {
            return Err(crate::ParserError::unexpected_lambda(&lambda.span));
        }
        let nargs = lambda.input_names.len();
        if nargs != 1 {
            return Err(ParserError::n_function_args(
                lambda.span.clone(),
                "filter takes a function with one argument",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::Program;

    #[test]
    pub fn test_simple_filter() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "map",
                "inputs": [],
                "transform": "[1, 2, 3, 4, 5, 6].filter((i) => i >= 4)"
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program.execute(&Value::Null).unwrap();

        assert_eq!(res.len(), 1);
        let val = res.first().unwrap();
        let val_arr = val.as_array().unwrap();
        assert_eq!(3, val_arr.len());
        assert_eq!(val_arr.get(0).unwrap().as_u64().unwrap(), 4);
        assert_eq!(val_arr.get(1).unwrap().as_u64().unwrap(), 5);
        assert_eq!(val_arr.get(2).unwrap().as_u64().unwrap(), 6);
    }
}
