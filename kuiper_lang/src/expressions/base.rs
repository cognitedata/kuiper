use logos::Span;
use serde_json::Value;
use std::{borrow::Cow, fmt::Display};

use crate::{compiler::BuildError, NULL_CONST};

use super::{
    functions::{
        distinct_by::DistinctByFunction, except::ExceptFunction, filter::FilterFunction,
        flatmap::FlatMapFunction, map::MapFunction, reduce::ReduceFunction, select::SelectFunction,
        zip::ZipFunction, *,
    },
    lambda::LambdaExpression,
    numbers::JsonNumber,
    operator::UnaryOpExpression,
    transform_error::TransformError,
    ArrayExpression, ObjectExpression, OpExpression, SelectorExpression,
};

use kuiper_lang_macros::PassThrough;

/// State for expression execution. This struct is constructed for each expression.
/// Notably lifetime heavy. `'a` is the lifetime of the input data.
/// `'b` is the lifetime of the transform execution, so the temporary data in the transform.
pub struct ExpressionExecutionState<'data, 'exec> {
    data: &'exec Vec<&'data Value>,
}

impl<'data, 'exec> ExpressionExecutionState<'data, 'exec> {
    /// Try to obtain a value with the given key from the state.
    #[inline]
    pub fn get_value(&self, key: usize) -> Option<&'data Value> {
        self.data.get(key).copied()
    }

    pub fn new(data: &'exec Vec<&'data Value>) -> Self {
        Self { data }
    }

    pub fn get_temporary_clone(&self, extra_cap: usize) -> InternalExpressionExecutionState<'data> {
        let mut data = Vec::with_capacity(self.data.len() + extra_cap);
        for elem in self.data {
            data.push(*elem);
        }
        InternalExpressionExecutionState {
            data,
            base_length: self.data.len(),
        }
    }

    pub fn get_temporary_clone_inner(
        &self,
        extra_values: impl Iterator<Item = &'data Value>,
        num_values: usize,
    ) -> InternalExpressionExecutionState<'data> {
        let mut data = Vec::with_capacity(self.data.len() + num_values);
        for elem in self.data.iter() {
            data.push(*elem);
        }
        let mut pushed = 0;
        for elem in extra_values.take(num_values) {
            data.push(elem);
            pushed += 1;
        }
        if pushed < num_values {
            for _ in pushed..num_values {
                data.push(&NULL_CONST);
            }
        }

        InternalExpressionExecutionState {
            data,
            base_length: self.data.len(),
        }
    }
}

#[derive(Debug)]
pub struct InternalExpressionExecutionState<'data> {
    pub data: Vec<&'data Value>,
    pub base_length: usize,
}

impl<'data> InternalExpressionExecutionState<'data> {
    pub fn get_temp_state<'slf>(&'slf self) -> ExpressionExecutionState<'data, 'slf> {
        ExpressionExecutionState { data: &self.data }
    }
}

#[macro_export]
macro_rules! with_temp_values {
    ($inner:ident, $inner_state:ident, $values:expr, $func:expr) => {{
        let len = $values.len();
        for val in $values {
            $inner.data.push(val);
        }
        let $inner_state = $inner.get_temp_state();
        let r = $func;
        for _ in 0..len {
            $inner.data.pop();
        }
        r
    }};
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
pub trait Expression<'a: 'c, 'c>: Display {
    const IS_DETERMINISTIC: bool = true;
    /// Resolve an expression.
    fn resolve(
        &'a self,
        state: &ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError>;

    fn get_is_deterministic(&self) -> bool {
        Self::IS_DETERMINISTIC
    }

    fn call(
        &'a self,
        state: &ExpressionExecutionState<'c, '_>,
        _values: &[&Value],
    ) -> Result<ResolveResult<'c>, TransformError> {
        self.resolve(state)
    }
}

/// Additional trait for expressions, separate from Expression to make it easier to implement in macros
pub trait ExpressionMeta {
    fn num_children(&self) -> usize;

    fn get_child(&self, idx: usize) -> Option<&ExpressionType>;

    fn get_child_mut(&mut self, idx: usize) -> Option<&mut ExpressionType>;

    fn set_child(&mut self, idx: usize, item: ExpressionType);
}

