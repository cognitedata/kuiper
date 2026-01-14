use std::{
    borrow::{Borrow, Cow},
    ops::Deref,
};

use logos::Span;
use serde_json::Value;

use crate::TransformError;

use super::{numbers::JsonNumber, ExpressionExecutionState};

/// The result of an expression resolution. The signature is a little weird.
/// An expression may either return a reference to the source, or an actual value.
/// By returning references as often as possible we reduce the number of clones.
#[derive(Clone, Debug)]
pub enum ResolveResult<'a> {
    /// An owned JSON value.
    Owned(Value),
    /// A borrowed JSON value.
    Borrowed(&'a Value),
}

impl<'a> ResolveResult<'a> {
    /// Convert the resolve result into an owned JSON value,
    /// cloning if necessary.
    pub fn into_owned(self) -> Value {
        match self {
            Self::Owned(v) => v,
            Self::Borrowed(b) => b.clone(),
        }
    }

    /// Check whether the value is truthy, i.e. whether it is not null or false.
    pub fn as_bool(&self) -> bool {
        !matches!(self.deref(), Value::Null | Value::Bool(false))
    }

    pub(crate) fn try_as_number(
        &self,
        desc: &str,
        span: &Span,
    ) -> Result<JsonNumber, TransformError> {
        get_number_from_value(desc, self, span)
    }

    /// Try to convert the resolve reslult into a string.
    pub fn try_into_string(self, desc: &str, span: &Span) -> Result<Cow<'a, str>, TransformError> {
        match self {
            Self::Owned(v) => get_string_from_value_owned(desc, v, span),
            Self::Borrowed(v) => get_string_from_value(desc, v, span),
        }
    }

    pub(crate) fn map_clone_string<'b: 'a, 'c, T>(
        self,
        state: &'a mut ExpressionExecutionState<'b, 'c>,
        string: impl FnOnce(String, &'a mut ExpressionExecutionState<'b, 'c>) -> T,
        other: impl FnOnce(&Value, &'a mut ExpressionExecutionState<'b, 'c>) -> T,
    ) -> T {
        match self {
            Self::Owned(Value::String(s)) => string(s, state),
            Self::Borrowed(Value::String(s)) => string(s.to_string(), state),
            c => other(c.as_ref(), state),
        }
    }

    /// Try to convert the resolve result into a string slice.
    pub fn try_as_string<'b: 'a>(
        &'b self,
        desc: &str,
        span: &Span,
    ) -> Result<Cow<'a, str>, TransformError> {
        match &self {
            Self::Owned(ref v) | Self::Borrowed(&ref v) => get_string_from_value(desc, v, span),
        }
    }
}

impl Deref for ResolveResult<'_> {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(v) => v,
            Self::Borrowed(v) => v,
        }
    }
}

impl AsRef<Value> for ResolveResult<'_> {
    fn as_ref(&self) -> &Value {
        match self {
            Self::Owned(v) => v,
            Self::Borrowed(v) => v,
        }
    }
}

impl Borrow<Value> for ResolveResult<'_> {
    fn borrow(&self) -> &Value {
        self
    }
}

impl From<bool> for ResolveResult<'_> {
    fn from(value: bool) -> Self {
        Self::Owned(Value::Bool(value))
    }
}

impl From<i64> for ResolveResult<'_> {
    fn from(value: i64) -> Self {
        Self::Owned(Value::from(value))
    }
}

impl From<u64> for ResolveResult<'_> {
    fn from(value: u64) -> Self {
        Self::Owned(Value::from(value))
    }
}

impl From<f64> for ResolveResult<'_> {
    fn from(value: f64) -> Self {
        Self::Owned(Value::from(value))
    }
}

impl From<String> for ResolveResult<'_> {
    fn from(value: String) -> Self {
        Self::Owned(Value::from(value))
    }
}

/// Convert a JSON value into a string. May return a direct reference to the JSON string itself if it is already a string.
/// `desc` is a description of the expression executing this, typically the name of a function or operator.
/// `val` is the value to be converted.
/// `span` is the span of the expression executing this, all expressions should store their own span.
/// `id` is the ID of the upper level transform running this, passed along with the state.
///
/// We use these to construct errors if the transform fails.
fn get_string_from_value<'a>(
    desc: &str,
    val: &'a Value,
    span: &Span,
) -> Result<Cow<'a, str>, TransformError> {
    match val {
        Value::Null => Ok(Cow::Borrowed("")),
        Value::Bool(n) => Ok(Cow::Borrowed(match n {
            true => "true",
            false => "false",
        })),
        Value::Number(n) => Ok(Cow::Owned(n.to_string())),
        Value::String(s) => Ok(Cow::Borrowed(s)),
        _ => Err(TransformError::new_incorrect_type(
            desc,
            "string or number",
            TransformError::value_desc(val),
            span,
        )),
    }
}

fn get_string_from_value_owned<'a>(
    desc: &str,
    val: Value,
    span: &Span,
) -> Result<Cow<'a, str>, TransformError> {
    match val {
        Value::Null => Ok(Cow::Borrowed("")),
        Value::Bool(n) => Ok(Cow::Borrowed(match n {
            true => "true",
            false => "false",
        })),
        Value::Number(n) => Ok(Cow::Owned(n.to_string())),
        Value::String(s) => Ok(Cow::Owned(s)),
        _ => Err(TransformError::new_incorrect_type(
            desc,
            "string or number",
            TransformError::value_desc(&val),
            span,
        )),
    }
}

/// Convenient method to convert a Value into a JsonNumber, our internal representation of numbers in JSON. Used in some math functions.
/// `desc` is a description of the expression executing this, typically the name of a function or operator.
/// `val` is the value to be converted.
/// `span` is the span of the expression executing this, all expressions should store their own span.
/// `id` is the ID of the upper level transform running this, passed along with the state.
///
/// We use these to construct errors if the transform fails.
fn get_number_from_value(
    desc: &str,
    val: &Value,
    span: &Span,
) -> Result<JsonNumber, TransformError> {
    let v = match val {
        Value::Number(n) => n,
        _ => {
            return Err(TransformError::new_incorrect_type(
                desc,
                "number",
                TransformError::value_desc(val),
                span,
            ))
        }
    };
    Ok(v.into())
}
