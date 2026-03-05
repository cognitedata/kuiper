mod compiler;
mod exceptions;
mod expressions;
pub(crate) mod python_json;

use crate::compiler::{compile_expression_py, CustomFunction};
use crate::expressions::KuiperExpression;
use pyo3::prelude::PyModule;
use pyo3::types::PyModuleMethods;
use pyo3::{pymodule, wrap_pyfunction, Bound, PyResult, Python};

#[pymodule]
fn _core(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(compile_expression_py, py)?)?;
    module.add_class::<KuiperExpression>()?;
    module.add_class::<CustomFunction>()?;
    Ok(())
}
