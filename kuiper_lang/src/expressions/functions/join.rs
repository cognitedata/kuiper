use serde_json::Value;

use crate::{
    expressions::{Expression, ResolveResult},
    TransformError,
};

function_def!(JoinFunction, "join", 2, None);

impl<'a: 'c, 'c> Expression<'a, 'c> for JoinFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.into_owned() {
            Value::Object(x) => {
                let mut res = x;
                for arg in self.args.iter().skip(1) {
                    let res_inner = arg.resolve(state)?;
                    let mut res_inner = res_inner.into_owned();
                    let value = match res_inner {
                        Value::Object(ref mut inner) => Ok(inner),
                        y => Err(TransformError::new_incorrect_type(
                            "Incorrect type provided for join",
                            "object",
                            TransformError::value_desc(&y),
                            &self.span,
                        )),
                    }?;
                    res.append(value);
                }
                Ok(ResolveResult::Owned(Value::Object(res)))
            }
            Value::Array(x) => {
                let mut res = x;
                for arg in self.args.iter().skip(1) {
                    let res_inner = arg.resolve(state)?;
                    let mut res_inner = res_inner.into_owned();
                    let value = match res_inner {
                        Value::Array(ref mut inner) => Ok(inner),
                        y => Err(TransformError::new_incorrect_type(
                            "Incorrect type provided for join",
                            "array",
                            TransformError::value_desc(&y),
                            &self.span,
                        )),
                    }?;
                    res.append(value);
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            x_val => Err(TransformError::new_incorrect_type(
                "Incorrect input to join",
                "object or array",
                TransformError::value_desc(&x_val),
                &self.span,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use logos::Span;

    use crate::{compile_expression, CompileError, TransformError};

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
    fn test_join_multiple() {
        let expr = compile_expression(r#"join({'a':1}, {'b': 2}, {'c': 3})"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();
        assert_eq!(val.len(), 3);
        assert_eq!(val.get("a").unwrap(), 1);
        assert_eq!(val.get("b").unwrap(), 2);
        assert_eq!(val.get("c").unwrap(), 3);
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
            Ok(_) => panic!("Should not be able to resolve"),
            Err(err) => match err {
                CompileError::Optimizer(TransformError::IncorrectTypeInField(t_err)) => {
                    assert_eq!(
                        t_err.desc,
                        "Incorrect type provided for join. Got array, expected object"
                    );
                    assert_eq!(t_err.span, Span { start: 0, end: 22 })
                }
                _ => panic!("Should be an optimizer error"),
            },
        }
    }

    #[test]
    fn test_join_arrays() {
        let expr = compile_expression(r#"join([1, 2, 3], [4, 5], [6, 7, 8])"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_array().unwrap();
        assert_eq!(val.len(), 8);

        for i in 0..val.len() {
            assert_eq!(val[i].as_u64().unwrap(), (i + 1) as u64);
        }
    }
}
