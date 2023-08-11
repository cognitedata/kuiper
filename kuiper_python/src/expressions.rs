use crate::exceptions::raise_kuiper_error;
use kuiper_lang::ExpressionType;
use pyo3::{pyclass, pymethods, PyResult};
use serde_json::from_str;

#[pyclass]
pub struct KuiperExpression {
    expression: ExpressionType,
}

impl KuiperExpression {
    pub fn new(expression: ExpressionType) -> Self {
        KuiperExpression { expression }
    }
}

impl KuiperExpression {
    fn run_internal<'a>(&self, input: impl IntoIterator<Item = &'a str>) -> PyResult<String> {
        let json = input
            .into_iter()
            .map(from_str)
            .collect::<Result<Vec<_>, _>>();
        let json = match json {
            Ok(values) => values,
            Err(json_error) => {
                return Err(raise_kuiper_error(
                    "KuiperRuntimeError",
                    json_error.to_string(),
                    Some(json_error.column()),
                    Some(json_error.column()),
                ))
            }
        };
        match self.expression.run(json.iter()) {
            Ok(result) => Ok(result.to_string()),
            Err(transform_error) => Err(raise_kuiper_error(
                "KuiperRuntimeError",
                transform_error.to_string(),
                transform_error.span().map(|s| s.start),
                transform_error.span().map(|s| s.end),
            )),
        }
    }
}

#[pymethods]
impl KuiperExpression {
    fn run(&self, input: String) -> PyResult<String> {
        self.run_internal([input.as_str()])
    }

    fn run_multiple_inputs(&self, inputs: Vec<String>) -> PyResult<String> {
        self.run_internal(inputs.iter().map(|s| s.as_str()))
    }
}
