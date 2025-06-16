use std::fmt::Display;

use logos::Span;

use crate::{compiler::BuildError, write_list};

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
        write_list!(f, &self.input_names);
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
        inner.fail_if_lambda()?;
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

    fn resolve_types(
        &'a self,
        state: &mut super::types::TypeExecutionState<'c, '_>,
    ) -> Result<super::types::Type, super::types::TypeError> {
        self.expr.resolve_types(state)
    }

    fn call_types(
        &'a self,
        state: &mut super::types::TypeExecutionState<'c, '_>,
        values: &[&super::types::Type],
    ) -> Result<super::types::Type, super::types::TypeError> {
        let mut inner = state.get_temporary_clone(values.iter().copied(), self.input_names.len());
        let mut state = inner.get_temp_state();
        self.expr.resolve_types(&mut state)
    }
}

impl ExpressionMeta for LambdaExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new([self.expr.as_mut()].into_iter())
    }
}
