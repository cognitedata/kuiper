use std::fmt::Display;

use logos::Span;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub struct TransformErrorData {
    pub id: String,
    pub span: Span,
    pub desc: String,
}

impl Display for TransformErrorData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}. ID: {} at {}..{}",
            self.desc, self.id, self.span.start, self.span.end
        )
    }
}

/// Error returned by a failed transform execution,
/// or by the optimizer.
///
/// These are typically runtime type errors, or other invalid operations.
#[derive(Debug, Error)]
pub enum TransformError {
    #[error("Source does not exist: {0}")]
    SourceMissingError(TransformErrorData),
    #[error("{0}")]
    IncorrectTypeInField(TransformErrorData),
    #[error("{0}")]
    ConversionFailed(TransformErrorData),
    #[error("{0}")]
    InvalidOperation(TransformErrorData),
    #[error("Program is invalid: {0}")]
    InvalidProgramError(String),
}

impl TransformError {
    pub(crate) fn new_incorrect_type(
        desc: &str,
        expected: &str,
        actual: &str,
        span: &Span,
        id: &str,
    ) -> Self {
        Self::IncorrectTypeInField(TransformErrorData {
            desc: format!("{desc}. Got {actual}, expected {expected}"),
            id: id.to_string(),
            span: span.clone(),
        })
    }

    pub(crate) fn new_source_missing(name: String, span: &Span, id: &str) -> Self {
        Self::SourceMissingError(TransformErrorData {
            desc: name,
            id: id.to_string(),
            span: span.clone(),
        })
    }

    pub(crate) fn new_conversion_failed(desc: String, span: &Span, id: &str) -> Self {
        Self::ConversionFailed(TransformErrorData {
            desc,
            id: id.to_string(),
            span: span.clone(),
        })
    }

    pub(crate) fn new_invalid_operation(desc: String, span: &Span, id: &str) -> Self {
        Self::InvalidOperation(TransformErrorData {
            desc,
            id: id.to_string(),
            span: span.clone(),
        })
    }

    pub(crate) fn value_desc(val: &Value) -> &str {
        match val {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}
