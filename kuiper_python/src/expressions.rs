use crate::{
    exceptions::raise_kuiper_error,
    python_json::{ConversionError, PythonJson},
};
use kuiper_lang::ExpressionType;
use pyo3::{
    pyclass, pymethods,
    types::{PyAnyMethods, PyTuple, PyTupleMethods},
    Py, PyAny, PyResult, Python,
};
use serde_json::{from_str, Value};

/// A compiled Kuiper expression.
///
/// This class can not be instantiated directly, it should be created through the
/// `compile_expression` function.
#[pyclass(module = "kuiper", frozen)]
pub struct KuiperExpression {
    expression: ExpressionType,
}

impl KuiperExpression {
    pub fn new(expression: ExpressionType) -> Self {
        KuiperExpression { expression }
    }
}

impl KuiperExpression {
    fn get_expression_input<'a>(input: impl IntoIterator<Item = &'a str>) -> PyResult<Vec<Value>> {
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
        Ok(json)
    }
}

#[pymethods]
impl KuiperExpression {
    /// Run the expression.
    ///
    /// This method evaluates the expression on the given input. It takes native python
    /// objects as input and returns a native python object as output. The inputs must
    /// be possible to turn into JSON, i.e., strings, integers, floats, booleans, None,
    /// lists, or dictionaries.
    ///
    /// Args:
    ///     inputs:          Inputs to the expression.
    ///     max_operations:  Maximum number of operations allowed, useful for limiting
    ///                      the computational resources used by an expression. If a
    ///                      computation exceeds this limit, a KuiperRuntimeError is
    ///                      raised.
    ///
    /// Returns:
    ///     The result of evaluating the expression on the given input.
    ///
    /// Raises:
    ///     KuiperRuntimeError: If the expression evaluation encounters an error.
    #[pyo3(signature = (*inputs, max_operations=None))]
    fn run(&self, inputs: Py<PyTuple>, max_operations: Option<i64>) -> PyResult<Py<PyAny>> {
        let inputs = Python::attach(|py| {
            inputs
                .bind(py)
                .iter()
                .map(|item| item.extract())
                .collect::<PyResult<Vec<PythonJson>>>()
        })?
        .into_iter()
        .map(|item| item.into_value())
        .collect::<Result<Vec<_>, _>>()
        .map_err(ConversionError::into_python_error)?;

        let run_result = if let Some(op_limit) = max_operations {
            self.expression.run_limited(inputs.iter(), op_limit)
        } else {
            self.expression.run(inputs.iter())
        };

        let run_result = run_result
            .map_err(|transform_error| {
                raise_kuiper_error(
                    "KuiperRuntimeError",
                    transform_error.to_string(),
                    transform_error.span().map(|s| s.start),
                    transform_error.span().map(|s| s.end),
                )
            })?
            .into_owned();

        Python::attach(|py| {
            Ok(PythonJson::from_value(run_result)
                .map_err(ConversionError::into_python_error)?
                .into_python(py))
        })
    }

    /// Run the expression.
    ///
    /// This method evaluates the expression on the given input. It takes a JSON string
    /// as input and returns a JSON string as output.
    ///
    /// Args:
    ///     inputs:          Inputs to the expression as JSON strings.
    ///     max_operations:  Maximum number of operations allowed, useful for limiting
    ///                      the computational resources used by an expression. If a
    ///                      computation exceeds this limit, a KuiperRuntimeError is
    ///                      raised.
    ///
    /// Returns:
    ///     The result of evaluating the expression on the given input.
    ///
    /// Raises:
    ///     KuiperRuntimeError: If the expression evaluation encounters an error.
    #[pyo3(signature = (*inputs, max_operations=None))]
    fn run_json(&self, inputs: Vec<String>, max_operations: Option<i64>) -> PyResult<String> {
        let json = Self::get_expression_input(inputs.iter().map(String::as_str))?;

        let run_result = if let Some(op_limit) = max_operations {
            self.expression.run_limited(json.iter(), op_limit)
        } else {
            self.expression.run(json.iter())
        };

        match run_result {
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

    fn __repr__(&self) -> String {
        self.expression.to_string()
    }
}
