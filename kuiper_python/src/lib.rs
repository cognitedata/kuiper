mod compiler;
mod exceptions;
mod expressions;

use crate::compiler::compile_expression_py;
use crate::expressions::KuiperExpression;
use pyo3::prelude::PyModule;
use pyo3::types::PyModuleMethods;
use pyo3::{pymodule, wrap_pyfunction_bound, Bound, PyResult, Python};

#[pymodule]
fn kuiper(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction_bound!(compile_expression_py, py)?)?;
    module.add_class::<KuiperExpression>()?;

    PyModule::from_code_bound(
        py,
        r#"
class KuiperError(Exception):
    def __init__(self, message, start, end):
        super().__init__(message)
        self.start = start
        self.end = end

class KuiperCompileError(KuiperError):
    pass

class KuiperRuntimeError(KuiperError):
    pass
"#,
        "kuiper_errors.py",
        "kuiper",
    )?;

    Ok(())
}
