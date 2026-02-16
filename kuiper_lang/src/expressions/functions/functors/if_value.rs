use serde_json::Value;

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    BuildError,
};

function_def!(IfValueFunction, "if_value", 2, lambda);

impl Expression for IfValueFunction {
    fn resolve<'a: 'c, 'c>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        if source.is_null() {
            return Ok(ResolveResult::Owned(Value::Null));
        }

        let res = self.args[1].call(state, &[source.as_ref()])?.into_owned();
        Ok(ResolveResult::Owned(res))
    }
}

impl LambdaAcceptFunction for IfValueFunction {
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
    use serde_json::Value;

    use crate::compile_expression;

    #[test]
    fn test_if_value() {
        let expr = compile_expression(
            r#"
            {
                "v1": "hello".if_value(a => concat(a, " world")),
                "v2": null.if_value(a => a + 1),
                "v3": 123.if_value(a => a + 1),
                "v4": [1, 2, 3].if_value(a => a[0] + a[1]),
            }
        "#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        assert_eq!(res.get("v1").unwrap(), "hello world");
        assert_eq!(res.get("v2").unwrap(), &Value::Null);
        assert_eq!(res.get("v3").unwrap(), 124);
        assert_eq!(res.get("v4").unwrap(), 3);
    }
}
