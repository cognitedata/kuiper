use serde_json::Value;

use crate::{
    compiler::BuildError,
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    TransformError,
};

function_def!(FilterFunction, "filter", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for FilterFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.into_owned() {
            Value::Array(x) => {
                let mut res = Vec::with_capacity(x.len());
                for item in x {
                    let should_add = self.args[1].call(state, &[&item])?.as_bool();

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
            )),
        }
    }
}

impl LambdaAcceptFunction for FilterFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
        _num_args: usize,
    ) -> Result<(), BuildError> {
        if idx != 1 {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }
        let nargs = lambda.input_names.len();
        if nargs != 1 {
            return Err(BuildError::n_function_args(
                lambda.span.clone(),
                "filter takes a function with one argument",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    pub fn test_simple_filter() {
        let expr = compile_expression("[1, 2, 3, 4, 5, 6].filter((i) => i >= 4)", &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(3, val_arr.len());
        assert_eq!(val_arr.first().unwrap().as_u64().unwrap(), 4);
        assert_eq!(val_arr.get(1).unwrap().as_u64().unwrap(), 5);
        assert_eq!(val_arr.get(2).unwrap().as_u64().unwrap(), 6);
    }
}
