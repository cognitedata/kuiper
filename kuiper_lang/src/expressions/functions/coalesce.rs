use crate::expressions::{types::Type, Expression, ExpressionExecutionState, ResolveResult};

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

    fn resolve_types(
        &'a self,
        state: &mut crate::expressions::types::TypeExecutionState<'c, '_>,
    ) -> Result<crate::expressions::types::Type, crate::expressions::types::TypeError> {
        let mut final_type = Type::Union(Vec::new());
        for arg in &self.args {
            let t = arg.resolve_types(state)?;
            if !t.is_null() {
                final_type = final_type.union_with(t);
            }
        }
        if final_type.is_never() {
            Ok(crate::expressions::types::Type::null())
        } else {
            Ok(final_type)
        }
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
