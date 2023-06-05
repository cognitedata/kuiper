mod compiler;
mod exceptions;
mod expressions;

use crate::compiler::compile_expression_py;
use crate::exceptions::KuiperError;
use crate::expressions::KuiperExpression;
use pyo3::prelude::PyModule;
use pyo3::{pymodule, wrap_pyfunction, PyResult, Python};

#[pymodule]
fn kuiper(py: Python<'_>, module: &PyModule) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(compile_expression_py, py)?)?;
    module.add_class::<KuiperExpression>()?;

    module.add("KuiperError", py.get_type::<KuiperError>())?;
    module.add("KuiperCompileError", py.get_type::<KuiperError>())?;
    module.add("KuiperRuntimeError", py.get_type::<KuiperError>())?;

    Ok(())
}
