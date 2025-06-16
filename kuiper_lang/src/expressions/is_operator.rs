use std::fmt::Display;

use serde_json::Value;

use crate::{
    expressions::types::{Truthy, Type},
    BuildError, ExpressionType, TransformError,
};

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

    fn resolve_types(
        &'a self,
        state: &mut super::types::TypeExecutionState<'c, '_>,
    ) -> Result<super::types::Type, super::types::TypeError> {
        let lhs = self.lhs.resolve_types(state)?;
        match Self::matches_type(self.rhs, &lhs) {
            Truthy::Always => Ok(Type::Constant(Value::Bool(!self.not))),
            Truthy::Maybe => Ok(Type::Boolean),
            Truthy::Never => Ok(Type::Constant(Value::Bool(self.not))),
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

    fn matches_type(lit: TypeLiteral, rhs: &Type) -> Truthy {
        match (lit, rhs) {
            (l, Type::Union(r)) => {
                if r.iter()
                    .all(|t| matches!(Self::matches_type(l, t), Truthy::Never))
                {
                    Truthy::Never
                } else {
                    Truthy::Maybe
                }
            }
            (_, Type::Any) => Truthy::Maybe,
            (TypeLiteral::Null, Type::Constant(Value::Null)) => Truthy::Always,
            (TypeLiteral::Int, Type::Constant(v)) if v.is_i64() || v.is_u64() => Truthy::Always,
            (TypeLiteral::Bool, Type::Constant(Value::Bool(_))) => Truthy::Always,
            (TypeLiteral::Float, Type::Constant(Value::Number(v))) if v.is_f64() => Truthy::Always,
            (TypeLiteral::String, Type::Constant(Value::String(_))) => Truthy::Always,
            (TypeLiteral::Array, Type::Constant(Value::Array(_))) => Truthy::Always,
            (TypeLiteral::Object, Type::Constant(Value::Object(_))) => Truthy::Always,
            (TypeLiteral::Number, Type::Constant(Value::Number(_))) => Truthy::Always,
            _ => Truthy::Never,
        }
    }
}

impl ExpressionMeta for IsExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new([self.lhs.as_mut()].into_iter())
    }
}
