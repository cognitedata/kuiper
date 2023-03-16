use logos::Span;
use serde_json::{Number, Value};
use std::{borrow::Cow, collections::HashMap, fmt::Display};

use crate::{parse::ParserError, program::TransformOrInput};

use super::{
    functions::*, numbers::JsonNumber, operator::UnaryOpExpression,
    transform_error::TransformError, ArrayExpression, ObjectExpression, OpExpression,
    SelectorExpression,
};

use transform_macros::PassThrough;

/// State for expression execution. This struct is constructed for each expression.
/// Notably lifetime heavy. `'a` is the lifetime of the input data.
/// `'b` is the lifetime of the transform execution, so the temporary data in the transform.
pub struct ExpressionExecutionState<'data, 'exec> {
    data: &'exec HashMap<TransformOrInput, &'data Value>,
    map: &'exec HashMap<String, TransformOrInput>,
    pub id: &'exec str,
}

impl<'data, 'exec> ExpressionExecutionState<'data, 'exec> {
    /// Try to obtain a value with the given key from the state.
    #[inline]
    pub fn get_value(&self, key: &str) -> Option<&'data Value> {
        let v = self.map.iter().find(|(k, _)| *k == key);
        let Some((_, v)) = v else {
            return None;
        };
        self.data.iter().find(|(k, _v)| *k == v).map(|(_, v)| *v)
    }

    pub fn new(
        data: &'exec HashMap<TransformOrInput, &'data Value>,
        map: &'exec HashMap<String, TransformOrInput>,
        id: &'exec str,
    ) -> Self {
        Self { data, map, id }
    }
}

/// Trait for top-level expressions.
/// The three lifetimes represent the three separate lifetimes in transform execution:
/// 'a is the lifetime of the transform itself
/// 'b is the lifetime of the current execution of the transform.
/// 'c is the lifetime of the execution of the program itself, so it goes beyond this transform.
///
/// In simple terms
///
/// 'a
/// start program execution
///     'c
///     for transform in program
///         for entry in inputs
///             'b
pub trait Expression<'a: 'c, 'b, 'c>: Display {
    /// Resolve an expression.
    fn resolve(
        &'a self,
        state: &'b ExpressionExecutionState<'c, 'b>,
    ) -> Result<ResolveResult<'c>, TransformError>;
}

/// Additional trait for expressions, separate from Expression to make it easier to implement in macros
pub trait ExpressionMeta<'a: 'c, 'b, 'c>: Expression<'a, 'b, 'c> {
    fn num_children(&self) -> usize;

    fn get_child(&self, idx: usize) -> Option<&ExpressionType>;

    fn get_child_mut(&mut self, idx: usize) -> Option<&mut ExpressionType>;

    fn set_child(&mut self, idx: usize, item: ExpressionType);
}

/// A function expression, new functions must be added here.
#[derive(PassThrough, Debug, Clone)]
#[pass_through(fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result, "", Display)]
#[pass_through(fn resolve(&'a self, state: &'b ExpressionExecutionState<'c, 'b>) -> Result<ResolveResult<'c>, TransformError>, "", Expression<'a, 'b, 'c>, where 'a: 'c)]
#[pass_through(fn num_children(&self) -> usize, "", ExpressionMeta<'a, 'b, 'c>, where 'a: 'c)]
#[pass_through(fn get_child(&self, idx: usize) -> Option<&ExpressionType>, "", ExpressionMeta<'a>)]
#[pass_through(fn get_child_mut(&mut self, idx: usize) -> Option<&mut ExpressionType>, "", ExpressionMeta<'a>)]
#[pass_through(fn set_child(&mut self, idx: usize, item: ExpressionType), "", ExpressionMeta<'a>)]
pub enum FunctionType {
    Pow(PowFunction),
    Log(LogFunction),
    Atan2(Atan2Function),
    Floor(FloorFunction),
    Ceil(CeilFunction),
    Round(RoundFunction),
    Concat(ConcatFunction),
    String(StringFunction),
    Int(IntFunction),
    Float(FloatFunction),
    If(IfFunction),
    ToUnixTime(ToUnixTimeFunction),
    Case(CaseFunction),
    Pairs(PairsFunction),
    Flatten(FlattenFunction),
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
        "round" => FunctionType::Round(RoundFunction::new(args, pos)?),
        "concat" => FunctionType::Concat(ConcatFunction::new(args, pos)?),
        "string" => FunctionType::String(StringFunction::new(args, pos)?),
        "int" => FunctionType::Int(IntFunction::new(args, pos)?),
        "float" => FunctionType::Float(FloatFunction::new(args, pos)?),
        "if" => FunctionType::If(IfFunction::new(args, pos)?),
        "to_unix_timestamp" => FunctionType::ToUnixTime(ToUnixTimeFunction::new(args, pos)?),
        "case" => FunctionType::Case(CaseFunction::new(args, pos)?),
        "pairs" => FunctionType::Pairs(PairsFunction::new(args, pos)?),
        "flatten" => FunctionType::Flatten(FlattenFunction::new(args, pos)?),
        _ => return Err(ParserError::unrecognized_function(pos, name)),
    };
    Ok(ExpressionType::Function(expr))
}

