use if_value::IfValueFunction;
use logos::Span;
use serde_json::Value;
use std::fmt::Display;

use crate::{
    compiler::BuildError,
    expressions::{run_builder::ExpressionRunBuilder, source::SourceData},
    types::{Type, TypeError, TypeExecutionState},
    NULL_CONST,
};

use self::objects::ToObjectFunction;

use super::{
    functions::{
        distinct_by::DistinctByFunction, except::ExceptFunction, filter::FilterFunction,
        flatmap::FlatMapFunction, map::MapFunction, reduce::ReduceFunction, select::SelectFunction,
        zip::ZipFunction, *,
    },
    is_operator::IsExpression,
    lambda::LambdaExpression,
    macro_call::MacroCallExpression,
    operator::UnaryOpExpression,
    transform_error::TransformError,
    ArrayExpression, IfExpression, ObjectExpression, OpExpression, ResolveResult,
    SelectorExpression,
};

use kuiper_lang_macros::PassThrough;

/// Type for storing completions collected during expression execution.
#[cfg(feature = "completions")]
pub type Completions = std::collections::HashMap<Span, std::collections::HashSet<String>>;

/// State for expression execution. This struct is constructed for each expression.
/// Notably lifetime heavy. `'a` is the lifetime of the input data.
/// `'b` is the lifetime of the transform execution, so the temporary data in the transform.
pub struct ExpressionExecutionState<'data, 'exec> {
    data: &'exec Vec<Option<&'data dyn SourceData>>,
    opcount: &'exec mut i64,
    max_opcount: i64,
    #[cfg(feature = "completions")]
    completions: Option<&'exec mut Completions>,
}

impl<'data, 'exec> ExpressionExecutionState<'data, 'exec> {
    /// Try to obtain a value with the given key from the state.
    #[inline]
    pub fn get_value(&self, key: usize) -> Option<&'data dyn SourceData> {
        self.data.get(key).copied().and_then(|o| o)
    }

    #[cfg(feature = "completions")]
    pub(crate) fn set_completions(&mut self, completions: &'exec mut Completions) {
        self.completions = Some(completions);
    }

    pub(crate) fn new(
        data: &'exec Vec<Option<&'data dyn SourceData>>,
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

    pub(crate) fn get_temporary_clone<'inner>(
        &'inner mut self,
        extra_values: impl Iterator<Item = &'inner dyn SourceData>,
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

    /// Increment the operation count, and check if it exceeds the maximum.
    /// If it does, return an error.
    pub fn inc_op(&mut self) -> Result<(), TransformError> {
        *self.opcount += 1;
        if *self.opcount > self.max_opcount && self.max_opcount > 0 {
            Err(TransformError::OperationLimitExceeded)
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "completions")]
    /// Add completion entries for the given span.
    pub fn add_completion_entries<I: Iterator<Item = impl Into<String>>, F: Fn() -> I>(
        &mut self,
        it: F,
        span: Span,
    ) {
        if let Some(c) = &mut self.completions {
            c.entry(span).or_default().extend(it().map(|i| i.into()));
        }
    }
}

#[derive(Debug)]
pub struct InternalExpressionExecutionState<'data, 'exec> {
    data: Vec<Option<&'data dyn SourceData>>,
    opcount: &'exec mut i64,
    max_opcount: i64,
    #[cfg(feature = "completions")]
    completions: Option<&'exec mut Completions>,
}

impl<'data> InternalExpressionExecutionState<'data, '_> {
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
pub trait Expression: Display {
    /// Check if the expression is deterministic.
    fn is_deterministic(&self) -> bool {
        true
    }

    /// Resolve an expression.
    fn resolve<'a: 'c, 'c>(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError>;

    fn call<'a: 'c, 'c>(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
        _values: &[&Value],
    ) -> Result<ResolveResult<'c>, TransformError> {
        self.resolve(state)
    }

    fn resolve_types(
        &self,
        _state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        Ok(Type::Any)
    }

    fn call_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
        _arguments: &[&Type],
    ) -> Result<Type, crate::types::TypeError> {
        self.resolve_types(state)
    }
}

/// Additional trait for expressions, separate from Expression to make it easier to implement in macros
pub trait ExpressionMeta {
    /// Get mutable references to the child expressions of this expression, for use in expression rewriting.
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_>;
}

