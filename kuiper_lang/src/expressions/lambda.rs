use std::fmt::Display;

use logos::Span;

use crate::compiler::BuildError;

use super::{base::ExpressionMeta, Expression, ExpressionType, ResolveResult};

#[derive(Debug, Clone)]
pub struct LambdaExpression {
    pub input_names: Vec<String>,
    expr: Box<ExpressionType>,
    pub span: Span,
}

impl Display for LambdaExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        let mut needs_comma = false;
        for arg in &self.input_names {
            if needs_comma {
                write!(f, ", ")?;
            }
            needs_comma = true;
            write!(f, "{arg}")?;
        }
        write!(f, ")")?;
        write!(f, " => {}", self.expr)?;
        Ok(())
    }
}

impl LambdaExpression {
    pub fn new(
        input_names: Vec<String>,
        inner: ExpressionType,
        span: Span,
    ) -> Result<Self, BuildError> {
        if let ExpressionType::Lambda(lambda) = &inner {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }
        Ok(Self {
            input_names,
            expr: Box::new(inner),
            span,
        })
    }
}

impl<'a: 'c, 'c> Expression<'a, 'c> for LambdaExpression {
    fn resolve(
        &'a self,
        state: &super::ExpressionExecutionState<'c, '_>,
    ) -> Result<super::ResolveResult<'c>, crate::TransformError> {
        self.expr.resolve(state)
    }

    fn call<'d>(
        &'a self,
        state: &super::ExpressionExecutionState<'c, '_>,
        values: &[&'d serde_json::Value],
    ) -> Result<super::ResolveResult<'c>, crate::TransformError> {
        let inner = state.get_temporary_clone_inner(values.iter().copied(), self.input_names.len());
        let state = inner.get_temp_state();
        let r = self.expr.resolve(&state)?;
        Ok(ResolveResult::Owned(r.into_owned()))
    }
}

impl ExpressionMeta for LambdaExpression {
    fn num_children(&self) -> usize {
        1
    }

    fn get_child(&self, idx: usize) -> Option<&ExpressionType> {
        if idx > 0 {
            None
        } else {
            Some(self.expr.as_ref())
        }
    }

    fn get_child_mut(&mut self, idx: usize) -> Option<&mut ExpressionType> {
        if idx > 0 {
            None
        } else {
            Some(self.expr.as_mut())
        }
    }

    fn set_child(&mut self, idx: usize, item: ExpressionType) {
        if idx == 0 {
            self.expr = Box::new(item);
        }
    }
}
