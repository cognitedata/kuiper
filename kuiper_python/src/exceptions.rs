use pyo3::{types::PyAnyMethods, PyErr, Python};

pub fn raise_kuiper_error(
    error: &str,
    message: String,
    start: Option<usize>,
    end: Option<usize>,
) -> PyErr {
    Python::with_gil(|py| {
        let errors = py.import_bound("kuiper").unwrap();
        let exception = errors.getattr(error).unwrap();
        PyErr::from_value_bound(exception.call1((message, start, end)).unwrap())
    })
}
