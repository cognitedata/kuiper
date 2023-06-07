use crate::exceptions::KuiperCompileError;
use crate::expressions::KuiperExpression;
use kuiper_lang::compile_expression;
use pyo3::{pyfunction, PyResult};
use std::collections::HashMap;

#[pyfunction]
#[pyo3(name = "compile_expression")]
pub fn compile_expression_py(
    expression: String,
    inputs: Vec<String>,
) -> PyResult<KuiperExpression> {
    let mut input_map =
        HashMap::from_iter(inputs.iter().enumerate().map(|(i, str)| (str.clone(), i)));

    match compile_expression(&expression, &mut input_map, "test") {
        Ok(expression) => Ok(KuiperExpression::new(expression)),
        Err(compile_error) => Err(KuiperCompileError::new_err(compile_error.to_string())),
    }
}
