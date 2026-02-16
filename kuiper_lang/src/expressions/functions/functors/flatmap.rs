use serde_json::Value;

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    BuildError, TransformError,
};

function_def!(FlatMapFunction, "flatmap", 2, lambda);

impl Expression for FlatMapFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<crate::expressions::ResolveResult<'a>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Array(x) => {
                let mut res = Vec::with_capacity(x.len());
                for val in x {
                    let res_inner = self.args[1].call(state, &[val])?.into_owned();
                    match res_inner {
                        Value::Array(y) => {
                            for item in y {
                                res.push(item);
                            }
                        }
                        _x => res.push(_x),
                    };
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to flatmap",
                "array",
                TransformError::value_desc(x),
                &self.span,
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
                "flatmap takes a function with one argument",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    fn test_flatmap() {
        let expr = compile_expression(r#"flatmap([1,2,3], a => [a + a])"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(val_arr.len(), 3);
        assert_eq!(val_arr.first().unwrap(), 2);
        assert_eq!(val_arr.get(1).unwrap(), 4);
        assert_eq!(val_arr.get(2).unwrap(), 6);
    }

    #[test]
    fn test_flatmap_where_include_single() {
        let expr = compile_expression(r#"flatmap([1,2,3, [4, 5]], a => a)"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(val_arr.len(), 5);
        assert_eq!(val_arr.first().unwrap(), 1);
        assert_eq!(val_arr.get(1).unwrap(), 2);
        assert_eq!(val_arr.get(2).unwrap(), 3);
        assert_eq!(val_arr.get(3).unwrap(), 4);
        assert_eq!(val_arr.get(4).unwrap(), 5);
    }
}
