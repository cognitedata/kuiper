use serde_json::Value;

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    BuildError,
};

function_def!(IfValueFunction, "if_value", 2, lambda);

impl Expression for IfValueFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<crate::expressions::ResolveResult<'a>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        if source.is_null() {
            return Ok(ResolveResult::Owned(Value::Null));
        }

        let res = self.args[1].call(state, &[source.as_ref()])?.into_owned();
        Ok(ResolveResult::Owned(res))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let source = self.args[0].resolve_types(state)?;

        let res = self.args[1].call_types(state, &[&source.clone().except_null()])?;

        if source.is_nullable() {
            Ok(res.nullable())
        } else {
            Ok(res)
        }
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
                "if_value takes a function with one argument",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::{compile_expression, types::Type};

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

    #[test]
    fn test_if_value_types() {
        let expr = compile_expression("if_value(input, a => a)", &["input"]).unwrap();
        let res = expr.run_types([Type::stringifyable()]).unwrap();
        assert_eq!(res, Type::stringifyable());

        let res = expr.run_types([Type::null()]).unwrap();
        assert_eq!(res, Type::null());

        let res = expr.run_types([Type::String]).unwrap();
        assert_eq!(res, Type::String);
    }
}
