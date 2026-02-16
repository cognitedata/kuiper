#[macro_use]
mod macros;
mod arrays;
mod coalesce;
mod conversions;
mod digest;
mod dynamic;
mod functors;
mod join;
mod json;
mod logic;
mod math;
mod regex;
mod string;
mod time;
mod transforms;

use crate::compiler::BuildError;
pub use arrays::*;
pub use coalesce::*;
pub use conversions::*;
pub use digest::*;
pub use functors::*;
pub use join::*;
pub use json::*;
pub use logic::*;
pub use math::*;
pub use regex::*;
pub use string::*;
pub use time::*;
pub use transforms::*;

pub use dynamic::{make_function, DynamicFunction, DynamicFunctionBuilder, DynamicFunctionSource};
pub use macros::function_def;

pub(crate) use dynamic::EmptyFunctionSource;

use super::{base::ExpressionType, LambdaExpression};

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
    /// Validate that the number of arguments passed to this function is valid.
    pub fn validate(&self, num_args: usize) -> bool {
        if num_args < self.minargs {
            return false;
        }
        !matches!(self.maxargs, Some(x) if num_args > x)
    }

    /// Get a human-readable description of the number of arguments this function takes, for use in error messages.
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
pub trait FunctionExpression
where
    Self: Sized,
{
    /// Static information about this function.
    const INFO: FunctionInfo;

    /// Create a new function from a list of expressions.
    fn new(args: Vec<ExpressionType>, span: Span) -> Result<Self, BuildError>;
}

/// A trait for functions that can accept lambdas as arguments.
/// The default implementation rejects all lambdas.
pub trait LambdaAcceptFunction {
    /// Validate that a lambda passed as an argument is valid for this function. `idx` is the index of the argument the lambda was passed as, and `num_args` is the number of arguments
    /// in the lambda itself.
    fn validate_lambda(
        _idx: usize,
        lambda: &LambdaExpression,
        _num_args: usize,
    ) -> Result<(), BuildError> {
        Err(BuildError::unexpected_lambda(&lambda.span))
    }
}
