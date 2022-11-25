use serde_json::Value;

use super::{
    base::{Expression, ExpressionExecutionState},
    transform_error::TransformError,
};

struct SelectorExpression {
    source: String,
    path: Vec<String>,
}

impl Expression for SelectorExpression {
    fn resolve(&self, state: &ExpressionExecutionState) -> Result<Value, TransformError> {
        let source = state.data.get(&self.source);
        let source =
            source.ok_or_else(|| TransformError::SourceMissingError(self.source.clone()))?;
        let mut elem = source;
        for p in &self.path {
            elem = match elem.as_object().and_then(|o| o.get(p)) {
                Some(x) => x,
                None => return Ok(Value::Null),
            }
        }
        Ok(elem.clone())
    }
}
