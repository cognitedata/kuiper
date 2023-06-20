use serde_json::Value;

use crate::{
    expressions::{
        base::get_boolean_from_value, functions::LambdaAcceptFunction, Expression, ResolveResult,
    },
    BuildError, TransformError,
};

function_def!(ExceptFunction, "except", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for ExceptFunction {
    fn resolve(
        &'a self,
        state: &crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let mut source = self.args[0].resolve(state)?;
        let source = source.to_mut().to_owned();
        match source {
            Value::Object(x) => {
                let mut output = x.to_owned();
                match &*self.args[1] {
                    crate::ExpressionType::Lambda(expr) => {
                        for (k, v) in x {
                            let should_remove = get_boolean_from_value(
                                expr.call(state, &[&v, &Value::String(k.to_owned())])?
                                    .as_ref(),
                            );
                            if should_remove {
                                output.remove(&k.to_owned());
                            }
                        }
                        Ok(ResolveResult::Owned(Value::Object(output)))
                    }
                    expr => {
                        let res = expr.resolve(state)?;
                        match res.as_ref() {
                            Value::Array(arr) => {
                                for f in arr {
                                    match f {
                                        Value::String(s) => match output.remove(s) {
                                            Some(_) => Ok(()),
                                            None => Ok(()),
                                        },
                                        x => Err(TransformError::new_incorrect_type(
                                            "Filter values should be of type string",
                                            "string",
                                            TransformError::value_desc(x),
                                            &self.span,
                                        )),
                                    }?
                                }
                                Ok(ResolveResult::Owned(Value::Object(output)))
                            }
                            x => Err(TransformError::new_incorrect_type(
                                "Incorrect input passed as second argument to except",
                                "array, lambda",
                                TransformError::value_desc(x),
                                &self.span,
                            )),
                        }
                    }
                }
            }
            x => Err(TransformError::new_incorrect_type(
                "The first argument to except should be an object",
                "object",
                TransformError::value_desc(&x),
                &self.span,
            )),
        }
    }
}

impl LambdaAcceptFunction for ExceptFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
        _num_args: usize,
    ) -> Result<(), BuildError> {
        if idx != 1 {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }
        let nargs = lambda.input_names.len();
        if nargs > 2 {
            return Err(BuildError::n_function_args(
                lambda.span.clone(),
                "except takes a function with a maximum of 2 argument",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    fn test_except() {
        let expr =
            compile_expression(r#"except({'a': 1, 'b': 2, 'c': 3}, (v) => v > 1)"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();

        assert_eq!(val.len(), 1);
        assert_eq!(val.get("a").unwrap(), 1);
    }

    #[test]
    fn test_except_filter_by_key() {
        let expr = compile_expression(
            r#"except({'a': 1, 'b': 2, 'c': 3}, (_, k) => k != 'a')"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();

        assert_eq!(val.len(), 1);
        assert_eq!(val.get("a").unwrap(), 1);
    }

    #[test]
    fn test_except_fails_for_other_types() {
        match compile_expression(r#"except({'a':1}, [1,2,3])"#, &[]) {
            Ok(_) => assert!(false, "Should not be able to resolve"),
            Err(_) => assert!(true),
        }
    }
}
