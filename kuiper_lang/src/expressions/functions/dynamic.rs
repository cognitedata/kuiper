use std::{collections::HashMap, fmt::Debug, sync::Arc};

use logos::Span;

use crate::{
    expressions::Expression, functions::FunctionExpression, ExpressionMeta, ExpressionType,
};

/// Trait for functions defined by library users.
///
/// This is implemented for types that implement ExpressionMeta,
/// Expression, Debug, Send and Sync.
pub trait DynamicFunction: ExpressionMeta + Expression + Debug + Send + Sync {}

impl<T> DynamicFunction for T where T: ExpressionMeta + Expression + Debug + Send + Sync {}

/// Utility function for creating a DynamicFunction from a type that implements FunctionExpression.
/// This method implements `DynamicFunctionBuilder` for any type that implements `FunctionExpression`.
pub(crate) fn make_function<T: FunctionExpression + DynamicFunction + 'static>(
    args: Vec<ExpressionType>,
    span: Span,
) -> Result<Box<dyn DynamicFunction>, crate::compiler::BuildError> {
    Ok(Box::new(T::new(args, span)?))
}

/// Trait for constructing dynamic functions.
/// This is used for custom functions defined by library users, and passed
/// to the compiler at build time.
///
/// This has a blanket implementation for
/// `Fn(Vec<ExpressionType>, Span) -> Result<Box<dyn DynamicFunction>, BuildError>`,
/// so you can just pass a function or closure of that type to the compiler.
///
/// Note that these function names shadow built-in functions, but not macros.
pub(crate) trait DynamicFunctionBuilder: Send + Sync {
    /// Create a new dynamic function with the given arguments and span.
    fn make_function(
        &self,
        args: Vec<ExpressionType>,
        span: Span,
    ) -> Result<Box<dyn DynamicFunction>, crate::compiler::BuildError>;
}

impl<T> DynamicFunctionBuilder for T
where
    T: Fn(
            Vec<ExpressionType>,
            Span,
        ) -> Result<Box<dyn DynamicFunction>, crate::compiler::BuildError>
        + Send
        + Sync,
{
    fn make_function(
        &self,
        args: Vec<ExpressionType>,
        span: Span,
    ) -> Result<Box<dyn DynamicFunction>, crate::compiler::BuildError> {
        self(args, span)
    }
}

#[derive(Default, Clone)]
pub(crate) struct DynamicFunctionSource {
    functions: HashMap<String, Arc<dyn DynamicFunctionBuilder>>,
}

impl DynamicFunctionSource {
    pub fn with_function<T: DynamicFunction + FunctionExpression + 'static>(
        &mut self,
        name: impl Into<String>,
    ) {
        self.functions
            .insert(name.into(), Arc::new(make_function::<T>));
    }

    pub fn get(&self, name: &str) -> Option<&dyn DynamicFunctionBuilder> {
        self.functions.get(name).map(|arc| arc.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::{
        compile_expression_with_config, expressions::Expression, CompilerConfig,
        ExpressionExecutionState, ResolveResult,
    };

    function_def!(MyCustomFunction, "test_func", 0);

    impl Expression for MyCustomFunction {
        fn resolve<'a>(
            &'a self,
            _state: &mut ExpressionExecutionState<'a, '_>,
        ) -> Result<ResolveResult<'a>, crate::TransformError> {
            Ok(ResolveResult::Owned(Value::String(
                "Hello from test_func!".into(),
            )))
        }
    }

    #[test]
    fn test_dynamic_function() {
        let expr = compile_expression_with_config(
            "test_func()",
            &[],
            &CompilerConfig::new().with_custom_function::<MyCustomFunction>("test_func"),
        )
        .unwrap();
        let res = expr.run(&[]).unwrap();
        assert_eq!(res.as_str().unwrap(), "Hello from test_func!");
    }
}
