use serde_json::Value;

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    with_temp_values, ParserError, TransformError,
};

function_def!(MapFunction, "map", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for MapFunction {
    fn resolve(
        &'a self,
        state: &crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Array(x) => {
                let mut res = Vec::with_capacity(x.len());
                let mut inner = state.get_temporary_clone(1);
                for val in x {
                    let r = with_temp_values!(inner, inner_state, &[val], {
                        self.args[1].resolve(&inner_state).map(|v| v.into_owned())
                    })?;
                    res.push(r);
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to map",
                "array",
                TransformError::value_desc(x),
                &self.span,
                state.id,
            )),
        }
    }
}

impl LambdaAcceptFunction for MapFunction {
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
                "map takes a function with one argument",
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
    pub fn test_simple_map() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "map",
                "inputs": [],
                "transform": r#"map([1, 2, 3, 4], (i) => pow(i, 2))"#
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program.execute(&Value::Null).unwrap();

        assert_eq!(res.len(), 1);
        let val = res.first().unwrap();
        let val_arr = val.as_array().unwrap();
        assert_eq!(4, val_arr.len());
        assert_eq!(val_arr.get(0).unwrap().as_f64().unwrap(), 1.0);
        assert_eq!(val_arr.get(1).unwrap().as_f64().unwrap(), 4.0);
        assert_eq!(val_arr.get(2).unwrap().as_f64().unwrap(), 9.0);
        assert_eq!(val_arr.get(3).unwrap().as_f64().unwrap(), 16.0);
    }
}
