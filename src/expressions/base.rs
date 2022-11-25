use std::{collections::HashMap, fmt::Display};

use serde_json::{Number, Value};

use super::{transform_error::TransformError, OpExpression, PowFunction, SelectorExpression};

pub struct ExpressionExecutionState {
    pub data: HashMap<String, Value>,
}

pub trait Expression: Display {
    fn resolve(&self, state: &ExpressionExecutionState) -> Result<Value, TransformError>;
}

pub enum FunctionType {
    Pow(PowFunction),
}

pub enum ExpressionType {
    Constant(Constant),
    Operator(OpExpression),
    Selector(SelectorExpression),
    Function(FunctionType),
}

impl Display for ExpressionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionType::Constant(e) => e.fmt(f),
            ExpressionType::Operator(e) => e.fmt(f),
            ExpressionType::Selector(e) => e.fmt(f),
            ExpressionType::Function(e) => e.fmt(f),
        }
    }
}

impl Expression for ExpressionType {
    fn resolve(&self, state: &ExpressionExecutionState) -> Result<Value, TransformError> {
        match self {
            ExpressionType::Constant(e) => e.resolve(state),
            ExpressionType::Operator(e) => e.resolve(state),
            ExpressionType::Selector(e) => e.resolve(state),
            ExpressionType::Function(e) => e.resolve(state),
        }
    }
}

impl Display for FunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pow(p) => p.fmt(f),
        }
    }
}

impl Expression for FunctionType {
    fn resolve(&self, state: &ExpressionExecutionState) -> Result<Value, TransformError> {
        match self {
            Self::Pow(p) => p.resolve(state),
        }
    }
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
        let val = Number::from_f64(v).map(|n| Value::Number(n));
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
    return v.as_f64().ok_or_else(|| {
        TransformError::ConversionFailed(format!(
            "Failed to convert field into number for operator {}",
            desc
        ))
    });
}
