use serde_json::{Map, Value};

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    BuildError, TransformError,
};

function_def!(ToObjectFunction, "to_object", 2, Some(3), lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for ToObjectFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        let Value::Array(arr) = source.as_ref() else {
            return Err(TransformError::new_incorrect_type(
                "Incorrect input to to_object",
                "array",
                TransformError::value_desc(source.as_ref()),
                &self.span,
            ));
        };

        let mut res = Map::with_capacity(arr.len());
        for elem in arr {
            let key = self.args[1].call(state, &[elem])?;
            let key = key.try_into_string("to_object", &self.span)?.into_owned();

            let value = if let Some(value_lambda) = self.args.get(2) {
                value_lambda.call(state, &[elem])?.into_owned()
            } else {
                elem.clone()
            };
            res.insert(key, value);
        }

        Ok(ResolveResult::Owned(Value::Object(res)))
    }
}

impl LambdaAcceptFunction for ToObjectFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
        _num_args: usize,
    ) -> Result<(), crate::BuildError> {
        if idx == 0 {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }

        if lambda.input_names.len() != 1 {
            return Err(BuildError::n_function_args(
                lambda.span.clone(),
                "to_object takes one or two lambdas with exactly one argument each",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    pub fn test_to_object_implicit_value() {
        let expr = compile_expression(
            r#"
            [{"test": 1, "key": "v1"}, {"test2": 2, "key": "v2"}]
                .to_object(v => v.key)
        "#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val_obj = res.as_object().unwrap();
        assert_eq!(2, val_obj.len());
        assert_eq!(
            1,
            val_obj["v1"].as_object().unwrap()["test"].as_i64().unwrap()
        );
        assert_eq!(
            2,
            val_obj["v2"].as_object().unwrap()["test2"]
                .as_i64()
                .unwrap()
        );
    }

    #[test]
    pub fn test_to_object() {
        let expr = compile_expression(
            r#"
            [{"test": 1, "key": "v1"}, {"test": 2, "key": "v2"}]
                .to_object(v => v.key, v => v.test)
        "#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val_obj = res.as_object().unwrap();
        assert_eq!(2, val_obj.len());
        assert_eq!(1, val_obj["v1"].as_i64().unwrap());
        assert_eq!(2, val_obj["v2"].as_i64().unwrap());
    }
}