/// A function expression, new functions must be added here.
#[derive(PassThrough, Debug)]
#[pass_through(fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result, "", Display)]
#[pass_through(fn resolve<'a: 'c, 'c>(&'a self, state: &mut ExpressionExecutionState<'c, '_>) -> Result<ResolveResult<'c>, TransformError>, "", Expression)]
#[pass_through(fn call<'a: 'c, 'c>(&'a self, state: &mut ExpressionExecutionState<'c, '_>, _values: &[&Value]) -> Result<ResolveResult<'c>, TransformError>, "", Expression)]
#[pass_through(fn is_deterministic(&self) -> bool, "", Expression)]
#[pass_through(fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_>, "", ExpressionMeta)]
#[pass_through(fn resolve_types(&self, state: &mut crate::types::TypeExecutionState<'_, '_>) -> Result<Type, crate::types::TypeError>, "", Expression)]
#[pass_through(fn call_types(&self, state: &mut crate::types::TypeExecutionState<'_, '_>, _arguments: &[&Type]) -> Result<Type, crate::types::TypeError>, "", Expression)]
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
    RegexIsMatch(RegexIsMatchFunction),
    RegexFirstMatch(RegexFirstMatchFunction),
    RegexAllMatches(RegexAllMatchesFunction),
    RegexFirstCaptures(RegexFirstCapturesFunction),
    RegexAllCaptures(RegexAllCapturesFunction),
    RegexReplace(RegexReplaceFunction),
    RegexReplaceAll(RegexReplaceAllFunction),
    StartsWith(StartsWithFunction),
    EndsWith(EndsWithFunction),
    IfValue(IfValueFunction),
    ParseJson(ParseJsonFunction),
    Lower(LowerFunction),
    Upper(UpperFunction),
    Translate(TranslateFunction),
    SqrtFunction(SqrtFunction),
    ExpFunction(ExpFunction),
    SinFunction(SinFunction),
    CosFunction(CosFunction),
    TanFunction(TanFunction),
    AsinFunction(AsinFunction),
    AcosFunction(AcosFunction),
    AtanFunction(AtanFunction),
    CustomFunction(Box<dyn DynamicFunction>),
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
        "regex_is_match" => FunctionType::RegexIsMatch(b.mk()?),
        "regex_first_match" => FunctionType::RegexFirstMatch(b.mk()?),
        "regex_all_matches" => FunctionType::RegexAllMatches(b.mk()?),
        "regex_first_captures" => FunctionType::RegexFirstCaptures(b.mk()?),
        "regex_all_captures" => FunctionType::RegexAllCaptures(b.mk()?),
        "regex_replace" => FunctionType::RegexReplace(b.mk()?),
        "regex_replace_all" => FunctionType::RegexReplaceAll(b.mk()?),
        "starts_with" => FunctionType::StartsWith(b.mk()?),
        "ends_with" => FunctionType::EndsWith(b.mk()?),
        "if_value" => FunctionType::IfValue(b.mk()?),
        "parse_json" => FunctionType::ParseJson(b.mk()?),
        "lower" => FunctionType::Lower(b.mk()?),
        "upper" => FunctionType::Upper(b.mk()?),
        "translate" => FunctionType::Translate(b.mk()?),
        "sqrt" => FunctionType::SqrtFunction(b.mk()?),
        "exp" => FunctionType::ExpFunction(b.mk()?),
        "sin" => FunctionType::SinFunction(b.mk()?),
        "cos" => FunctionType::CosFunction(b.mk()?),
        "tan" => FunctionType::TanFunction(b.mk()?),
        "asin" => FunctionType::AsinFunction(b.mk()?),
        "acos" => FunctionType::AcosFunction(b.mk()?),
        "atan" => FunctionType::AtanFunction(b.mk()?),
        _ => return Err(BuildError::unrecognized_function(b.pos, name)),
    };
    Ok(ExpressionType::Function(expr))
}

/// An executable node in the expression tree.
/// This type can be executed with the `run` function, to yield a transformed Value.
#[derive(PassThrough, Debug)]
#[pass_through(fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result, "", Display)]
#[pass_through(fn resolve<'a: 'c, 'c>(&'a self, state: &mut ExpressionExecutionState<'c, '_>) -> Result<ResolveResult<'c>, TransformError>, "", Expression)]
#[pass_through(fn is_deterministic(&self) -> bool, "", Expression)]
#[pass_through(fn call<'a: 'c, 'c>(&'a self, state: &mut ExpressionExecutionState<'c, '_>, _values: &[&Value]) -> Result<ResolveResult<'c>, TransformError>, "", Expression)]
#[pass_through(fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_>, "", ExpressionMeta)]
#[pass_through(fn resolve_types(&self, state: &mut crate::types::TypeExecutionState<'_, '_>) -> Result<Type, crate::types::TypeError>, "", Expression)]
#[pass_through(fn call_types(&self, state: &mut crate::types::TypeExecutionState<'_, '_>, _arguments: &[&Type]) -> Result<Type, crate::types::TypeError>, "", Expression)]
pub enum ExpressionType {
    /// A constant value expression.
    Constant(Constant),
    /// A binary operator expression.
    Operator(OpExpression),
    /// A unary operator expression.
    UnaryOperator(UnaryOpExpression),
    /// A selector expression, i.e. accessing fields in objects or arrays.
    Selector(SelectorExpression),
    /// A function call expression.
    Function(FunctionType),
    /// An array expression.
    Array(ArrayExpression),
    /// An object expression.
    Object(ObjectExpression),
    /// A lambda expression.
    Lambda(LambdaExpression),
    /// An "is" type check expression.
    Is(IsExpression),
    /// An "if" conditional expression.
    If(IfExpression),
    /// A macro call expression.
    MacroCallExpression(MacroCallExpression),
}

