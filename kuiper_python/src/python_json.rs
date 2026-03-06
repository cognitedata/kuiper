use std::collections::HashMap;

use kuiper_lang::{Span, TransformError};
use pyo3::{
    exceptions::PyTypeError,
    types::{PyAnyMethods, PyBool, PyDict, PyFloat, PyInt, PyList, PyListMethods, PyString},
    FromPyObject, Py, PyAny, PyErr, PyResult, Python,
};
use serde_json::{Map, Number, Value};

pub(crate) struct PythonNone {}

impl<'a, 'py> FromPyObject<'a, 'py> for PythonNone {
    type Error = PyErr;

    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if obj.is_none() {
            Ok(PythonNone {})
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Expected None",
            ))
        }
    }
}

pub enum ConversionError {
    PythonError(PyErr),
    PlainMessage(String),
}

impl From<PyErr> for ConversionError {
    fn from(value: PyErr) -> Self {
        Self::PythonError(value)
    }
}

impl ConversionError {
    pub fn into_transform_error(self, span: &Span) -> TransformError {
        match self {
            ConversionError::PythonError(e) => {
                TransformError::new_conversion_failed(e.to_string(), span)
            }
            ConversionError::PlainMessage(s) => TransformError::new_conversion_failed(s, span),
        }
    }

    pub fn into_python_error(self) -> PyErr {
        match self {
            ConversionError::PythonError(e) => e,
            ConversionError::PlainMessage(s) => PyErr::new::<PyTypeError, _>(s),
        }
    }
}

#[derive(FromPyObject)]
pub(crate) enum PythonJson {
    // This order is important, as it is the order PyO3 will try to convert the type to. That's why e.g. bool is
    // before float - since a bool can also be coerced into 0.0 and 1.0.
    #[pyo3(transparent, annotation = "bool")]
    Bool(bool),
    #[pyo3(transparent, annotation = "int")]
    Int(i64),
    #[pyo3(transparent, annotation = "float")]
    Float(f64),
    #[pyo3(transparent, annotation = "str")]
    Str(String),
    #[pyo3(transparent, annotation = "None")]
    None(PythonNone),
    #[pyo3(transparent, annotation = "list")]
    List(Vec<PythonJson>),
    #[pyo3(transparent, annotation = "dict")]
    Dict(HashMap<String, PythonJson>),
}

impl PythonJson {
    pub fn into_value(self) -> Result<Value, ConversionError> {
        match self {
            PythonJson::Bool(b) => Ok(Value::Bool(b)),

            PythonJson::Int(i) => Ok(Value::Number(Number::from(i))),

            PythonJson::Float(f) => Ok(Value::Number(Number::from_f64(f).ok_or(
                ConversionError::PlainMessage("Invalid float value".to_string()),
            )?)),

            PythonJson::Str(s) => Ok(Value::String(s)),

            PythonJson::None(_none) => Ok(Value::Null),

            PythonJson::List(list) => {
                let values = list
                    .into_iter()
                    .map(|item| item.into_value())
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::Array(values))
            }

            PythonJson::Dict(dict) => {
                let values = dict
                    .into_iter()
                    .map(|(k, v)| Ok((k, v.into_value()?)))
                    .collect::<Result<Map<_, _>, ConversionError>>()?;
                Ok(Value::Object(values))
            }
        }
    }

    pub fn into_python(self, py: Python) -> Py<PyAny> {
        match self {
            PythonJson::Bool(b) => PyBool::new(py, b).to_owned().into(),
            PythonJson::Int(i) => PyInt::new(py, i).into(),
            PythonJson::Float(f) => PyFloat::new(py, f).into(),
            PythonJson::Str(s) => PyString::new(py, &s).into(),
            PythonJson::None(_) => py.None(),
            PythonJson::List(list) => {
                let py_list = PyList::empty(py);
                for item in list {
                    py_list.append(item.into_python(py)).unwrap();
                }
                py_list.into()
            }
            PythonJson::Dict(dict) => {
                let py_dict = PyDict::new(py);
                for (k, v) in dict {
                    py_dict.set_item(k, v.into_python(py)).unwrap();
                }
                py_dict.into()
            }
        }
    }

    pub fn from_value(value: Value) -> Result<Self, ConversionError> {
        match value {
            Value::Null => Ok(PythonJson::None(PythonNone {})),
            Value::Bool(b) => Ok(PythonJson::Bool(b)),
            Value::Number(number) => {
                if let Some(i) = number.as_i64() {
                    Ok(PythonJson::Int(i))
                } else if let Some(f) = number.as_f64() {
                    Ok(PythonJson::Float(f))
                } else {
                    Err(ConversionError::PlainMessage(
                        "Invalid number value".to_string(),
                    ))
                }
            }
            Value::String(s) => Ok(PythonJson::Str(s)),
            Value::Array(values) => {
                let python_values = values
                    .into_iter()
                    .map(PythonJson::from_value)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(PythonJson::List(python_values))
            }
            Value::Object(map) => {
                let python_map = map
                    .into_iter()
                    .map(|(k, v)| Ok((k, PythonJson::from_value(v)?)))
                    .collect::<Result<HashMap<_, _>, ConversionError>>()?;
                Ok(PythonJson::Dict(python_map))
            }
        }
    }
}
