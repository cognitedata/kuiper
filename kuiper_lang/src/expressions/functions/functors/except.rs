use serde_json::Value;

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    BuildError, TransformError,
};

function_def!(ExceptFunction, "except", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for ExceptFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;
        let source = source.into_owned();
        match source {
            Value::Object(x) => {
                let mut output = x.to_owned();
                match &*self.args[1] {
                    crate::ExpressionType::Lambda(expr) => {
                        for (k, v) in x {
                            let should_remove = expr
                                .call(state, &[&v, &Value::String(k.to_owned())])?
                                .as_bool();
                            if should_remove {
                                output.remove(&k);
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
                                        Value::String(s) => {
                                            output.remove(s);
                                            Ok(())
                                        }
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
        if !(1..=2).contains(&nargs) {
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
    use logos::Span;

    use crate::{compile_expression, CompileError, TransformError};

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
            Ok(_) => panic!("Should not be able to resolve"),
            Err(err) => match err {
                CompileError::Optimizer(TransformError::IncorrectTypeInField(t_err)) => {
                    assert_eq!(
                        t_err.desc,
                        "Filter values should be of type string. Got number, expected string"
                    );
                    assert_eq!(t_err.span, Span { start: 0, end: 24 })
                }
                _ => panic!("Should be an optimizer error"),
            },
        }
    }
}
