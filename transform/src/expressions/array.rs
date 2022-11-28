use std::fmt::Display;

use logos::Span;
use serde_json::Value;

use super::{
    transform_error::TransformError, Expression, ExpressionExecutionState, ExpressionType,
};

pub struct ArrayExpression {
    items: Vec<ExpressionType>,
    span: Span,
}

impl Display for ArrayExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let mut needs_comma = false;
        for it in self.items.iter() {
            if needs_comma {
                write!(f, ", ")?;
            }
            needs_comma = true;
            write!(f, "{}", it)?;
        }
        Ok(())
    }
}

impl Expression for ArrayExpression {
    fn resolve(&self, state: &ExpressionExecutionState) -> Result<Value, TransformError> {
        let mut arr = vec![];
        for expr in self.items.iter() {
            arr.push(expr.resolve(state)?);
        }
        Ok(Value::Array(arr))
    }
}

impl ArrayExpression {
    pub fn new(items: Vec<ExpressionType>, span: Span) -> Self {
        Self { items, span }
    }
}
