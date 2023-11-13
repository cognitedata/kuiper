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
        state: &mut super::ExpressionExecutionState<'c, '_>,
    ) -> Result<super::ResolveResult<'c>, crate::TransformError> {
        state.inc_op()?;
        self.expr.resolve(state)
    }

    fn call<'d>(
        &'a self,
        state: &mut super::ExpressionExecutionState<'c, '_>,
        values: &[&'d serde_json::Value],
    ) -> Result<super::ResolveResult<'c>, crate::TransformError> {
        state.inc_op()?;
        let mut inner = state.get_temporary_clone(values.iter().copied(), self.input_names.len());
        let mut state = inner.get_temp_state();
        let r = self.expr.resolve(&mut state)?;
        Ok(ResolveResult::Owned(r.into_owned()))
    }
}

impl ExpressionMeta for LambdaExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new([self.expr.as_mut()].into_iter())
    }
}
