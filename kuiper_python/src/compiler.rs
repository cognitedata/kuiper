use crate::{exceptions::raise_kuiper_error, expressions::KuiperExpression};
use kuiper_lang::compile_expression;
use pyo3::{pyfunction, PyResult};

#[pyfunction]
#[pyo3(name = "compile_expression")]
pub fn compile_expression_py(
    expression: String,
    inputs: Vec<String>,
) -> PyResult<KuiperExpression> {
    match compile_expression(
        &expression,
        &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
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
