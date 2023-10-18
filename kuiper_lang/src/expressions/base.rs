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
    is_operator::IsExpression,
    lambda::LambdaExpression,
    numbers::JsonNumber,
    operator::UnaryOpExpression,
    transform_error::TransformError,
    ArrayExpression, ObjectExpression, OpExpression, SelectorExpression,
};

use kuiper_lang_macros::PassThrough;

#[cfg(feature = "completions")]
type Completions = std::collections::HashMap<Span, std::collections::HashSet<String>>;

/// State for expression execution. This struct is constructed for each expression.
/// Notably lifetime heavy. `'a` is the lifetime of the input data.
/// `'b` is the lifetime of the transform execution, so the temporary data in the transform.
pub struct ExpressionExecutionState<'data, 'exec> {
    data: &'exec Vec<&'data Value>,
    opcount: &'exec mut i64,
    max_opcount: i64,
    #[cfg(feature = "completions")]
    completions: Option<&'exec mut Completions>,
}

impl<'data, 'exec> ExpressionExecutionState<'data, 'exec> {
    /// Try to obtain a value with the given key from the state.
    #[inline]
    pub fn get_value(&self, key: usize) -> Option<&'data Value> {
        self.data.get(key).copied()
    }

    pub fn new(data: &'exec Vec<&'data Value>, opcount: &'exec mut i64, max_opcount: i64) -> Self {
        Self {
            data,
            opcount,
            max_opcount,
            #[cfg(feature = "completions")]
            completions: Default::default(),
        }
    }

    pub fn get_temporary_clone<'inner>(
        &'inner mut self,
        extra_values: impl Iterator<Item = &'inner Value>,
        num_values: usize,
    ) -> InternalExpressionExecutionState<'inner, 'inner>
    where
        'data: 'inner,
    {
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
            opcount: self.opcount,
            max_opcount: self.max_opcount,
            #[cfg(feature = "completions")]
            completions: self.completions.as_deref_mut(),
        }
    }

    pub fn inc_op(&mut self) -> Result<(), TransformError> {
        *self.opcount += 1;
        if *self.opcount > self.max_opcount && self.max_opcount > 0 {
            Err(TransformError::OperationLimitExceeded)
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "completions")]
    pub fn add_completion_entries(
        &mut self,
        it: impl Iterator<Item = impl Into<String>>,
        span: Span,
    ) {
        if let Some(c) = &mut self.completions {
            c.entry(span).or_default().extend(it.map(|i| i.into()));
        }
    }
}

#[derive(Debug)]
pub struct InternalExpressionExecutionState<'data, 'exec> {
    data: Vec<&'data Value>,
    opcount: &'exec mut i64,
    max_opcount: i64,
    #[cfg(feature = "completions")]
    completions: Option<&'exec mut Completions>,
}

impl<'data, 'exec> InternalExpressionExecutionState<'data, 'exec> {
    pub fn get_temp_state<'slf>(&'slf mut self) -> ExpressionExecutionState<'data, 'slf> {
        ExpressionExecutionState {
            data: &self.data,
            opcount: self.opcount,
            max_opcount: self.max_opcount,
            #[cfg(feature = "completions")]
            completions: self.completions.as_deref_mut(),
        }
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
///
/// 'a is the lifetime of the transform itself
///
/// 'b is the lifetime of the current execution of the transform.
///
/// 'c is the lifetime of the execution of the program itself, so it goes beyond this transform.
///
/// In simple terms
///```ignore
///     'a
///
///     start program execution
///
///         'c
///
///         for transform in program
///
///             for entry in inputs
///
///                 'b
/// ````
pub trait Expression<'a: 'c, 'c>: Display {
    const IS_DETERMINISTIC: bool = true;
    /// Resolve an expression.
    fn resolve(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError>;

    fn get_is_deterministic(&self) -> bool {
        Self::IS_DETERMINISTIC
    }

    fn call(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
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
#[pass_through(fn resolve(&'a self, state: &mut ExpressionExecutionState<'c, '_>) -> Result<ResolveResult<'c>, TransformError>, "", Expression<'a, 'c>, where 'a: 'c)]
#[pass_through(fn call(&'a self, state: &mut ExpressionExecutionState<'c, '_>, _values: &[&Value]) -> Result<ResolveResult<'c>, TransformError>, "", Expression<'a, 'c>, where 'a: 'c)]
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
    FormatTimestamp(FormatTimestampFunction),
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
        "format_timestamp" => {
            FunctionType::FormatTimestamp(FormatTimestampFunction::new(args, pos)?)
        }
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
        "distinct_by" => FunctionType::DistinctBy(DistinctByFunction::new(args, pos)?),
        _ => return Err(BuildError::unrecognized_function(pos, name)),
    };
    Ok(ExpressionType::Function(expr))
}

/// An executable node in the expression tree.
/// This type can be executed with the `run` function, to yield a transformed Value.
#[derive(PassThrough, Debug, Clone)]
#[pass_through(fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result, "", Display)]
#[pass_through(fn resolve(&'a self, state: &mut ExpressionExecutionState<'c, '_>) -> Result<ResolveResult<'c>, TransformError>, "", Expression<'a, 'c>, where 'a: 'c)]
#[pass_through(fn get_is_deterministic(&self) -> bool, "", Expression<'a, 'c>, where 'a: 'c)]
#[pass_through(fn call(&'a self, state: &mut ExpressionExecutionState<'c, '_>, _values: &[&Value]) -> Result<ResolveResult<'c>, TransformError>, "", Expression<'a, 'c>, where 'a: 'c)]
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
    Is(IsExpression),
}

