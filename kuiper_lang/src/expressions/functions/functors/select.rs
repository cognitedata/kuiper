use serde_json::{Map, Value};

use crate::{
    expressions::{
        base::get_boolean_from_value, functions::LambdaAcceptFunction, Expression, ResolveResult,
    },
    BuildError, TransformError,
};

function_def!(SelectFunction, "select", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for SelectFunction {
    fn resolve(
        &'a self,
        state: &crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;
        let init_res = match self.args[1].call(state, &[]) {
            Ok(res) => match res.as_ref().to_owned() {
                Value::Array(val) => Some(val),
                _ => None,
            },
            Err(_) => None,
        };

        match source.as_ref() {
            Value::Object(x) => {
                let mut output = Map::new();
                match init_res {
                    Some(arr) => {
                        for f in arr {
                            let (should_add, k, v) = match f {
                                Value::String(k) => match x.get(&k) {
                                    Some(val) => Ok((true, k, val)),
                                    None => Ok((false, k, &Value::Null)),
                                },
                                x => Err(TransformError::new_incorrect_type(
                                    "Filter values should be of type string",
                                    "string",
                                    TransformError::value_desc(&x),
                                    &self.span,
                                )),
                            }?;
                            if should_add {
                                output.insert(k, v.to_owned());
                            }
                        }
                        Ok(ResolveResult::Owned(Value::Object(output)))
                    }
                    None => {
                        for (k, v) in x {
                            let should_add = get_boolean_from_value(
                                self.args[1]
                                    .call(state, &[v, &Value::String(k.to_owned())])?
                                    .as_ref(),
                            );
                            if should_add {
                                output.insert(k.to_owned(), v.to_owned());
                            }
                        }
                        Ok(ResolveResult::Owned(Value::Object(output)))
                    }
                }
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input given to except",
                "object",
                TransformError::value_desc(x),
                &self.span,
            )),
        }
    }
}

impl LambdaAcceptFunction for SelectFunction {
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
    fn test_select() {
        let expr =
            compile_expression(r#"select({'a': 1, 'b': 2, 'c': 3}, (v) => v > 1)"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();

        assert_eq!(val.len(), 2);
        assert_eq!(val.get("b").unwrap(), 2);
        assert_eq!(val.get("c").unwrap(), 3);
    }

    #[test]
    fn test_select_filter_by_key() {
        let expr = compile_expression(
            r#"select({'a': 1, 'b': 2, 'c': 3}, (_, k) => k != 'a')"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();

        assert_eq!(val.len(), 2);
        assert_eq!(val.get("b").unwrap(), 2);
        assert_eq!(val.get("c").unwrap(), 3);
    }

    #[test]
    fn test_select_fails_for_other_types() {
        match compile_expression(r#"select({'a':1}, ['a',2,3])"#, &[]) {
            Ok(_) => assert!(false, "Should not be able to resolve"),
            Err(_) => assert!(true),
        }
    }
}
