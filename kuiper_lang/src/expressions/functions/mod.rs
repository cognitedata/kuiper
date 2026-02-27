#[macro_use]
mod macros;
mod arrays;
mod coalesce;
mod conversions;
mod digest;
pub(super) mod dynamic;
mod functors;
mod join;
mod json;
mod logic;
mod math;
#[cfg(feature = "std")]
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
pub use macros::function_def;
pub use math::*;
#[cfg(feature = "std")]
pub use regex::*;
pub use string::*;
pub use time::*;
pub use transforms::*;

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
    /// Validate the number of arguments for this function.
    pub fn validate(&self, num_args: usize) -> bool {
        if num_args < self.minargs {
            return false;
        }
        !matches!(self.maxargs, Some(x) if num_args > x)
    }

    /// Get a human-readable description of the number of arguments this function takes, for error messages.
    pub fn num_args_desc(&self) -> crate::String {
        match self.maxargs {
            Some(x) => {
                if x == self.minargs {
                    alloc::format!("function {} takes {} arguments", self.name, self.minargs)
                } else {
                    alloc::format!(
                        "function {} takes {} to {} arguments",
                        self.name,
                        self.minargs,
                        x
                    )
                }
            }
            None => alloc::format!(
                "function {} takes at least {} arguments",
                self.name,
                self.minargs
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
    fn new(args: crate::Vec<ExpressionType>, span: Span) -> Result<Self, BuildError>;
}

/// Trait for validating lambdas passed to functions.
/// This is used for functions that accept lambdas as arguments,
/// to validate the number of arguments in the lambda and which arguments are lambdas.
pub trait LambdaAcceptFunction {
    /// Validate that the argument at the given index is allowed to
    /// be a lambda, and that the lambda itself is valid.
    /// This also includes the number of arguments in the function itself,
    /// if that is relevant.
    fn validate_lambda(
        _idx: usize,
        lambda: &LambdaExpression,
        _num_args: usize,
    ) -> Result<(), BuildError> {
        Err(BuildError::unexpected_lambda(&lambda.span))
    }
}
