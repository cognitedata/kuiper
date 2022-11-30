use logos::Span;
use serde_json::{Number, Value};
use std::{collections::HashMap, fmt::Display};

use crate::{parse::ParserError, program::TransformOrInput};

use super::{
    function::*, transform_error::TransformError, ArrayExpression, OpExpression, PowFunction,
    SelectorExpression,
};

use transform_macros::{pass_through, PassThrough};

/// State for expression execution. This struct is constructed for each expression.
/// Notably lifetime heavy. `'a` is the lifetime of the input data.
/// `'b` is the lifetime of the transform execution, so the temporary data in the transform.
pub struct ExpressionExecutionState<'a, 'b>
where
    'b: 'a,
{
    data: &'b HashMap<TransformOrInput, ResolveResult<'a>>,
    map: &'b HashMap<String, TransformOrInput>,
    pub id: &'b str,
}

impl<'a, 'b> ExpressionExecutionState<'a, 'b> {
    /// Try to obtain a value with the given key from the state.
    pub fn get_value(&self, key: &str) -> Option<&'a Value> {
        self.map
            .get(key)
            .and_then(|k| self.data.get(k))
            .map(|r| match r {
                ResolveResult::Reference(rf) => *rf,
                ResolveResult::Value(v) => v,
            })
    }

    pub fn new(
        data: &'b HashMap<TransformOrInput, ResolveResult<'a>>,
        map: &'b HashMap<String, TransformOrInput>,
        id: &'b str,
    ) -> Self {
        Self { data, map, id }
    }
}

/// Trait for top-level expressions.
pub trait Expression<'a>: Display {
    /// Resolve an expression.
    fn resolve(
        &'a self,
        state: &'a ExpressionExecutionState,
    ) -> Result<ResolveResult<'a>, TransformError>;
}

/// A function expression, new functions must be added here.
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

/// Create a function expression from its name, or return a parser exception if it has the wrong number of arguments,
/// or does not exist.
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
        _ => return Err(ParserError::unrecognized_function(pos, name)),
    };
    Ok(ExpressionType::Function(expr))
}

/// The main expression type. All expressions must be included here.
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

/// The result of an expression resolution. The signature is a little weird.
/// An expression may either return a reference to the source, or an actual value.
/// By returning references as often as possible we reduce the number of clones.
#[derive(Clone)]
pub enum ResolveResult<'a> {
    Reference(&'a Value),
    Value(Value),
}

impl<'a> ResolveResult<'a> {
    /// Return the internal reference or a reference to the internal value.
    pub fn as_ref(&self) -> &Value {
        match self {
            Self::Reference(r) => r,
            Self::Value(v) => v,
        }
    }

    /// Create a value from this, either returning the internal value, or cloning the internal reference.
    pub fn into_value(self) -> Value {
        match self {
            Self::Reference(r) => r.clone(),
            Self::Value(v) => v,
        }
    }

    /// Convert into a ResolveResult::Reference.
    pub fn as_self_ref(&'a self) -> Self {
        Self::Reference(self.as_ref())
    }
}

/// A constant expression. This always resolves to a reference to its value.
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

    pub fn try_new_i64(v: i64) -> Option<Self> {
        let val = Number::try_from(v).map(Value::Number).ok();
        val.map(|v| Self { val: v })
    }

    pub fn try_new_u64(v: u64) -> Option<Self> {
        let val = Number::try_from(v).map(Value::Number).ok();
        val.map(|v| Self { val: v })
    }

    pub fn new_string(v: String) -> Self {
        Self {
            val: Value::String(v),
        }
    }

    pub fn new_null() -> Self {
        Self { val: Value::Null }
    }
}

/// Convenient method to convert a Value into a f64. Used in some math functions.
pub fn get_number_from_value(
    desc: &str,
    val: &Value,
    span: &Span,
    id: &str,
) -> Result<f64, TransformError> {
    let v = match val {
        Value::Number(n) => n,
        _ => {
            return Err(TransformError::new_incorrect_type(
                desc,
                "number",
                TransformError::value_desc(val),
                span,
                id,
            ))
        }
    };
    v.as_f64().ok_or_else(|| {
        TransformError::new_conversion_failed(
            format!("Failed to convert input into number for operator {}", desc),
            span,
            id,
        )
    })
}
