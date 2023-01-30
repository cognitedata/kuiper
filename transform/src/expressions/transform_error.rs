use logos::Span;
use serde_json::Value;

#[derive(Debug)]
pub struct TransformErrorData {
    pub id: String,
    pub span: Span,
    pub desc: String,
}

#[derive(Debug)]
pub enum TransformError {
    SourceMissingError(TransformErrorData),
    IncorrectTypeInField(TransformErrorData),
    ConversionFailed(TransformErrorData),
    InvalidOperation(TransformErrorData),
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
