use serde_json::Value;

use crate::{
    compiler::BuildError,
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    TransformError,
};

function_def!(MapFunction, "map", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for MapFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Array(x) => {
                let mut res = Vec::with_capacity(x.len());
                for (idx, val) in x.iter().enumerate() {
                    res.push(
                        self.args[1]
                            .call(state, &[val, &Value::Number(idx.into())])?
                            .into_owned(),
                    );
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to map",
                "array",
                TransformError::value_desc(x),
                &self.span,
            )),
        }
    }
}

impl LambdaAcceptFunction for MapFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
        _num_args: usize,
    ) -> Result<(), BuildError> {
        if idx != 1 {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }
        let nargs = lambda.input_names.len();
        if !(1..=2).contains(&nargs) {
            return Err(BuildError::n_function_args(
                lambda.span.clone(),
                "map takes a function with one or two arguments",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    pub fn test_simple_map() {
        let expr = compile_expression(r#"map([1, 2, 3, 4], (i) => pow(i, 2))"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(4, val_arr.len());
        assert_eq!(val_arr.first().unwrap().as_f64().unwrap(), 1.0);
        assert_eq!(val_arr.get(1).unwrap().as_f64().unwrap(), 4.0);
        assert_eq!(val_arr.get(2).unwrap().as_f64().unwrap(), 9.0);
        assert_eq!(val_arr.get(3).unwrap().as_f64().unwrap(), 16.0);
    }

    #[test]
    pub fn test_map_with_index() {
        let expr =
            compile_expression(r#"map(["a", "b", "c"], (it, index) => index)"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(3, val_arr.len());
        assert_eq!(0, val_arr.first().unwrap().as_u64().unwrap());
        assert_eq!(1, val_arr.get(1).unwrap().as_u64().unwrap());
        assert_eq!(2, val_arr.get(2).unwrap().as_u64().unwrap());
    }
}
