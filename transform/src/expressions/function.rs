use std::fmt::Display;

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
{
    /// Static information about this function.
    const INFO: FunctionInfo;

    /// Create a new function from a list of expressions.
    fn new(args: Vec<ExpressionType>, span: Span) -> Result<Self, ParserError>;
}

/// Macro that creates a math function of the type `my_float.func(arg)`, which becomes `func(my_float, arg)`
/// in the expression language.
macro_rules! arg2_math_func {
    ($typ:ident, $name:expr, $rname:ident) => {
        pub struct $typ {
            lhs: Box<ExpressionType>,
            rhs: Box<ExpressionType>,
            span: Span,
        }

        impl Display for $typ {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({}, {})", $name, self.lhs, self.rhs)
            }
        }

        impl<'a> Expression<'a> for $typ {
            fn resolve(
                &self,
                state: &super::base::ExpressionExecutionState,
            ) -> Result<ResolveResult<'a>, super::transform_error::TransformError> {
                let lhs = get_number_from_value(
                    Self::INFO.name,
                    self.lhs.resolve(state)?.as_ref(),
                    &self.span,
                    state.id,
                )?
                .as_f64();
                let rhs = get_number_from_value(
                    Self::INFO.name,
                    self.rhs.resolve(state)?.as_ref(),
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

        impl<'a> FunctionExpression<'a> for $typ {
            const INFO: FunctionInfo = FunctionInfo {
                minargs: 2,
                maxargs: Some(2),
                name: $name,
            };

            fn new(args: Vec<ExpressionType>, span: Span) -> Result<Self, ParserError> {
                if !Self::INFO.validate(args.len()) {
                    return Err(ParserError::n_function_args(
                        span,
                        &Self::INFO.num_args_desc(),
                    ));
                }
                let mut iter = args.into_iter();
                Ok(Self {
                    lhs: Box::new(iter.next().unwrap()),
                    rhs: Box::new(iter.next().unwrap()),
                    span,
                })
            }
        }
    };
}

/// Macro that creates a math function of the type `my_float.func()`, which becomes `func(my_float)`
/// in the expression language.
macro_rules! arg1_math_func {
    ($typ:ident, $name:expr, $rname:ident) => {
        pub struct $typ {
            arg: Box<ExpressionType>,
            span: Span,
        }

        impl Display for $typ {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", $name, self.arg)
            }
        }

        impl<'a> Expression<'a> for $typ {
            fn resolve(
                &self,
                state: &super::base::ExpressionExecutionState,
            ) -> Result<ResolveResult<'a>, super::transform_error::TransformError> {
                let arg = get_number_from_value(
                    Self::INFO.name,
                    self.arg.resolve(state)?.as_ref(),
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

        impl<'a> FunctionExpression<'a> for $typ {
            const INFO: FunctionInfo = FunctionInfo {
                minargs: 1,
                maxargs: Some(1),
                name: $name,
            };

            fn new(args: Vec<ExpressionType>, span: Span) -> Result<Self, ParserError> {
                if !Self::INFO.validate(args.len()) {
                    return Err(ParserError::n_function_args(
                        span,
                        &Self::INFO.num_args_desc(),
                    ));
                }
                let mut iter = args.into_iter();
                Ok(Self {
                    arg: Box::new(iter.next().unwrap()),
                    span,
                })
            }
        }
    };
}

arg2_math_func!(PowFunction, "pow", powf);
arg2_math_func!(LogFunction, "log", log);
arg2_math_func!(Atan2Function, "atan2", atan2);
arg1_math_func!(FloorFunction, "floor", floor);
arg1_math_func!(CeilFunction, "ceil", ceil);
