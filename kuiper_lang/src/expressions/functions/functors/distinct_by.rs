use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    BuildError, TransformError,
};

function_def!(DistinctByFunction, "distinctBy", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for DistinctByFunction {
    fn resolve(
        &'a self,
        state: &crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Array(x) => {
                let mut res: Vec<Value> = Vec::new();
                let mut found: HashMap<String, bool> = HashMap::new();
                for val in x {
                    let res_inner = self.args[1].call(state, &[val])?.into_owned();
                    let by_value: String = match res_inner {
                        Value::Bool(b) => Ok(b.to_string()),
                        Value::Number(n) => Ok(n.to_string()),
                        Value::String(s) => Ok(s),
                        x => Err(TransformError::new_incorrect_type(
                            "Incorrect type returned by lambda",
                            "string, number, boolean",
                            TransformError::value_desc(&x),
                            &self.span,
                        )),
                    }?;
                    match found.get(&by_value) {
                        Some(_) => (),
                        None => {
                            res.push(val.to_owned());
                            found.insert(by_value, true);
                        }
                    }
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            Value::Object(x) => {
                let mut res: Map<String, Value> = Map::new();
                let mut found: HashMap<String, bool> = HashMap::new();
                for (k, v) in x {
                    let res_inner = self.args[1]
                        .call(state, &[v, &Value::String(k.to_owned())])?
                        .into_owned();
                    let by_value: String = match res_inner {
                        Value::Bool(b) => Ok(b.to_string()),
                        Value::Number(n) => Ok(n.to_string()),
                        Value::String(s) => Ok(s),
                        x => Err(TransformError::new_incorrect_type(
                            "Incorrect type returned by lambda",
                            "string, number, boolean",
                            TransformError::value_desc(&x),
                            &self.span,
                        )),
                    }?;
                    match found.get(&by_value) {
                        Some(_) => (),
                        None => {
                            res.insert(k.to_owned(), v.to_owned());
                            found.insert(by_value, true);
                        }
                    }
                }
                Ok(ResolveResult::Owned(Value::Object(res)))
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to distinctBy",
                "array or object",
                TransformError::value_desc(x),
                &self.span,
            )),
        }
    }
}

impl LambdaAcceptFunction for DistinctByFunction {
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
                "distictBy takes a function with one argument",
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
    fn test_distinct_by_fails_for_unknown_types() {
        match compile_expression(r#"distinctBy(1234567890, (a) => a)"#, &[]) {
            Ok(_) => assert!(false, "Should not be able to resolve"),
            Err(err) => {
                match err {
                    CompileError::Optimizer(TransformError::IncorrectTypeInField(t_err)) => {
                        assert_eq!(
                            t_err.desc,
                            "Incorrect input to distinctBy. Got number, expected array or object"
                        );
                        assert_eq!(t_err.span, Span { start: 0, end: 32 })
                    }
                    _ => assert!(false, "Should be an optimizer error"),
                }
                assert!(true);
            }
        }
    }

    #[test]
    fn test_distinct_by_for_arrays() {
        let expr =
            compile_expression(r#"distinctBy(["sheep", "apple", "sheep"], a => a)"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(val_arr.len(), 2);
        assert_eq!(val_arr.get(0).unwrap(), "sheep");
        assert_eq!(val_arr.get(1).unwrap(), "apple");
    }

    #[test]
    fn test_distinct_by_for_objects() {
        let expr = compile_expression(
            r#"distinctBy({'x': 'y', 'a': 'b', 'c': 'b'}, (a, b) => a)"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();
        assert_eq!(val.len(), 2);
        assert_eq!(val.get("x").unwrap().to_owned(), "y".to_string());
        assert_eq!(val.get("a").unwrap().to_owned(), "b".to_string());
    }
}