impl ExpressionType {
    /// Run the expression. Takes a list of values.
    ///
    /// * `data` - An iterator over the inputs to the expression. The count must match the count provided when the expression was compiled
    pub fn run<'a: 'c, 'c>(
        &'a self,
        data: impl IntoIterator<Item = &'c Value>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        self.run_limited(data, -1)
    }

    /// Get a builder for running the expression.
    pub fn builder(&self) -> ExpressionRunBuilder<'_, '_, ()> {
        ExpressionRunBuilder::<'_, '_, ()>::new(self)
    }

    /// Run the expression. Takes a list of values.
    ///
    /// * `data` - An iterator over the inputs to the expression. The count must match the count provided when the expression was compiled
    /// * `max_operation_count` - The maximum number of operations performed by the program. This is a rough estimate of the complexity of
    ///   the program. If set to -1, no limit is enforced.
    pub fn run_limited<'a: 'c, 'c>(
        &'a self,
        data: impl IntoIterator<Item = &'c Value>,
        max_operation_count: i64,
    ) -> Result<ResolveResult<'c>, TransformError> {
        self.builder()
            .with_values(data)
            .max_operation_count(max_operation_count)
            .run()
    }

    /// Run the expression. Takes a list of values. Returns the result along with the number of operations performed.
    ///
    /// * `max_operation_count` - The maximum number of operations performed by the program. This is a rough estimate of the complexity of
    ///   the program. If set to -1, no limit is enforced.
    pub fn run_get_opcount<'a: 'c, 'c>(
        &'a self,
        data: impl IntoIterator<Item = &'c Value>,
    ) -> Result<(ResolveResult<'c>, i64), TransformError> {
        self.builder().with_values(data).run_get_opcount()
    }

    #[cfg(feature = "completions")]
    /// Run the expression, and return the result along with a map from range in the input
    /// to possible completions in that range. These are only collected from selectors.
    pub fn run_get_completions<'a: 'c, 'c>(
        &'a self,
        data: impl IntoIterator<Item = &'c Value>,
    ) -> Result<(ResolveResult<'c>, Completions), TransformError> {
        self.builder().with_values(data).run_get_completions()
    }

    /// Run the expression with a list of custom input data.
    pub fn run_custom_input<'a: 'c, 'c>(
        &'a self,
        data: impl IntoIterator<Item = &'c dyn SourceData>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        self.builder().with_custom_items(data).run()
    }

    /// Run the expression in type space with a list of types.
    pub fn run_types<'a: 'c, 'c>(
        &'a self,
        data: impl IntoIterator<Item = Type>,
    ) -> Result<Type, TypeError> {
        let data_owned = data.into_iter().collect::<Vec<_>>();
        let data = data_owned.iter().collect::<Vec<_>>();
        let mut state = TypeExecutionState::new(&data);
        self.resolve_types(&mut state)
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

impl Expression for Constant {
    fn resolve<'a: 'c, 'c>(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        state.inc_op()?;
        Ok(ResolveResult::Borrowed(&self.val))
    }

    fn resolve_types(
        &self,
        _state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        Ok(Type::from_const(self.value().clone()))
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

    pub(crate) fn value(&self) -> &Value {
        &self.val
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    fn test_constant_type_resolution() {
        let expr = compile_expression("15", &[]).unwrap();
        let r = expr.run_types([]).unwrap();
        assert_eq!(r, crate::types::Type::from_const(15));
    }

    #[test]
    fn test_optimized_type_resolution() {
        let expr = compile_expression(r#""test".concat("hello")"#, &[]).unwrap();
        let r = expr.run_types([]).unwrap();
        assert_eq!(r, crate::types::Type::from_const("testhello".to_string()));
    }
}
