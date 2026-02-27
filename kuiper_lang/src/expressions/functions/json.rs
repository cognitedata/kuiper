use serde_json::Value;

use crate::{
    expressions::{Expression, ResolveResult},
    TransformError,
};

function_def!(ParseJsonFunction, "parse_json", 1);

impl Expression for ParseJsonFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::String(x) => {
                let parsed = serde_json::from_str(x).map_err(|e| {
                    TransformError::InvalidOperation(crate::TransformErrorData {
                        span: self.span.clone(),
                        desc: alloc::format!("Failed to parse JSON in function parse_json: {e}"),
                    })
                })?;
                Ok(ResolveResult::Owned(parsed))
            }
            // If the input isn't a string, just return it as-is. No matter what the
            // value is, if we were to "parse" it as JSON, we would just get the original value back.
            _ => Ok(source),
        }
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let source = self.args[0].resolve_types(state)?;
        if source.is_assignable_to(&crate::types::Type::String) {
            Ok(crate::types::Type::Any)
        } else {
            Ok(source)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    fn test_parse_json() {
        let expr = compile_expression(
            r#"
        parse_json('{"foo": "bar"}')
        "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        assert_eq!(
            res.as_ref(),
            &serde_json::json!({
                "foo": "bar"
            })
        );
    }

    #[test]
    fn test_parse_json_misc() {
        let expr = compile_expression(
            r#"
        {
            "v1": parse_json(1),
            "v2": parse_json(null),
            "v3": parse_json(true),
            "v4": parse_json([1, 2, 3]),
            "v5": parse_json({"a": 1, "b": 2}),
            "v6": parse_json("[1, 2, 3]"),
            "v7": parse_json('"hello"'),
            "v8": parse_json("null"),
            "v9": parse_json(input),
        }
        "#,
            &["input"],
        )
        .unwrap();
        let input = serde_json::json!("\"hello there\"");
        let res = expr.run([&input]).unwrap();

        let o = res.as_object().unwrap();
        assert_eq!(o.get("v1").unwrap(), &serde_json::json!(1));
        assert_eq!(o.get("v2").unwrap(), &serde_json::json!(null));
        assert_eq!(o.get("v3").unwrap(), &serde_json::json!(true));
        assert_eq!(o.get("v4").unwrap(), &serde_json::json!([1, 2, 3]));
        assert_eq!(o.get("v5").unwrap(), &serde_json::json!({"a": 1, "b": 2}));
        assert_eq!(o.get("v6").unwrap(), &serde_json::json!([1, 2, 3]));
        assert_eq!(o.get("v7").unwrap(), &serde_json::json!("hello"));
        assert_eq!(o.get("v8").unwrap(), &serde_json::json!(null));
        assert_eq!(o.get("v9").unwrap(), &serde_json::json!("hello there"));
    }

    #[test]
    fn test_parse_json_wrong_keys() {
        // This is the original motivating example for this function.

        let expr = compile_expression(
            r#"
            parse_json("{'foo': 'bar'}".replace('\'', '"'))
            "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        assert_eq!(
            res.as_ref(),
            &serde_json::json!({
                "foo": "bar"
            })
        );
    }

    #[test]
    fn test_json_types() {
        let expr = compile_expression(
            r#"
            parse_json(input)
            "#,
            &["input"],
        )
        .unwrap();
        let t = expr.run_types([crate::types::Type::String]).unwrap();
        assert_eq!(t, crate::types::Type::Any);

        let t = expr.run_types([crate::types::Type::Integer]).unwrap();
        assert_eq!(t, crate::types::Type::Integer);
    }
}
