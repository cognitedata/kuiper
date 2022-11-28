use serde_json::{Number, Value};
use std::{collections::HashMap, fmt::Display};

use super::{transform_error::TransformError, OpExpression, PowFunction, SelectorExpression};

use transform_macros::{pass_through, PassThrough};

pub struct ExpressionExecutionState {
    pub data: HashMap<String, Value>,
}

pub trait Expression: Display {
    fn resolve(&self, state: &ExpressionExecutionState) -> Result<Value, TransformError>;
}

#[derive(PassThrough)]
#[pass_through(fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result, "", Display)]
#[pass_through(fn resolve(&self, state: &ExpressionExecutionState) -> Result<Value, TransformError>, "", Expression)]
pub enum FunctionType {
    Pow(PowFunction),
}

#[derive(PassThrough)]
#[pass_through(fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result, "", Display)]
#[pass_through(fn resolve(&self, state: &ExpressionExecutionState) -> Result<Value, TransformError>, "", Expression)]
pub enum ExpressionType {
    Constant(Constant),
    Operator(OpExpression),
    Selector(SelectorExpression),
    Function(FunctionType),
}

pub struct Constant {
    val: Value,
}

impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.val)
    }
}

impl Expression for Constant {
    fn resolve(&self, _state: &ExpressionExecutionState) -> Result<Value, TransformError> {
        Ok(self.val.clone())
    }
}

impl Constant {
    pub fn try_new_f64(v: f64) -> Option<Self> {
        let val = Number::from_f64(v).map(Value::Number);
        val.map(|v| Self { val: v })
    }

    pub fn try_new_string(v: String) -> Self {
        Self {
            val: Value::String(v),
        }
    }
}

pub fn get_number_from_value(desc: &str, val: Value) -> Result<f64, TransformError> {
    let v = match val {
        Value::Number(n) => n,
        _ => return Err(TransformError::new_incorrect_type(desc, "number", &val)),
    };
    v.as_f64().ok_or_else(|| {
        TransformError::ConversionFailed(format!(
            "Failed to convert field into number for operator {}",
            desc
        ))
    })
}
