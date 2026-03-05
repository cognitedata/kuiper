use crate::{
    exceptions::raise_kuiper_error,
    expressions::KuiperExpression,
    python_json::{ConversionError, PythonJson},
};
use kuiper_lang::{
    compile_expression_with_config, functions::DynamicFunction, CompilerConfig,
    DynamicFunctionBuilder, Expression, ExpressionExecutionState, ExpressionMeta, ExpressionType,
    ResolveResult, Span, TransformError,
};
use pyo3::{pyclass, pyfunction, pymethods, types::PyTuple, Py, PyAny, PyResult, Python};
use std::{fmt::Display, sync::Arc};

/// A custom function that can be used in Kuiper expressions.
///
/// Custom functions allows you to extend the Kuiper language with your own
/// functions.
///
/// Args:
///     name:    Name of the function as it will appear in Kuiper
///     target:  Python callable that implements the function
#[derive(Debug)]
#[pyclass(module = "kuiper", frozen)]
pub struct CustomFunction {
    name: String,
    target: Py<PyAny>,
}

#[pymethods]
impl CustomFunction {
    #[new]
    fn new(name: String, target: Py<PyAny>) -> Self {
        CustomFunction { name, target }
    }
}

fn build_custom_functions(
    mut config: CompilerConfig,
    custom_functions: Vec<Py<CustomFunction>>,
) -> CompilerConfig {
    for function in custom_functions {
        #[derive(Debug)]
        struct Custom {
            function: Arc<Py<CustomFunction>>,
            args: Vec<ExpressionType>,
            span: Span,
        }

        impl Display for Custom {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{}", self.function.get().name)
            }
        }

        impl Expression for Custom {
            fn resolve<'a>(
                &'a self,
                state: &mut ExpressionExecutionState<'a, '_>,
            ) -> Result<ResolveResult<'a>, TransformError> {
                let args = self
                    .args
                    .iter()
                    .map(|arg| arg.resolve(state))
                    .collect::<Result<Vec<_>, _>>()?;

                let result = Python::attach(|py| {
                    let args =
                        PyTuple::new(
                            py,
                            args.into_iter()
                                .map(|arg| {
                                    Ok(PythonJson::from_value(arg.into_owned().take())?
                                        .into_python(py))
                                })
                                .collect::<Result<Vec<_>, ConversionError>>()?,
                        )?;

                    Ok(self
                        .function
                        .get()
                        .target
                        .call(py, args, None)
                        .and_then(|res| res.extract::<PythonJson>(py))?)
                })
                .map_err(|err: ConversionError| err.into_transform_error(&self.span))?;

                Ok(ResolveResult::Owned(
                    result
                        .into_value()
                        .map_err(|e| e.into_transform_error(&self.span))?,
                ))
            }

            fn is_deterministic(&self) -> bool {
                false
            }
        }

        impl ExpressionMeta for Custom {
            fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
                Box::new(self.args.iter_mut())
            }
        }

        struct CustomBuilder {
            function: Arc<Py<CustomFunction>>,
        }

        impl DynamicFunctionBuilder for CustomBuilder {
            fn make_function(
                &self,
                args: Vec<ExpressionType>,
                span: Span,
            ) -> Result<Box<dyn DynamicFunction>, kuiper_lang::BuildError> {
                Ok(Box::new(Custom {
                    function: self.function.clone(),
                    args,
                    span,
                }))
            }
        }

        config = config.with_custom_dynamic_function(
            function.get().name.clone(),
            Arc::new(CustomBuilder {
                function: Arc::new(function),
            }),
        );
    }
    config
}

/// Compile a Kuiper expression.
///
/// This function compiles a Kuiper expression into a `KuiperExpression` object.
/// It takes the expression as a string, a list of input names, and optional
/// configuration parameters for the compiler.
///
/// Args:
///     expression:                 The Kuiper expression to compile.
///     inputs:                     A list of input names for the expression.
///     optimizer_operation_limit:  Maximum number of operations allowed during
///                                 optimization.
///     max_macro_expansions:       Maximum number of macro expansions allowed.
///     custom_functions:           Optional list of custom functions to include.
///
/// Returns:
///     A `KuiperExpression` object representing the compiled expression.
///
/// Raises:
///     KuiperCompileError: If the compilation encounters an error.
#[pyfunction]
#[pyo3(name = "compile_expression")]
#[pyo3(signature = (expression, inputs, optimizer_operation_limit=100_000, max_macro_expansions=20, custom_functions=None))]
pub fn compile_expression_py(
    expression: String,
    inputs: Vec<String>,
    optimizer_operation_limit: i64,
    max_macro_expansions: i32,
    custom_functions: Option<Vec<Py<CustomFunction>>>,
) -> PyResult<KuiperExpression> {
    let mut config = CompilerConfig::new()
        .optimizer_operation_limit(optimizer_operation_limit)
        .max_macro_expansions(max_macro_expansions);

    if let Some(custom_functions) = custom_functions {
        config = build_custom_functions(config, custom_functions);
    }

    match compile_expression_with_config(
        &expression,
        &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
        &config,
    ) {
        Ok(expression) => Ok(KuiperExpression::new(expression)),
        Err(compile_error) => Err(raise_kuiper_error(
            "KuiperCompileError",
            compile_error.to_string(),
            compile_error.span().map(|s| s.start),
            compile_error.span().map(|s| s.end),
        )),
    }
}
