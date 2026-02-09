use std::fmt::Display;

use logos::Span;
use serde_json::Value;

use crate::{compiler::BuildError, write_list};

use super::{
    base::ExpressionMeta, transform_error::TransformError, Expression, ExpressionExecutionState,
    ExpressionType, ResolveResult,
};

#[derive(Debug, Clone)]
pub enum ArrayElement {
    Expression(ExpressionType),
    Concat(ExpressionType),
}

impl Display for ArrayElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expression(x) => write!(f, "{x}"),
            Self::Concat(x) => write!(f, "...{x}"),
        }
    }
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
        write_list!(f, self.items.iter());
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
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new(self.items.iter_mut().map(|e| match e {
            ArrayElement::Expression(x) => x,
            ArrayElement::Concat(x) => x,
        }))
    }
}

impl ArrayExpression {
    pub fn new(items: Vec<ArrayElement>, span: Span) -> Result<Self, BuildError> {
        for item in &items {
            let expr = match item {
                ArrayElement::Expression(x) => x,
                ArrayElement::Concat(x) => x,
            };
            expr.fail_if_lambda()?;
        }
        Ok(Self { items, span })
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::compile_err;

    #[test]
    fn test_invalid_concat() {
        let err = compile_err("[1, ...2]", &[]);
        assert_eq!(
            err.to_string(),
            "Compilation failed: array. Got number, expected array at 0..9"
        );
    }
}
