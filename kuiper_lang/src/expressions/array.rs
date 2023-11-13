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
pub enum ArrayElement {
    Expression(ExpressionType),
    Concat(ExpressionType),
}

#[derive(Debug, Clone)]
/// Array expression. This contains a list of expressions and returns an array.
pub struct ArrayExpression {
    items: Vec<ArrayElement>,
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
            match it {
                ArrayElement::Expression(x) => write!(f, "{x}")?,
                ArrayElement::Concat(x) => write!(f, "..{x}")?,
            }
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
            match expr {
                ArrayElement::Expression(x) => arr.push(x.resolve(state)?.into_owned()),
                ArrayElement::Concat(x) => {
                    let conc = x.resolve(state)?;
                    match conc {
                        ResolveResult::Owned(Value::Array(x)) => {
                            for elem in x {
                                arr.push(elem);
                            }
                        }
                        ResolveResult::Borrowed(Value::Array(x)) => {
                            for elem in x {
                                arr.push(elem.to_owned());
                            }
                        }
                        x => {
                            return Err(TransformError::new_incorrect_type(
                                "array",
                                "array",
                                TransformError::value_desc(&x),
                                &self.span,
                            ))
                        }
                    };
                }
            }
        }
        Ok(ResolveResult::Owned(Value::Array(arr)))
    }
}

impl ExpressionMeta for ArrayExpression {
    fn num_children(&self) -> usize {
        self.items.len()
    }

    fn get_child(&self, idx: usize) -> Option<&ExpressionType> {
        self.items.get(idx).map(|e| match e {
            ArrayElement::Expression(x) => x,
            ArrayElement::Concat(x) => x,
        })
    }

    fn get_child_mut(&mut self, idx: usize) -> Option<&mut ExpressionType> {
        self.items.get_mut(idx).map(|e| match e {
            ArrayElement::Expression(x) => x,
            ArrayElement::Concat(x) => x,
        })
    }

    fn set_child(&mut self, idx: usize, item: ExpressionType) {
        if idx >= self.items.len() {
            return;
        }
        let rf = &mut self.items[idx];
        match rf {
            ArrayElement::Expression(x) => *x = item,
            ArrayElement::Concat(x) => *x = item,
        }
    }
}

impl ArrayExpression {
    pub fn new(items: Vec<ArrayElement>, span: Span) -> Result<Self, BuildError> {
        for item in &items {
            let expr = match item {
                ArrayElement::Expression(x) => x,
                ArrayElement::Concat(x) => x,
            };
            if let ExpressionType::Lambda(lambda) = &expr {
                return Err(BuildError::unexpected_lambda(&lambda.span));
            }
        }
        Ok(Self { items, span })
    }
}
