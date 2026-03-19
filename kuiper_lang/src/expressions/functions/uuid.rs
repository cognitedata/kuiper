use serde_json::Value;

use crate::expressions::{Expression, ResolveResult};

function_def!(Uuid4Function, "uuid4", 0);

impl Expression for Uuid4Function {
    fn is_deterministic(&self) -> bool {
        false
    }

    fn resolve<'a>(
        &'a self,
        _state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, crate::TransformError> {
        let id = uuid::Uuid::new_v4();
        Ok(ResolveResult::Owned(Value::String(id.to_string())))
    }

    fn resolve_types(
        &self,
        _state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        Ok(crate::types::Type::String)
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    fn test_uuid4() {
        let expr = compile_expression("uuid4()", &[]).unwrap();
        let result = expr.run(std::iter::empty::<&serde_json::Value>()).unwrap();
        result.as_str().expect("uuid4() should return a string");
    }

    #[test]
    fn test_uuid4_types() {
        let expr = compile_expression("uuid4()", &[]).unwrap();
        let ty = expr
            .run_types(std::iter::empty::<crate::types::Type>())
            .unwrap();
        assert_eq!(ty, crate::types::Type::String);
    }
}
