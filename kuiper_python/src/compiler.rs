use crate::{exceptions::raise_kuiper_error, expressions::KuiperExpression};
use kuiper_lang::{compile_expression_with_config, CompilerConfig};
use pyo3::{pyfunction, PyResult};

#[pyfunction]
#[pyo3(name = "compile_expression")]
#[pyo3(signature = (expression, inputs, optimizer_operation_limit=100_000, max_macro_expansions=20))]
pub fn compile_expression_py(
    expression: String,
    inputs: Vec<String>,
    optimizer_operation_limit: i64,
    max_macro_expansions: i32,
) -> PyResult<KuiperExpression> {
    match compile_expression_with_config(
        &expression,
        &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
        &CompilerConfig::new()
            .optimizer_operation_limit(optimizer_operation_limit)
            .max_macro_expansions(max_macro_expansions),
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
