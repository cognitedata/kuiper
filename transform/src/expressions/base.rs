use logos::Span;
use serde_json::{Number, Value};
use std::{collections::HashMap, fmt::Display};

use crate::parse::ParserError;

use super::{
    function::*, transform_error::TransformError, ArrayExpression, OpExpression, PowFunction,
    SelectorExpression,
};

use transform_macros::{pass_through, PassThrough};

pub struct ExpressionExecutionState {
    pub data: HashMap<String, Value>,
}

pub trait Expression<'a>: Display {
    fn resolve(
        &'a self,
        state: &'a ExpressionExecutionState,
    ) -> Result<ResolveResult<'a>, TransformError>;
}

#[derive(PassThrough)]
#[pass_through(fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result, "", Display)]
#[pass_through(fn resolve(&'a self, state: &'a ExpressionExecutionState) -> Result<ResolveResult<'a>, TransformError>, "", Expression<'a>)]
pub enum FunctionType {
    Pow(PowFunction),
    Log(LogFunction),
    Atan2(Atan2Function),
    Floor(FloorFunction),
    Ceil(CeilFunction),
}

pub fn get_function_expression(
    pos: Span,
    name: &str,
    args: Vec<ExpressionType>,
) -> Result<ExpressionType, ParserError> {
    let expr = match name {
        "pow" => FunctionType::Pow(PowFunction::new(args, pos)?),
        "log" => FunctionType::Log(LogFunction::new(args, pos)?),
        "atan2" => FunctionType::Atan2(Atan2Function::new(args, pos)?),
        "floor" => FunctionType::Floor(FloorFunction::new(args, pos)?),
        "ceil" => FunctionType::Ceil(CeilFunction::new(args, pos)?),
        _ => return Err(ParserError::incorrect_symbol(pos, name.to_string())),
    };
    Ok(ExpressionType::Function(expr))
}

#[derive(PassThrough)]
#[pass_through(fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result, "", Display)]
#[pass_through(fn resolve(&'a self, state: &'a ExpressionExecutionState) -> Result<ResolveResult<'a>, TransformError>, "", Expression<'a>)]
pub enum ExpressionType {
    Constant(Constant),
    Operator(OpExpression),
    Selector(SelectorExpression),
    Function(FunctionType),
    Array(ArrayExpression),
}

pub enum ResolveResult<'a> {
    Reference(&'a Value),
    Value(Value),
}

impl<'a> ResolveResult<'a> {
    pub fn as_ref(&self) -> &Value {
        match self {
            Self::Reference(r) => r,
            Self::Value(v) => v,
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

impl<'a> Expression<'a> for Constant {
    fn resolve(
        &'a self,
        _state: &ExpressionExecutionState,
    ) -> Result<ResolveResult<'a>, TransformError> {
        Ok(ResolveResult::Reference(&self.val))
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

pub fn get_number_from_value(desc: &str, val: &Value, span: &Span) -> Result<f64, TransformError> {
    let v = match val {
        Value::Number(n) => n,
        _ => return Err(TransformError::new_incorrect_type(desc, "number", &val)),
    };
    v.as_f64().ok_or_else(|| {
        TransformError::ConversionFailed(format!(
            "Failed to convert field into number for operator {} at {}",
            desc, span.start
        ))
    })
}
