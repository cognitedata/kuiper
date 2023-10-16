use std::fmt::Display;

use serde_json::Value;

use crate::{BuildError, ExpressionType, TransformError};

use super::{Expression, ExpressionExecutionState, ExpressionMeta, ResolveResult};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TypeLiteral {
    Null,
    Int,
    Bool,
    Float,
    String,
    Array,
    Object,
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
}

impl Display for IsExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is {}", self.lhs, self.rhs)
    }
}

impl<'a: 'c, 'c> Expression<'a, 'c> for IsExpression {
    fn resolve(
        &'a self,
        state: &ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
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
        Ok(ResolveResult::Owned(Value::Bool(res)))
    }
}

impl IsExpression {
    pub fn new(lhs: ExpressionType, rhs: TypeLiteral) -> Result<Self, BuildError> {
        if let ExpressionType::Lambda(lambda) = &lhs {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }
        Ok(Self {
            lhs: Box::new(lhs),
            rhs,
        })
    }
}

impl ExpressionMeta for IsExpression {
    fn num_children(&self) -> usize {
        1
    }

    fn get_child(&self, idx: usize) -> Option<&ExpressionType> {
        if idx > 0 {
            None
        } else {
            Some(&self.lhs)
        }
    }

    fn get_child_mut(&mut self, idx: usize) -> Option<&mut ExpressionType> {
        if idx > 0 {
            None
        } else {
            Some(&mut self.lhs)
        }
    }

    fn set_child(&mut self, idx: usize, item: ExpressionType) {
        if idx > 0 {
            return;
        }
        self.lhs = Box::new(item);
    }
}