impl ExpressionType {
    /// Run the expression. Takes a list of values and a chunk_id, the id is just used for
    /// errors and logging.
    ///
    /// * `data` - An iterator over the inputs to the expression. The count must match the count provided when the expression was compiled
    pub fn run<'a: 'c, 'c>(
        &'a self,
        data: impl IntoIterator<Item = &'c Value>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        self.run_limited(data, -1)
    }

    /// Run the expression. Takes a list of values and a chunk_id, the id is just used for
    /// errors and logging.
    ///
    /// * `data` - An iterator over the inputs to the expression. The count must match the count provided when the expression was compiled
    /// * `max_operation_count` - The maximum number of operations performed by the program. This is a rough estimate of the complexity of
    /// the program. If set to -1, no limit is enforced.
    pub fn run_limited<'a: 'c, 'c>(
        &'a self,
        data: impl IntoIterator<Item = &'c Value>,
        max_operation_count: i64,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let mut opcount = 0;
        let data = data.into_iter().collect();
        let mut state = ExpressionExecutionState::new(&data, &mut opcount, max_operation_count);
        self.resolve(&mut state)
    }

    #[cfg(feature = "completions")]
    /// Run the expression, and return the result along with a map from range in the input
    /// to possible completions in that range. These are only collected from selectors.
    pub fn run_get_completions<'a: 'c, 'c>(
        &'a self,
        data: impl IntoIterator<Item = &'c Value>,
    ) -> Result<(ResolveResult<'c>, Completions), TransformError> {
        use std::collections::HashMap;

        let data = data.into_iter().collect();
        let mut opcount = 0;
        let mut state = ExpressionExecutionState::new(&data, &mut opcount, -1);
        let mut completions = HashMap::new();
        state.completions = Some(&mut completions);
        let r = self.resolve(&mut state)?;
        Ok((r, completions))
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
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        state.inc_op()?;
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
    Ok(v.into())
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
) -> Result<Cow<'a, str>, TransformError> {
    match val {
        Value::Null => Ok(Cow::Borrowed("")),
        Value::Bool(n) => Ok(Cow::Borrowed(match n {
            true => "true",
            false => "false",
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

pub(crate) fn get_string_from_value_owned<'a>(
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
        _ => {
            return Err(TransformError::new_incorrect_type(
                desc,
                "string or number",
                TransformError::value_desc(&val),
                span,
            ))
        }
    }
}

pub(crate) fn map_cow_clone_string<'a, 'b, 'c, T>(
    value: ResolveResult<'_>,
    state: &'a mut ExpressionExecutionState<'b, 'c>,
    string: impl FnOnce(String, &'a mut ExpressionExecutionState<'b, 'c>) -> T,
    other: impl FnOnce(&Value, &'a mut ExpressionExecutionState<'b, 'c>) -> T,
) -> T {
    match value {
        Cow::Owned(Value::String(s)) => string(s, state),
        Cow::Borrowed(Value::String(s)) => string(s.to_string(), state),
        c => other(c.as_ref(), state),
    }
}

// This isn't using map_cow_clone_string because it is so critical,
// and it seems that the complexity of this prevents some optimizations which eliminates
// allocations in object functions and elsewhere.
pub(crate) fn get_string_from_cow_value<'a>(
    desc: &str,
    val: ResolveResult<'a>,
    span: &Span,
) -> Result<Cow<'a, str>, TransformError> {
    match val {
        Cow::Borrowed(v) => get_string_from_value(desc, v, span),
        Cow::Owned(v) => get_string_from_value_owned(desc, v, span),
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
