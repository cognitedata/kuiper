use serde_json::Value;

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    BuildError, TransformError,
};

function_def!(FlatMapFunction, "flatmap", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for FlatMapFunction {
    fn resolve(
        &'a self,
        state: &crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Array(x) => {
                let mut res = Vec::with_capacity(x.len());
                for val in x {
                    let res_inner = self.args[1].call(state, &[val])?.into_owned();
                    match res_inner {
                        Value::Array(y) => for item in y {
                            res.push(item);
                        },
                        _ => res.push(res_inner),
                    };
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

impl LambdaAcceptFunction for FlatMapFunction {
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
                "map takes a function with one argument",
            ));
        }
        Ok(())
    }
}
