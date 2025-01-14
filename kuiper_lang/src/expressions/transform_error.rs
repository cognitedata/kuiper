use std::fmt::Display;

use logos::Span;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub struct TransformErrorData {
    pub span: Span,
    pub desc: String,
}

impl Display for TransformErrorData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}..{}", self.desc, self.span.start, self.span.end)
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
    #[error("Too many operations: the transform expression was terminated because it exceeded the operation limit")]
    OperationLimitExceeded,
}

impl TransformError {
    pub(crate) fn new_incorrect_type(
        desc: &str,
        expected: &str,
        actual: &str,
        span: &Span,
    ) -> Self {
        Self::IncorrectTypeInField(TransformErrorData {
            desc: format!("{desc}. Got {actual}, expected {expected}"),
            span: span.clone(),
        })
    }

    pub(crate) fn new_source_missing(name: String, span: &Span) -> Self {
        Self::SourceMissingError(TransformErrorData {
            desc: name,
            span: span.clone(),
        })
    }

    pub(crate) fn new_conversion_failed(desc: impl Into<String>, span: &Span) -> Self {
        Self::ConversionFailed(TransformErrorData {
            desc: desc.into(),
            span: span.clone(),
        })
    }

    pub(crate) fn new_invalid_operation(desc: String, span: &Span) -> Self {
        Self::InvalidOperation(TransformErrorData {
            desc,
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

    pub fn span(&self) -> Option<Span> {
        match self {
            TransformError::SourceMissingError(x) => Some(x.span.clone()),
            TransformError::IncorrectTypeInField(x) => Some(x.span.clone()),
            TransformError::ConversionFailed(x) => Some(x.span.clone()),
            TransformError::InvalidOperation(x) => Some(x.span.clone()),
            TransformError::OperationLimitExceeded => None,
        }
    }

    pub fn message(&self) -> String {
        match self {
            TransformError::SourceMissingError(transform_error_data) => {
                format!("Source {} does not exist", transform_error_data.desc)
            }
            TransformError::IncorrectTypeInField(transform_error_data) => {
                transform_error_data.desc.clone()
            }
            TransformError::ConversionFailed(transform_error_data) => {
                transform_error_data.desc.clone()
            }
            TransformError::InvalidOperation(transform_error_data) => {
                transform_error_data.desc.clone()
            }
            TransformError::OperationLimitExceeded => {
                "Too many operations: the transform expression was terminated because it exceeded the operation limit".to_string()
            }
        }
    }
}
