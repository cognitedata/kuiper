use std::fmt::Display;

use logos::Span;
use serde_json::Value;

use crate::compiler::BuildError;

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
            write!(f, "{it}")?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl<'a: 'c, 'c> Expression<'a, 'c> for ArrayExpression {
    fn resolve(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        state.inc_op()?;

        let mut arr = vec![];
        for expr in self.items.iter() {
            arr.push(expr.resolve(state)?.into_owned());
        }
        Ok(ResolveResult::Owned(Value::Array(arr)))
    }
}

impl ExpressionMeta for ArrayExpression {
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
    pub fn new(items: Vec<ExpressionType>, span: Span) -> Result<Self, BuildError> {
        for item in &items {
            if let ExpressionType::Lambda(lambda) = &item {
                return Err(BuildError::unexpected_lambda(&lambda.span));
            }
        }
        Ok(Self { items, _span: span })
    }
}
