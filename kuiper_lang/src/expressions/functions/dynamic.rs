use std::fmt::Debug;

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
pub fn make_function<T: FunctionExpression + DynamicFunction + 'static>(
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
pub trait DynamicFunctionBuilder: Send + Sync {
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

/// Trait for providing a builder for a dynamic function with a given name.
///
/// This should simply return `None` if the function does not exist.
pub trait DynamicFunctionSource: Send + Sync {
    fn build_function(&self, name: &str) -> Option<Box<dyn DynamicFunctionBuilder>>;
}

impl<T> DynamicFunctionSource for T
where
    T: Fn(&str) -> Option<Box<dyn DynamicFunctionBuilder>> + Send + Sync,
{
    fn build_function(&self, name: &str) -> Option<Box<dyn DynamicFunctionBuilder>> {
        self(name)
    }
}

pub(crate) struct EmptyFunctionSource;

impl DynamicFunctionSource for EmptyFunctionSource {
    fn build_function(&self, _name: &str) -> Option<Box<dyn DynamicFunctionBuilder>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::{
        compile_expression_with_config,
        expressions::{functions::dynamic::make_function, Expression},
        functions::DynamicFunctionBuilder,
        CompilerConfig, ExpressionExecutionState, ResolveResult,
    };

    function_def!(MyCustomFunction, "test_func", 0);

    impl Expression for MyCustomFunction {
        fn resolve<'a: 'c, 'c>(
            &'a self,
            _state: &mut ExpressionExecutionState<'c, '_>,
        ) -> Result<ResolveResult<'c>, crate::TransformError> {
            Ok(ResolveResult::Owned(Value::String(
                "Hello from test_func!".into(),
            )))
        }
    }

    fn mk_my_function(name: &str) -> Option<Box<dyn DynamicFunctionBuilder>> {
        if name == "test_func" {
            Some(Box::new(make_function::<MyCustomFunction>) as Box<dyn DynamicFunctionBuilder>)
        } else {
            None
        }
    }

    #[test]
    fn test_dynamic_function() {
        let expr = compile_expression_with_config(
            "test_func()",
            &[],
            &CompilerConfig::new().custom_function_source(mk_my_function),
        )
        .unwrap();
        let res = expr.run(&[]).unwrap();
        assert_eq!(res.as_str().unwrap(), "Hello from test_func!");
    }
}
