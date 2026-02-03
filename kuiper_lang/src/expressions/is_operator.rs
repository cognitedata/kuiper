use std::fmt::Display;

use serde_json::Value;

use crate::{BuildError, ExpressionType, TransformError};

use super::{Expression, ExpressionExecutionState, ExpressionMeta, ResolveResult};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// A type literal used in "is" expressions.
pub enum TypeLiteral {
    /// The null type.
    Null,
    /// An integer.
    Int,
    /// A boolean.
    Bool,
    /// A floating point number.
    Float,
    /// A string.
    String,
    /// An array.
    Array,
    /// An object
    Object,
    /// Any number, floating point or integer.
    Number,
}

impl Display for TypeLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeLiteral::Null => write!(f, "null"),
            TypeLiteral::Int => write!(f, "int"),
            TypeLiteral::Bool => write!(f, "bool"),
            TypeLiteral::Float => write!(f, "float"),
            TypeLiteral::String => write!(f, "string"),
            TypeLiteral::Array => write!(f, "array"),
            TypeLiteral::Object => write!(f, "object"),
            TypeLiteral::Number => write!(f, "number"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IsExpression {
    lhs: Box<ExpressionType>,
    rhs: TypeLiteral,
    not: bool,
}

impl Display for IsExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.not {
            write!(f, "{} is not {}", self.lhs, self.rhs)
        } else {
            write!(f, "{} is {}", self.lhs, self.rhs)
        }
    }
}

impl<'a: 'c, 'c> Expression<'a, 'c> for IsExpression {
    fn resolve(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        state.inc_op()?;
        let lhs = self.lhs.resolve(state)?;
        let res = match self.rhs {
            TypeLiteral::Null => lhs.is_null(),
            TypeLiteral::Int => lhs.is_i64() || lhs.is_u64(),
            TypeLiteral::Bool => lhs.is_boolean(),
            TypeLiteral::Float => lhs.is_f64(),
            TypeLiteral::String => lhs.is_string(),
            TypeLiteral::Array => lhs.is_array(),
            TypeLiteral::Object => lhs.is_object(),
            TypeLiteral::Number => lhs.is_number(),
        };
        if self.not {
            Ok(ResolveResult::Owned(Value::Bool(!res)))
        } else {
            Ok(ResolveResult::Owned(Value::Bool(res)))
        }
    }
}

impl IsExpression {
    pub fn new(lhs: ExpressionType, rhs: TypeLiteral, not: bool) -> Result<Self, BuildError> {
        lhs.fail_if_lambda()?;
        Ok(Self {
            lhs: Box::new(lhs),
            rhs,
            not,
        })
    }
}

impl ExpressionMeta for IsExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new([self.lhs.as_mut()].into_iter())
    }
}
