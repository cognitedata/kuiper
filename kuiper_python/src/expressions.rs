use crate::exceptions::KuiperRuntimeError;
use kuiper_lang::ExpressionType;
use pyo3::exceptions::PyNotImplementedError;
use pyo3::prelude::PyModule;
use pyo3::types::PyDict;
use pyo3::{pyclass, pymethods, PyResult, Python};
use serde_json::{from_str, Value};

#[pyclass]
pub struct KuiperExpression {
    expression: ExpressionType,
}

impl KuiperExpression {
    pub fn new(expression: ExpressionType) -> Self {
        KuiperExpression { expression }
    }
}

#[pymethods]
impl KuiperExpression {
    fn run(&self, input: String) -> PyResult<String> {
        let json: Vec<Value> = match from_str(&input) {
            Ok(value) => vec![value],
            Err(json_error) => return Err(KuiperRuntimeError::new_err(json_error.to_string())),
        };

        match self.expression.run(json.iter(), "testrun") {
            Ok(result) => Ok(result.to_string()),
            Err(transform_error) => Err(KuiperRuntimeError::new_err(transform_error.to_string())),
        }
    }
}
