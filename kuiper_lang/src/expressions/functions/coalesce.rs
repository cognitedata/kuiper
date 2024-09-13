use crate::expressions::{Expression, ExpressionExecutionState, ResolveResult};

function_def!(CoalesceFunction, "coalesce", 2, None);

impl<'a: 'c, 'c> Expression<'a, 'c> for CoalesceFunction {
    fn resolve(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        for arg in &self.args {
            let v = arg.resolve(state)?;
            if !v.is_null() {
                return Ok(v);
            }
        }
        Ok(ResolveResult::Owned(serde_json::Value::Null))
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    pub fn test_coalesce() {
        let expr = compile_expression(r#"coalesce(null, "a", "b", "c")"#, &[]).unwrap();
        let res = expr.run([]).unwrap();

        let v = res.as_str().unwrap();
        assert_eq!(v, "a");
    }
}
