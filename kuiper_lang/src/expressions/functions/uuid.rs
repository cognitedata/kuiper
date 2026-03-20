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
        let uuid_str = result.as_str().expect("uuid4() should return a string");

        let parsed_uuid =
            uuid::Uuid::parse_str(uuid_str).expect("uuid4() should return a valid UUID string");

        assert_eq!(
            parsed_uuid.get_version_num(),
            4,
            "uuid4() should return a version 4 UUID"
        );
    }

    #[test]
    fn test_uuid4_types() {
        let expr = compile_expression("uuid4()", &[]).unwrap();
        let ty = expr
            .run_types(std::iter::empty::<crate::types::Type>())
            .unwrap();
        assert_eq!(ty, crate::types::Type::String);
    }

    #[test]
    fn test_uuid4_uniqueness() {
        let expr = compile_expression("uuid4()", &[]).unwrap();
        let result1 = expr.run(std::iter::empty::<&serde_json::Value>()).unwrap();
        let result2 = expr.run(std::iter::empty::<&serde_json::Value>()).unwrap();

        let uuid1 = result1.as_str().unwrap();
        let uuid2 = result2.as_str().unwrap();

        assert_ne!(
            uuid1, uuid2,
            "uuid4() should generate different UUIDs on each call"
        );
    }
}
