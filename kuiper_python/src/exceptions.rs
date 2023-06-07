use pyo3::create_exception;
use pyo3::exceptions::PyException;

create_exception!(kuiper, KuiperError, PyException);

create_exception!(kuiper, KuiperCompileError, KuiperError);
create_exception!(kuiper, KuiperRuntimeError, KuiperError);
