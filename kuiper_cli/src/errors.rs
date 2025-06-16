use kuiper_lang::types::TypeError;
use kuiper_lang::{CompileError, PrettyError, TransformError};
use std::fmt::{Display, Formatter};
use std::io;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum KuiperCliError {
    JsonError(serde_json::Error),
    IoError(io::Error),
    ErrorMessage(String),
    CompileError(CompileError),
    TransformError(TransformError),
    Utf8Error(FromUtf8Error),
    FormatError(PrettyError),
    TypeError(TypeError),
}

impl Display for KuiperCliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            KuiperCliError::JsonError(e) => e.fmt(f),
            KuiperCliError::IoError(e) => e.fmt(f),
            KuiperCliError::ErrorMessage(e) => e.fmt(f),
            KuiperCliError::CompileError(e) => e.fmt(f),
            KuiperCliError::TransformError(e) => e.fmt(f),
            KuiperCliError::Utf8Error(e) => e.fmt(f),
            KuiperCliError::FormatError(e) => e.fmt(f),
            KuiperCliError::TypeError(e) => e.fmt(f),
        }
    }
}

impl From<serde_json::Error> for KuiperCliError {
    fn from(value: serde_json::Error) -> Self {
        KuiperCliError::JsonError(value)
    }
}

impl From<io::Error> for KuiperCliError {
    fn from(value: io::Error) -> Self {
        KuiperCliError::IoError(value)
    }
}

impl From<&str> for KuiperCliError {
    fn from(value: &str) -> Self {
        KuiperCliError::ErrorMessage(String::from(value))
    }
}

impl From<CompileError> for KuiperCliError {
    fn from(value: CompileError) -> Self {
        KuiperCliError::CompileError(value)
    }
}

impl From<TransformError> for KuiperCliError {
    fn from(value: TransformError) -> Self {
        KuiperCliError::TransformError(value)
    }
}

impl From<FromUtf8Error> for KuiperCliError {
    fn from(value: FromUtf8Error) -> Self {
        KuiperCliError::Utf8Error(value)
    }
}

impl From<PrettyError> for KuiperCliError {
    fn from(value: PrettyError) -> Self {
        KuiperCliError::FormatError(value)
    }
}

impl From<TypeError> for KuiperCliError {
    fn from(value: TypeError) -> Self {
        KuiperCliError::TypeError(value)
    }
}