/// The main expression type. All expressions must be included here.
#[derive(PassThrough, Debug, Clone)]
#[pass_through(fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result, "", Display)]
#[pass_through(fn resolve(&'a self, state: &'b ExpressionExecutionState<'c, 'b>) -> Result<ResolveResult<'c>, TransformError>, "", Expression<'a, 'b, 'c>, where 'a: 'c)]
#[pass_through(fn num_children(&self) -> usize, "", ExpressionMeta<'a, 'b, 'c>, where 'a: 'c)]
#[pass_through(fn get_child(&self, idx: usize) -> Option<&ExpressionType>, "", ExpressionMeta<'a>)]
#[pass_through(fn get_child_mut(&mut self, idx: usize) -> Option<&mut ExpressionType>, "", ExpressionMeta<'a>)]
#[pass_through(fn set_child(&mut self, idx: usize, item: ExpressionType), "", ExpressionMeta<'a>)]
pub enum ExpressionType {
    Constant(Constant),
    Operator(OpExpression),
    UnaryOperator(UnaryOpExpression),
    Selector(SelectorExpression),
    Function(FunctionType),
    Array(ArrayExpression),
    Object(ObjectExpression),
}

/// The result of an expression resolution. The signature is a little weird.
/// An expression may either return a reference to the source, or an actual value.
/// By returning references as often as possible we reduce the number of clones.
pub type ResolveResult<'a> = Cow<'a, Value>;

#[derive(Debug, Clone)]
/// A constant expression. This always resolves to a reference to its value.
pub struct Constant {
    val: Value,
}

impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.val)
    }
}

impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for Constant {
    fn resolve(
        &'a self,
        _state: &'b ExpressionExecutionState<'c, 'b>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        Ok(ResolveResult::Borrowed(&self.val))
    }
}

impl<'a: 'c, 'b, 'c> ExpressionMeta<'a, 'b, 'c> for Constant {
    fn num_children(&self) -> usize {
        0
    }

    fn get_child(&self, _idx: usize) -> Option<&ExpressionType> {
        None
    }

    fn get_child_mut(&mut self, _idx: usize) -> Option<&mut ExpressionType> {
        None
    }

    fn set_child(&mut self, _idx: usize, _item: ExpressionType) {}
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

    pub fn new(val: Value) -> Self {
        Self { val }
    }

    pub fn new_string(v: String) -> Self {
        Self {
            val: Value::String(v),
        }
    }

    pub fn new_null() -> Self {
        Self { val: Value::Null }
    }

    pub fn new_bool(val: bool) -> Self {
        Self {
            val: Value::Bool(val),
        }
    }
}

/// Convenient method to convert a Value into a JsonNumber, our internal representation of numbers in JSON. Used in some math functions.
/// `desc` is a description of the expression executing this, typically the name of a function or operator.
/// `val` is the value to be converted.
/// `span` is the span of the expression executing this, all expressions should store their own span.
/// `id` is the ID of the upper level transform running this, passed along with the state.
///
/// We use these to construct errors if the transform fails.
pub(crate) fn get_number_from_value(
    desc: &str,
    val: &Value,
    span: &Span,
    id: &str,
) -> Result<JsonNumber, TransformError> {
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
    v.as_u64()
        .map(JsonNumber::PosInteger)
        .or_else(|| v.as_i64().map(JsonNumber::NegInteger))
        .or_else(|| v.as_f64().map(JsonNumber::Float))
        .ok_or_else(|| {
            TransformError::new_conversion_failed(
                format!("Failed to convert input into number for operator {desc}"),
                span,
                id,
            )
        })
}

/// Convert a JSON value into a string. May return a direct reference to the JSON string itself if it is already a string.
/// `desc` is a description of the expression executing this, typically the name of a function or operator.
/// `val` is the value to be converted.
/// `span` is the span of the expression executing this, all expressions should store their own span.
/// `id` is the ID of the upper level transform running this, passed along with the state.
///
/// We use these to construct errors if the transform fails.
pub(crate) fn get_string_from_value<'a>(
    desc: &str,
    val: &'a Value,
    span: &Span,
    id: &str,
) -> Result<Cow<'a, String>, TransformError> {
    match val {
        Value::Null => Ok(Cow::Owned("".to_string())),
        Value::Bool(n) => Ok(Cow::Owned(match n {
            true => "true".to_string(),
            false => "false".to_string(),
        })),
        Value::Number(n) => Ok(Cow::Owned(n.to_string())),
        Value::String(s) => Ok(Cow::Borrowed(s)),
        _ => {
            return Err(TransformError::new_incorrect_type(
                desc,
                "string or number",
                TransformError::value_desc(val),
                span,
                id,
            ))
        }
    }
}

/// Convert the JSON value into a boolean. Cannot fail, `null` and `false` are falsy, all others are true.
pub(crate) fn get_boolean_from_value(val: &Value) -> bool {
    match val {
        Value::Null => false,
        Value::Bool(b) => *b,
        _ => true,
    }
}
