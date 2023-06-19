use serde_json::Value;

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    BuildError, TransformError,
};

function_def!(JoinFunction, "join", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for JoinFunction {
    fn resolve(
        &'a self,
        state: &crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Object(x) => {
                let res_inner = self.args[1].call(state, &[source.as_ref()])?.to_owned();
                let mut x = x.to_owned();
                match res_inner.as_ref() {
                    Value::Object(inner) => {
                        for (key, val) in inner {
                            x.insert(key.to_owned(), val.to_owned());
                        }
                        Ok(ResolveResult::Owned(Value::Object(x)))
                    }
                    x => Err(TransformError::new_incorrect_type(
                        "Incorrect type provided for join",
                        "object",
                        TransformError::value_desc(x),
                        &self.span,
                    )),
                }
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to join",
                "object",
                TransformError::value_desc(x),
                &self.span,
            )),
        }
    }
}

impl LambdaAcceptFunction for JoinFunction {
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
                "join takes a function with one argument",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    fn test_join() {
        let expr = compile_expression(r#"join({'a': 1}, {'b': 2})"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();

        assert_eq!(val.len(), 2);
        assert_eq!(val.get("a").unwrap(), 1);
        assert_eq!(val.get("b").unwrap(), 2);
    }

    #[test]
    fn test_join_overwrites() {
        let expr = compile_expression(r#"join({'a':1}, {'a': 2})"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();
        assert_eq!(val.len(), 1);
        assert_eq!(val.get("a").unwrap(), 2);
    }

    #[test]
    fn test_join_fails_for_other_types() {
        match compile_expression(r#"join({'a':1}, [1,2,3])"#, &[]) {
            Ok(_) => assert!(false, "Should not be able to resolve"),
            Err(_) => assert!(true),
        }
    }
}
