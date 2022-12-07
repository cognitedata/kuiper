use std::fmt::Display;

use logos::Span;
use serde_json::Value;

use super::{
    base::{ExpressionMeta, ResolveResult},
    transform_error::TransformError,
    Expression, ExpressionExecutionState, ExpressionType,
};

#[derive(Debug, Clone)]
/// Array expression. This contains a list of expressions and returns an array.
pub struct ArrayExpression {
    items: Vec<ExpressionType>,
    _span: Span,
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

impl<'a> Expression<'a> for ArrayExpression {
    fn resolve(
        &self,
        state: &ExpressionExecutionState,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let mut arr = vec![];
        for expr in self.items.iter() {
            arr.push(expr.resolve(state)?.as_ref().clone());
        }
        Ok(ResolveResult::Value(Value::Array(arr)))
    }
}

impl<'a> ExpressionMeta<'a> for ArrayExpression {
    fn num_children(&self) -> usize {
        self.items.len()
    }

    fn get_child(&self, idx: usize) -> Option<&ExpressionType> {
        self.items.get(idx)
    }

    fn get_child_mut(&mut self, idx: usize) -> Option<&mut ExpressionType> {
        self.items.get_mut(idx)
    }

    fn set_child(&mut self, idx: usize, item: ExpressionType) {
        if idx >= self.items.len() {
            return;
        }
        self.items[idx] = item;
    }
}

impl ArrayExpression {
    pub fn new(items: Vec<ExpressionType>, span: Span) -> Self {
        Self { items, _span: span }
    }
}
