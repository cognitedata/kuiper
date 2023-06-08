use crate::expressions::functions::LambdaAcceptFunction;
use crate::expressions::{Expression, ExpressionExecutionState, ResolveResult};
use crate::{BuildError, TransformError};
use serde_json::Value;

function_def!(ReduceFunction, "reduce", 3, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for ReduceFunction {
    fn resolve(
        &'a self,
        state: &ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Array(xs) => {
                let mut value = self.args[2].resolve(state)?.clone();
                for x in xs {
                    let res = self.args[1].call(state, &[value.as_ref(), x])?;
                    value = res.clone();
                }
                Ok(value)
            }

            non_array => Err(TransformError::new_incorrect_type(
                "Incorrect input to reduce",
                "array",
                TransformError::value_desc(non_array),
                &self.span,
            )),
        }
    }
}

impl LambdaAcceptFunction for ReduceFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
        _num_args: usize,
    ) -> Result<(), BuildError> {
        if idx != 1 {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }
        let nargs = lambda.input_names.len();
        if nargs != 2 {
            return Err(BuildError::n_function_args(
                lambda.span.clone(),
                "reduce takes a function with two arguments",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    pub fn test_reduce_strings() {
        let expr = compile_expression(r#"['a', 'b', 'c'].reduce((a, b) => concat(a, b), '')"#, &[])
            .unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_str().unwrap();
        assert_eq!(val_arr, "abc");
    }

    #[test]
    pub fn test_reduce_numbers() {
        let expr = compile_expression(r#"[1, 2, 3, 4].reduce((a, b) => a+b, 0)"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_i64().unwrap();
        assert_eq!(val_arr, 10);
    }
}
