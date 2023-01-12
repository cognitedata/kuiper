#[macro_use]
mod macros;
mod logic;
mod math;
mod string;
mod time;
mod transforms;

use crate::parse::ParserError;
pub use logic::*;
pub use math::*;
pub use string::*;
pub use time::*;
pub use transforms::*;

use super::{base::ExpressionType, Expression};

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
    /// Static information about this function.
    const INFO: FunctionInfo;

    /// Create a new function from a list of expressions.
    fn new(args: Vec<ExpressionType>, span: Span) -> Result<Self, ParserError>;
}
