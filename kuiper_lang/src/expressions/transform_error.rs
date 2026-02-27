use core::fmt::Display;

use alloc::{borrow::ToOwned, string::ToString};
use logos::Span;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
/// Data associated with a transform error.
pub struct TransformErrorData {
    /// The span in the source code where the error occurred.
    pub span: Span,
    /// A description of the error.
    pub desc: crate::String,
}

impl Display for TransformErrorData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} at {}..{}", self.desc, self.span.start, self.span.end)
    }
}

/// Error returned by a failed transform execution,
/// or by the optimizer.
///
/// These are typically runtime type errors, or other invalid operations.
#[derive(Debug, Error)]
pub enum TransformError {
    /// The source value does not exist.
    #[error("Source does not exist: {0}")]
    SourceMissingError(TransformErrorData),
    /// A field had an incorrect type.
    #[error("{0}")]
    IncorrectTypeInField(TransformErrorData),
    /// A conversion between types failed.
    #[error("{0}")]
    ConversionFailed(TransformErrorData),
    /// An invalid operation was performed.
    #[error("{0}")]
    InvalidOperation(TransformErrorData),
    /// The operation limit was exceeded.
    #[error("Too many operations: the transform expression was terminated because it exceeded the operation limit")]
    OperationLimitExceeded,
}

impl TransformError {
    /// Create a new TransformError for an incorrect type in a field.
    /// `desc` should be a description of the field and the error, e.g. "Field 'x' must be a string".
    /// `expected` and `actual` should be human-readable descriptions of the expected and actual
    /// types, e.g. "string" and "number".
    pub fn new_incorrect_type(desc: &str, expected: &str, actual: &str, span: &Span) -> Self {
        Self::IncorrectTypeInField(TransformErrorData {
            desc: alloc::format!("{desc}. Got {actual}, expected {expected}"),
            span: span.clone(),
        })
    }

    pub(crate) fn new_source_missing(name: crate::String, span: &Span) -> Self {
        Self::SourceMissingError(TransformErrorData {
            desc: name,
            span: span.clone(),
        })
    }

    /// Create a new TransformError for a failed conversion.
    /// `desc` should be a description of where this happened, e.g. "my_function".
    pub fn new_conversion_failed(desc: impl Into<crate::String>, span: &Span) -> Self {
        Self::ConversionFailed(TransformErrorData {
            desc: desc.into(),
            span: span.clone(),
        })
    }

    /// Create a new TransformError for an invalid operation.
    /// `desc` should be a description of the operation, e.g. "Cannot add a string and a number".
    pub fn new_invalid_operation(desc: crate::String, span: &Span) -> Self {
        Self::InvalidOperation(TransformErrorData {
            desc,
            span: span.clone(),
        })
    }

    pub(crate) fn new_arith_overflow(span: &Span) -> Self {
        Self::InvalidOperation(TransformErrorData {
            desc: "Arithmetic overflow".to_owned(),
            span: span.clone(),
        })
    }

    /// Utility function to get a human-readable description of a serde_json::Value, for error messages.
    pub fn value_desc(val: &Value) -> &str {
        match val {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    /// Get the span in the source code where the error occurred, if available.
    pub fn span(&self) -> Option<Span> {
        match self {
            TransformError::SourceMissingError(x) => Some(x.span.clone()),
            TransformError::IncorrectTypeInField(x) => Some(x.span.clone()),
            TransformError::ConversionFailed(x) => Some(x.span.clone()),
            TransformError::InvalidOperation(x) => Some(x.span.clone()),
            TransformError::OperationLimitExceeded => None,
        }
    }

    /// Get a human-readable message describing the error.
    pub fn message(&self) -> crate::String {
        match self {
            TransformError::SourceMissingError(transform_error_data) => {
               alloc::format!("Source {} does not exist", transform_error_data.desc)
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
