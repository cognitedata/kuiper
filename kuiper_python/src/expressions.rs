use crate::exceptions::raise_kuiper_error;
use kuiper_lang::ExpressionType;
use pyo3::{pyclass, pymethods, PyResult};
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

    fn __str__(&self) -> String {
        self.expression.to_string()
    }
}
