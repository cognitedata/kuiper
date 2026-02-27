use core::marker::PhantomData;

use crate::{
    expressions::{Expression, ExpressionExecutionState},
    source::SourceData,
    ExpressionType, ResolveResult, TransformError,
};

/// Builder for running an expression with custom inputs.
pub struct ExpressionRunBuilder<'a, 'c, T> {
    expression: &'a ExpressionType,
    _phantom: PhantomData<&'c ()>,
    items: T,
    max_operation_count: i64,
}

impl<'a, 'c, T> ExpressionRunBuilder<'a, 'c, T> {
    pub(super) fn new(expression: &'a ExpressionType) -> ExpressionRunBuilder<'a, 'c, ()> {
        ExpressionRunBuilder {
            expression,
            items: (),
            _phantom: PhantomData,
            max_operation_count: -1,
        }
    }

    /// Set the items to use as input for the expression.
    ///
    /// The count must match the count provided when the expression was compiled.
    pub fn with_custom_items<U>(self, items: U) -> ExpressionRunBuilder<'a, 'c, U::IntoIter>
    where
        U: IntoIterator<Item = &'c dyn SourceData>,
    {
        ExpressionRunBuilder {
            expression: self.expression,
            items: items.into_iter(),
            _phantom: PhantomData,
            max_operation_count: self.max_operation_count,
        }
    }

    /// Set the JSON values to use as input for the expression.
    ///
    /// The count must match the count provided when the expression was compiled.
    pub fn with_values<U>(
        self,
        items: U,
    ) -> ExpressionRunBuilder<'a, 'c, impl Iterator<Item = &'c dyn SourceData>>
    where
        U: IntoIterator<Item = &'c serde_json::Value>,
    {
        ExpressionRunBuilder {
            expression: self.expression,
            items: items.into_iter().map(|v| v as &dyn SourceData),
            _phantom: PhantomData,
            max_operation_count: self.max_operation_count,
        }
    }

    /// Set the maximum number of operations performed by the program. This is a rough estimate of the complexity of
    /// the program. If set to -1, no limit is enforced.
    pub fn max_operation_count(mut self, count: i64) -> Self {
        self.max_operation_count = count;
        self
    }
}

impl<'a: 'c, 'c, T> ExpressionRunBuilder<'a, 'c, T>
where
    T: Iterator<Item = &'c dyn SourceData>,
{
    /// Run the expression, returning the result.
    pub fn run(self) -> Result<ResolveResult<'c>, TransformError> {
        let mut opcount = 0;
        let data = self.items.map(Some).collect();
        let mut state =
            ExpressionExecutionState::new(&data, &mut opcount, self.max_operation_count);
        self.expression.resolve(&mut state)
    }

    /// Run the expression, returning the result along with the number of operations performed.
    pub fn run_get_opcount(self) -> Result<(ResolveResult<'c>, i64), TransformError> {
        let mut opcount = 0;
        let data = self.items.map(Some).collect();
        let mut state =
            ExpressionExecutionState::new(&data, &mut opcount, self.max_operation_count);
        let result = self.expression.resolve(&mut state)?;
        Ok((result, opcount))
    }

    #[cfg(feature = "completions")]
    /// Run the expression, and return the result along with a map from range in the input
    /// to possible completions in that range. These are only collected from selectors.
    pub fn run_get_completions(
        self,
    ) -> Result<(ResolveResult<'c>, crate::Completions), TransformError> {
        use std::collections::HashMap;

        let mut opcount = 0;
        let data = self.items.map(Some).collect();
        let mut state =
            ExpressionExecutionState::new(&data, &mut opcount, self.max_operation_count);
        let mut completions = HashMap::new();
        state.set_completions(&mut completions);
        let result = self.expression.resolve(&mut state)?;
        Ok((result, completions))
    }
}
