use crate::{
    expressions::{Expression, ExpressionExecutionState, ResolveResult},
    types::Type,
};

function_def!(CoalesceFunction, "coalesce", 2, None);

impl Expression for CoalesceFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        for arg in &self.args {
            let v = arg.resolve(state)?;
            if !v.is_null() {
                return Ok(v);
            }
        }
        Ok(ResolveResult::Owned(serde_json::Value::Null))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let mut final_type = Type::never();
        let mut final_found = false;
        for arg in &self.args {
            let t = arg.resolve_types(state)?;
            if !t.is_null() && !final_found {
                final_type = final_type.union_with(t.clone().except_null());
            }
            if !t.is_assignable_to(&Type::null()) {
                final_found = true;
            }
        }
        if !final_found {
            final_type = final_type.nullable();
        }
        if final_type.is_never() {
            Ok(crate::types::Type::null())
        } else {
            Ok(final_type)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{compile_expression, types::Type};

    #[test]
    pub fn test_coalesce() {
        let expr = compile_expression(r#"coalesce(null, "a", "b", "c")"#, &[]).unwrap();
        let res = expr.run([]).unwrap();

        let v = res.as_str().unwrap();
        assert_eq!(v, "a");
    }

    #[test]
    pub fn test_coalesce_types() {
        let expr = compile_expression(
            r#"coalesce(input1, input2, input3)"#,
            &["input1", "input2", "input3"],
        )
        .unwrap();
        let t = expr
            .run_types([Type::null(), Type::Integer, Type::String])
            .unwrap();
        assert_eq!(t, Type::Integer);
        let t = expr
            .run_types([Type::null(), Type::null(), Type::String])
            .unwrap();
        assert_eq!(t, Type::String);
        let t = expr
            .run_types([Type::null(), Type::null(), Type::null()])
            .unwrap();
        assert_eq!(t, Type::null());

        let t = expr
            .run_types([
                Type::null(),
                Type::Integer.nullable(),
                Type::String.nullable(),
            ])
            .unwrap();
        assert_eq!(t, Type::Integer.union_with(Type::String).nullable());
    }
}
