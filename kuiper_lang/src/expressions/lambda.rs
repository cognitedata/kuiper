use core::fmt::Display;

use logos::Span;

use crate::{compiler::BuildError, expressions::source::SourceData, write_list};

use super::{base::ExpressionMeta, Expression, ExpressionType, ResolveResult};

#[derive(Debug)]
pub struct LambdaExpression {
    pub input_names: crate::Vec<crate::String>,
    expr: crate::Box<ExpressionType>,
    pub span: Span,
}

impl Display for LambdaExpression {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(")?;
        write_list!(f, &self.input_names);
        write!(f, ")")?;
        write!(f, " => {}", self.expr)?;
        Ok(())
    }
}

impl LambdaExpression {
    pub fn new(
        input_names: crate::Vec<crate::String>,
        inner: ExpressionType,
        span: Span,
    ) -> Result<Self, BuildError> {
        inner.fail_if_lambda()?;
        Ok(Self {
            input_names,
            expr: crate::Box::new(inner),
            span,
        })
    }
}

impl Expression for LambdaExpression {
    fn resolve<'a>(
        &'a self,
        state: &mut super::ExpressionExecutionState<'a, '_>,
    ) -> Result<super::ResolveResult<'a>, crate::TransformError> {
        state.inc_op()?;
        self.expr.resolve(state)
    }

    fn call<'a>(
        &'a self,
        state: &mut super::ExpressionExecutionState<'a, '_>,
        values: &[&serde_json::Value],
    ) -> Result<super::ResolveResult<'a>, crate::TransformError> {
        state.inc_op()?;
        let mut inner = state.get_temporary_clone(
            values.iter().map(|v| *v as &dyn SourceData),
            self.input_names.len(),
        );
        let mut state = inner.get_temp_state();
        let r = self.expr.resolve(&mut state)?;
        Ok(ResolveResult::Owned(r.into_owned()))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        self.expr.resolve_types(state)
    }

    fn call_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
        values: &[&crate::types::Type],
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let mut inner = state.get_temporary_clone(values.iter().copied(), self.input_names.len());
        let mut state = inner.get_temp_state();
        self.expr.resolve_types(&mut state)
    }
}

impl ExpressionMeta for LambdaExpression {
    fn iter_children_mut(&mut self) -> crate::Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        crate::Box::new([self.expr.as_mut()].into_iter())
    }
}
