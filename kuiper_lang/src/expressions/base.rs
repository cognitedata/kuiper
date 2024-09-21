use logos::Span;
use serde_json::Value;
use std::fmt::Display;

use crate::{compiler::BuildError, NULL_CONST};

use self::objects::ToObjectFunction;

use super::{
    functions::{
        distinct_by::DistinctByFunction, except::ExceptFunction, filter::FilterFunction,
        flatmap::FlatMapFunction, map::MapFunction, reduce::ReduceFunction, select::SelectFunction,
        zip::ZipFunction, *,
    },
    is_operator::IsExpression,
    lambda::LambdaExpression,
    operator::UnaryOpExpression,
    transform_error::TransformError,
    ArrayExpression, IfExpression, ObjectExpression, OpExpression, ResolveResult,
    SelectorExpression,
};

use kuiper_lang_macros::PassThrough;

#[cfg(feature = "completions")]
type Completions = std::collections::HashMap<Span, std::collections::HashSet<String>>;

/// State for expression execution. This struct is constructed for each expression.
/// Notably lifetime heavy. `'a` is the lifetime of the input data.
/// `'b` is the lifetime of the transform execution, so the temporary data in the transform.
pub struct ExpressionExecutionState<'data, 'exec> {
    data: &'exec Vec<Option<&'data Value>>,
    opcount: &'exec mut i64,
    max_opcount: i64,
    #[cfg(feature = "completions")]
    completions: Option<&'exec mut Completions>,
}

impl<'data, 'exec> ExpressionExecutionState<'data, 'exec> {
    /// Try to obtain a value with the given key from the state.
    #[inline]
    pub fn get_value(&self, key: usize) -> Option<&'data Value> {
        self.data.get(key).copied().and_then(|o| o)
    }

    pub fn new(
        data: &'exec Vec<Option<&'data Value>>,
        opcount: &'exec mut i64,
        max_opcount: i64,
    ) -> Self {
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
            data.push(Some(elem));
            pushed += 1;
        }
        if pushed < num_values {
            for _ in pushed..num_values {
                data.push(Some(&NULL_CONST));
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
    data: Vec<Option<&'data Value>>,
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
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_>;
}

/// A function expression, new functions must be added here.
#[derive(PassThrough, Debug, Clone)]
#[pass_through(fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result, "", Display)]
#[pass_through(fn resolve(&'a self, state: &mut ExpressionExecutionState<'c, '_>) -> Result<ResolveResult<'c>, TransformError>, "", Expression<'a, 'c>, where 'a: 'c)]
#[pass_through(fn call(&'a self, state: &mut ExpressionExecutionState<'c, '_>, _values: &[&Value]) -> Result<ResolveResult<'c>, TransformError>, "", Expression<'a, 'c>, where 'a: 'c)]
#[pass_through(fn get_is_deterministic(&self) -> bool, "", Expression<'a, 'c>, where 'a: 'c)]
#[pass_through(fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_>, "", ExpressionMeta)]
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
    Tail(TailFunction),
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
    Substring(SubstringFunction),
    Replace(ReplaceFunction),
    Split(SplitFunction),
    TrimWhitespace(TrimWhitespace),
    Slice(SliceFunction),
    Chars(CharsFunction),
    ToObject(ToObjectFunction),
    Sum(SumFunction),
    Any(AnyFunction),
    All(AllFunction),
    Contains(ContainsFunction),
    StringJoin(StringJoinFunction),
    Min(MinFunction),
    Max(MaxFunction),
    Digest(DigestFunction),
    Coalesce(CoalesceFunction),
}

struct FunctionBuilder {
    args: Vec<ExpressionType>,
    pos: Span,
}

impl FunctionBuilder {
    fn mk<T: FunctionExpression>(self) -> Result<T, BuildError> {
        T::new(self.args, self.pos)
    }
}

