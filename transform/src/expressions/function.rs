use serde_json::{Number, Value};

use crate::parse::ParserError;

use super::{
    base::{get_number_from_value, ExpressionType, ResolveResult},
    transform_error::TransformError,
    Expression,
};

use logos::Span;

/// Static information about a function type.
pub struct FunctionInfo {
    /// Minimum number of arguments
    pub minargs: usize,
    /// Maximum number of arguments, or None if the function can accept any number.
    pub maxargs: Option<usize>,
    /// Function name.
    pub name: &'static str,
}

impl FunctionInfo {
    pub fn validate(&self, num_args: usize) -> bool {
        if num_args < self.minargs {
            return false;
        }
        !matches!(self.maxargs, Some(x) if num_args > x)
    }

    pub fn num_args_desc(&self) -> String {
        match self.maxargs {
            Some(x) => {
                if x == self.minargs {
                    format!("function {} takes {} arguments", self.name, self.minargs)
                } else {
                    format!(
                        "function {} takes {} to {} arguments",
                        self.name, self.minargs, x
                    )
                }
            }
            None => format!(
                "function {} takes at least {} arguments",
                self.name, self.minargs
            ),
        }
    }
}

/// An expansion of Expression especially for functions, contains a `new` method, and `INFO`.
pub trait FunctionExpression<'a>: Expression<'a>
where
    Self: Sized,
    Self: 'a,
{
    type Iter<'b>: Iterator<Item = &'b ExpressionType>
    where
        Self: 'b;
    /// Static information about this function.
    const INFO: FunctionInfo;

    /// Create a new function from a list of expressions.
    fn new(args: Vec<ExpressionType>, span: Span) -> Result<Self, ParserError>;

    fn get_args(&'a self) -> Self::Iter<'a>;
}

macro_rules! function_def {
    // Base, should have defined the struct
    (_display $typ:ident) => {
        impl std::fmt::Display for $typ {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}(", Self::INFO.name)?;
                let mut is_first = true;
                for expr in self.get_args() {
                    if !is_first {
                        write!(f, ", ")?;
                    }
                    is_first = false;
                    write!(f, "{}", expr)?;
                }
                write!(f, ")")
            }
        }
    };
    ($typ:ident, $name:expr, $nargs:expr) => {
        #[derive(Debug)]
        pub struct $typ {
            args: [Box<ExpressionType>; $nargs],
            span: logos::Span,
        }

        function_def!(_display $typ);

        impl<'a> FunctionExpression<'a> for $typ {
            type Iter<'b> = std::iter::Map<std::slice::Iter<'b, Box<ExpressionType>>, fn(&'b Box<ExpressionType>) -> &'b ExpressionType>;// std::iter::Map<std::slice::Iter<'_, &ExpressionType>>;
            const INFO: FunctionInfo = FunctionInfo {
                minargs: $nargs,
                maxargs: Some($nargs),
                name: $name
            };

            fn new(args: Vec<ExpressionType>, span: logos::Span) -> Result<Self, crate::parse::ParserError> {
                if !Self::INFO.validate(args.len()) {
                    return Err(ParserError::n_function_args(
                        span,
                        &Self::INFO.num_args_desc(),
                    ));
                }
                Ok(Self {
                    span,
                    args: args.into_iter().map(|a| Box::new(a)).collect::<Vec<_>>().try_into().unwrap()
                })
            }

            fn get_args(&'a self) -> Self::Iter<'a> {
                self.args.iter().map(|a| &a)
            }
        }
    };
    ($typ:ident, $name:expr, $minargs:expr, $maxargs:expr) => {
        #[derive(Debug)]
        pub struct $typ {
            args: Vec<ExpressionType>,
            span: logos::Span
        }

        function_def!(_display $typ);

        impl<'a> FunctionExpression<'a> for $typ {
            const INFO: FunctionInfo = FunctionInfo {
                minargs: $minargs,
                maxargs: $maxargs,
                name: $name
            };

            fn new(args: Vec<ExpressionType>, span: logos::Span) -> Result<Self, crate::parse::ParserError> {
                if !Self::INFO.validate(args.len()) {
                    return Err(ParserError::n_function_args(
                        span,
                        &Self::INFO.num_args_desc(),
                    ));
                }
                Self {
                    span,
                    args,
                }
            }
        }
    }
}

/// Macro that creates a math function of the type `my_float.func(arg)`, which becomes `func(my_float, arg)`
/// in the expression language.
macro_rules! arg2_math_func {
    ($typ:ident, $name:expr, $rname:ident) => {
        function_def!($typ, $name, 2);

        impl<'a> Expression<'a> for $typ {
            fn resolve(
                &self,
                state: &super::base::ExpressionExecutionState,
            ) -> Result<ResolveResult<'a>, super::transform_error::TransformError> {
                let lhs = get_number_from_value(
                    &format!("{} argument 1", Self::INFO.name),
                    self.args[0].resolve(state)?.as_ref(),
                    &self.span,
                    state.id,
                )?
                .as_f64();
                let rhs = get_number_from_value(
                    &format!("{} argument 2", Self::INFO.name),
                    self.args[1].resolve(state)?.as_ref(),
                    &self.span,
                    state.id,
                )?
                .as_f64();

                let res = lhs.$rname(rhs);

                Ok(ResolveResult::Value(Value::Number(
                    Number::from_f64(res).ok_or_else(|| {
                        TransformError::new_conversion_failed(
                            format!(
                                "Failed to convert result of operator {} to number at {}",
                                $name, self.span.start
                            ),
                            &self.span,
                            state.id,
                        )
                    })?,
                )))
            }
        }
    };
}

/// Macro that creates a math function of the type `my_float.func()`, which becomes `func(my_float)`
/// in the expression language.
macro_rules! arg1_math_func {
    ($typ:ident, $name:expr, $rname:ident) => {
        function_def!($typ, $name, 1);

        impl<'a> Expression<'a> for $typ {
            fn resolve(
                &self,
                state: &super::base::ExpressionExecutionState,
            ) -> Result<ResolveResult<'a>, super::transform_error::TransformError> {
                let arg = get_number_from_value(
                    Self::INFO.name,
                    self.args[0].resolve(state)?.as_ref(),
                    &self.span,
                    state.id,
                )?
                .as_f64();

                let res = arg.$rname();

                Ok(ResolveResult::Value(Value::Number(
                    Number::from_f64(res).ok_or_else(|| {
                        TransformError::new_conversion_failed(
                            format!(
                                "Failed to convert result of operator {} to number at {}",
                                $name, self.span.start
                            ),
                            &self.span,
                            state.id,
                        )
                    })?,
                )))
            }
        }
    };
}

arg2_math_func!(PowFunction, "pow", powf);
arg2_math_func!(LogFunction, "log", log);
arg2_math_func!(Atan2Function, "atan2", atan2);
arg1_math_func!(FloorFunction, "floor", floor);
arg1_math_func!(CeilFunction, "ceil", ceil);