/// A function expression, new functions must be added here.
#[derive(PassThrough, Debug, Clone)]
#[pass_through(fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result, "", Display)]
#[pass_through(fn resolve(&'a self, state: &ExpressionExecutionState<'c, '_>) -> Result<ResolveResult<'c>, TransformError>, "", Expression<'a, 'c>, where 'a: 'c)]
#[pass_through(fn call(&'a self, state: &ExpressionExecutionState<'c, '_>, _values: &[&Value]) -> Result<ResolveResult<'c>, TransformError>, "", Expression<'a, 'c>, where 'a: 'c)]
#[pass_through(fn get_is_deterministic(&self) -> bool, "", Expression<'a, 'c>, where 'a: 'c)]
#[pass_through(fn num_children(&self) -> usize, "", ExpressionMeta)]
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
    TryFloat(TryFloatFunction),
    TryInt(TryIntFunction),
    TryBool(TryBoolFunction),
    If(IfFunction),
    ToUnixTime(ToUnixTimeFunction),
    Case(CaseFunction),
    Pairs(PairsFunction),
    Map(MapFunction),
    FlatMap(FlatMapFunction),
    Reduce(ReduceFunction),
    Filter(FilterFunction),
    Zip(ZipFunction),
    Length(LengthFunction),
    Chunk(ChunkFunction),
    Now(NowFunction),
    Join(JoinFunction),
    Except(ExceptFunction),
    Select(SelectFunction),
    DistinctBy(DistinctByFunction),
}

/// Create a function expression from its name, or return a parser exception if it has the wrong number of arguments,
/// or does not exist.
pub fn get_function_expression(
    pos: Span,
    name: &str,
    args: Vec<ExpressionType>,
) -> Result<ExpressionType, BuildError> {
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
        "try_float" => FunctionType::TryFloat(TryFloatFunction::new(args, pos)?),
        "try_int" => FunctionType::TryInt(TryIntFunction::new(args, pos)?),
        "try_bool" => FunctionType::TryBool(TryBoolFunction::new(args, pos)?),
        "if" => FunctionType::If(IfFunction::new(args, pos)?),
        "to_unix_timestamp" => FunctionType::ToUnixTime(ToUnixTimeFunction::new(args, pos)?),
        "case" => FunctionType::Case(CaseFunction::new(args, pos)?),
        "pairs" => FunctionType::Pairs(PairsFunction::new(args, pos)?),
        "map" => FunctionType::Map(MapFunction::new(args, pos)?),
        "flatmap" => FunctionType::FlatMap(FlatMapFunction::new(args, pos)?),
        "reduce" => FunctionType::Reduce(ReduceFunction::new(args, pos)?),
        "filter" => FunctionType::Filter(FilterFunction::new(args, pos)?),
        "zip" => FunctionType::Zip(ZipFunction::new(args, pos)?),
        "length" => FunctionType::Length(LengthFunction::new(args, pos)?),
        "chunk" => FunctionType::Chunk(ChunkFunction::new(args, pos)?),
        "now" => FunctionType::Now(NowFunction::new(args, pos)?),
        "join" => FunctionType::Join(JoinFunction::new(args, pos)?),
        "except" => FunctionType::Except(ExceptFunction::new(args, pos)?),
        "select" => FunctionType::Select(SelectFunction::new(args, pos)?),
        "distinctBy" => FunctionType::DistinctBy(DistinctByFunction::new(args, pos)?),
        _ => return Err(BuildError::unrecognized_function(pos, name)),
    };
    Ok(ExpressionType::Function(expr))
}

/// An executable node in the expression tree.
/// This type can be executed with the `run` function, to yield a transformed Value.
#[derive(PassThrough, Debug, Clone)]
#[pass_through(fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result, "", Display)]
#[pass_through(fn resolve(&'a self, state: &ExpressionExecutionState<'c, '_>) -> Result<ResolveResult<'c>, TransformError>, "", Expression<'a, 'c>, where 'a: 'c)]
#[pass_through(fn get_is_deterministic(&self) -> bool, "", Expression<'a, 'c>, where 'a: 'c)]
#[pass_through(fn call(&'a self, state: &ExpressionExecutionState<'c, '_>, _values: &[&Value]) -> Result<ResolveResult<'c>, TransformError>, "", Expression<'a, 'c>, where 'a: 'c)]
#[pass_through(fn num_children(&self) -> usize, "", ExpressionMeta)]
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
    Lambda(LambdaExpression),
}

impl ExpressionType {
    /// Run the expression. Takes a list of values and a chunk_id, the id is just used for
    /// errors and logging.
    pub fn run<'a: 'c, 'c>(
        &'a self,
        data: impl IntoIterator<Item = &'c Value>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let data = data.into_iter().collect();
        let state = ExpressionExecutionState::new(&data);
        self.resolve(&state)
    }
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

impl<'a: 'c, 'c> Expression<'a, 'c> for Constant {
    fn resolve(
        &'a self,
        _state: &ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        Ok(ResolveResult::Borrowed(&self.val))
    }
}

impl ExpressionMeta for Constant {
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
    pub fn new(val: Value) -> Self {
        Self { val }
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
    v.as_u64()
        .map(JsonNumber::PosInteger)
        .or_else(|| v.as_i64().map(JsonNumber::NegInteger))
        .or_else(|| v.as_f64().map(JsonNumber::Float))
        .ok_or_else(|| {
            TransformError::new_conversion_failed(
                format!("Failed to convert input into number for operator {desc}"),
                span,
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
