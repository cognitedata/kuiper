use serde_json::Value;

pub enum TransformError {
    GenericFailure(String),
    SourceMissingError(String),
    IncorrectTypeInField(String),
    ConversionFailed(String),
}

impl TransformError {
    pub fn new_incorrect_type(descriptor: &str, expected: &str, actual: &Value) -> Self {
        Self::IncorrectTypeInField(format!(
            "Incorrect type in operator {}. Got {}, expected {}",
            descriptor,
            Self::value_desc(actual),
            expected
        ))
    }

    fn value_desc(val: &Value) -> &str {
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
