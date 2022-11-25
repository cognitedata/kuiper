use std::collections::HashMap;

use serde_json::Value;

use super::transform_error::TransformError;

pub struct ExpressionExecutionState {
    pub data: HashMap<String, Value>,
}

pub trait Expression {
    fn resolve(&self, state: &ExpressionExecutionState) -> Result<Value, TransformError>;
}
