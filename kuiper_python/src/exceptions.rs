use pyo3::{types::PyAnyMethods, PyErr, Python};

pub fn raise_kuiper_error(
    error: &str,
    message: String,
    start: Option<usize>,
    end: Option<usize>,
) -> PyErr {
    Python::attach(|py| {
        let errors = py.import("kuiper").unwrap();
        let exception = errors.getattr(error).unwrap();
        PyErr::from_value(exception.call1((message, start, end)).unwrap())
    })
}