/// Create a function expression from its name, or return a parser exception if it has the wrong number of arguments,
/// or does not exist.
pub fn get_function_expression(
    pos: Span,
    name: &str,
    args: Vec<ExpressionType>,
) -> Result<ExpressionType, BuildError> {
    let b = FunctionBuilder { pos, args };

    let expr = match name {
        "pow" => FunctionType::Pow(b.mk()?),
        "log" => FunctionType::Log(b.mk()?),
        "atan2" => FunctionType::Atan2(b.mk()?),
        "floor" => FunctionType::Floor(b.mk()?),
        "ceil" => FunctionType::Ceil(b.mk()?),
        "round" => FunctionType::Round(b.mk()?),
        "concat" => FunctionType::Concat(b.mk()?),
        "string" => FunctionType::String(b.mk()?),
        "int" => FunctionType::Int(b.mk()?),
        "float" => FunctionType::Float(b.mk()?),
        "try_float" => FunctionType::TryFloat(b.mk()?),
        "try_int" => FunctionType::TryInt(b.mk()?),
        "try_bool" => FunctionType::TryBool(b.mk()?),
        "if" => FunctionType::If(b.mk()?),
        "to_unix_timestamp" => FunctionType::ToUnixTime(b.mk()?),
        "format_timestamp" => FunctionType::FormatTimestamp(b.mk()?),
        "case" => FunctionType::Case(b.mk()?),
        "pairs" => FunctionType::Pairs(b.mk()?),
        "map" => FunctionType::Map(b.mk()?),
        "flatmap" => FunctionType::FlatMap(b.mk()?),
        "reduce" => FunctionType::Reduce(b.mk()?),
        "filter" => FunctionType::Filter(b.mk()?),
        "zip" => FunctionType::Zip(b.mk()?),
        "length" => FunctionType::Length(b.mk()?),
        "chunk" => FunctionType::Chunk(b.mk()?),
        "now" => FunctionType::Now(b.mk()?),
        "join" => FunctionType::Join(b.mk()?),
        "except" => FunctionType::Except(b.mk()?),
        "select" => FunctionType::Select(b.mk()?),
        "distinct_by" => FunctionType::DistinctBy(b.mk()?),
        "substring" => FunctionType::Substring(b.mk()?),
        "replace" => FunctionType::Replace(b.mk()?),
        "split" => FunctionType::Split(b.mk()?),
        "trim_whitespace" => FunctionType::TrimWhitespace(b.mk()?),
        "slice" => FunctionType::Slice(b.mk()?),
        "chars" => FunctionType::Chars(b.mk()?),
        "tail" => FunctionType::Tail(b.mk()?),
        "to_object" => FunctionType::ToObject(b.mk()?),
        "sum" => FunctionType::Sum(b.mk()?),
        "any" => FunctionType::Any(b.mk()?),
        "all" => FunctionType::All(b.mk()?),
        "contains" => FunctionType::Contains(b.mk()?),
        "string_join" => FunctionType::StringJoin(b.mk()?),
        "min" => FunctionType::Min(b.mk()?),
        "max" => FunctionType::Max(b.mk()?),
        "digest" => FunctionType::Digest(b.mk()?),
        "coalesce" => FunctionType::Coalesce(b.mk()?),
        _ => return Err(BuildError::unrecognized_function(b.pos, name)),
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
#[pass_through(fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_>, "", ExpressionMeta)]
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
    If(IfExpression),
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
    ///   the program. If set to -1, no limit is enforced.
    pub fn run_limited<'a: 'c, 'c>(
        &'a self,
        data: impl IntoIterator<Item = &'c Value>,
        max_operation_count: i64,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let mut opcount = 0;
        let data = data.into_iter().map(Some).collect();
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

        let data = data.into_iter().map(Some).collect();
        let mut opcount = 0;
        let mut state = ExpressionExecutionState::new(&data, &mut opcount, -1);
        let mut completions = HashMap::new();
        state.completions = Some(&mut completions);
        let r = self.resolve(&mut state)?;
        Ok((r, completions))
    }

    pub(crate) fn fail_if_lambda(&self) -> Result<(), BuildError> {
        if let ExpressionType::Lambda(lambda) = self {
            Err(BuildError::unexpected_lambda(&lambda.span))
        } else {
            Ok(())
        }
    }
}

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
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new([].into_iter())
    }
}

impl Constant {
    pub fn new(val: Value) -> Self {
        Self { val }
    }
}
