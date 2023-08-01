use pyo3::{create_exception, exceptions::PyException, import_exception};

create_exception!(kuiper, KuiperError, PyException);

create_exception!(kuiper, KuiperCompileError, KuiperError);
create_exception!(kuiper, KuiperRuntimeError, KuiperError);
