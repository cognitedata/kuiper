use std::fmt::Display;

use logos::Span;
use serde_json::Value;

use crate::compiler::BuildError;

use super::{
    base::{Expression, ExpressionExecutionState, ExpressionMeta, ExpressionType},
    transform_error::TransformError,
    ResolveResult,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Operator {
    Plus,
    Minus,
    Multiply,
    Divide,
    And,
    Or,
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanEquals,
    LessThanEquals,
    Modulo,
    Is,
}

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operator::Plus => write!(f, "+"),
            Operator::Minus => write!(f, "-"),
            Operator::Multiply => write!(f, "*"),
            Operator::Divide => write!(f, "/"),
            Operator::And => write!(f, "&&"),
            Operator::Or => write!(f, "||"),
            Operator::Equals => write!(f, "=="),
            Operator::NotEquals => write!(f, "!="),
            Operator::GreaterThan => write!(f, ">"),
            Operator::LessThan => write!(f, "<"),
            Operator::GreaterThanEquals => write!(f, ">="),
            Operator::LessThanEquals => write!(f, "<="),
            Operator::Modulo => write!(f, "%"),
            Operator::Is => write!(f, "is"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnaryOperator {
    Negate,
    Minus,
}

impl Display for UnaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Negate => write!(f, "!"),
            Self::Minus => write!(f, "-"),
        }
    }
}

#[derive(Debug, Clone)]
/// Expression for an operator. Consists of two expressions, and an operator.
pub struct OpExpression {
    operator: Operator,
    descriptor: String,
    elements: [Box<ExpressionType>; 2],
    span: Span,
}

impl Display for OpExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({} {} {})",
            self.elements[0], self.operator, self.elements[1]
        )
    }
}

impl<'a: 'c, 'c> Expression<'a, 'c> for OpExpression {
    fn resolve(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        state.inc_op()?;
        let lhs = self.elements[0].resolve(state)?;
        if matches!(self.operator, Operator::And | Operator::Or) {
            self.resolve_boolean_operator(lhs, state)
        } else if lhs.is_string()
            && !matches!(
                self.operator,
                Operator::And | Operator::Or | Operator::Equals | Operator::NotEquals
            )
        {
            self.resolve_string_operator(lhs, state)
        } else if lhs.is_number() {
            self.resolve_numeric_operator(lhs, state)
        } else {
            self.resolve_generic_operator(&lhs, state)
        }
    }
}

impl ExpressionMeta for OpExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new(self.elements.iter_mut().map(|m| m.as_mut()))
    }
}

impl OpExpression {
    pub fn new(
        op: Operator,
        lhs: ExpressionType,
        rhs: ExpressionType,
        span: Span,
    ) -> Result<Self, BuildError> {
        lhs.fail_if_lambda()?;
        rhs.fail_if_lambda()?;
        Ok(Self {
            operator: op,
            descriptor: format!("'{}'", &op),
            elements: [Box::new(lhs), Box::new(rhs)],
            span,
        })
    }

    fn resolve_generic_operator<'a: 'b, 'b>(
        &'a self,
        lhs: &Value,
        state: &mut ExpressionExecutionState<'b, '_>,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let rhs = self.elements[1].resolve(state)?;
        let rhs_ref = rhs.as_ref();

        let res = match &self.operator {
            Operator::Equals => lhs.eq(rhs_ref),
            Operator::NotEquals => !lhs.eq(rhs_ref),
            _ => {
                return Err(TransformError::new_invalid_operation(
                    format!(
                        "Operator {} not applicable to {} and {}",
                        &self.operator,
                        TransformError::value_desc(lhs),
                        TransformError::value_desc(rhs_ref)
                    ),
                    &self.span,
                ))
            }
        };

        Ok(ResolveResult::Owned(Value::Bool(res)))
    }

    fn resolve_boolean_operator<'a: 'b, 'b>(
        &'a self,
        lhs: ResolveResult<'a>,
        state: &mut ExpressionExecutionState<'b, '_>,
    ) -> Result<ResolveResult<'b>, TransformError> {
        let lhs = lhs.as_bool();
        let rhs = self.elements[1].resolve(state)?.as_bool();

        let res = match &self.operator {
            Operator::And => lhs && rhs,
            Operator::Or => lhs || rhs,
            _ => {
                return Err(TransformError::new_invalid_operation(
                    format!("Operator {} not applicable to booleans", &self.operator),
                    &self.span,
                ))
            }
        };

        Ok(ResolveResult::Owned(Value::Bool(res)))
    }

    fn resolve_string_operator<'a: 'b, 'b>(
        &'a self,
        lhs: ResolveResult<'b>,
        state: &mut ExpressionExecutionState<'b, '_>,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let lhs = lhs.try_into_string(&self.descriptor, &self.span)?;
        let rhs = self.elements[1].resolve(state)?;
        let rhs = rhs.try_into_string(&self.descriptor, &self.span)?;

        let res = match &self.operator {
            Operator::Equals => lhs == rhs,
            Operator::NotEquals => lhs != rhs,
            Operator::GreaterThan => lhs > rhs,
            Operator::LessThan => lhs < rhs,
            Operator::GreaterThanEquals => lhs >= rhs,
            Operator::LessThanEquals => lhs <= rhs,
            _ => {
                return Err(TransformError::new_invalid_operation(
                    format!("Operator {} not applicable to strings", &self.operator),
                    &self.span,
                ))
            }
        };
        Ok(ResolveResult::Owned(Value::Bool(res)))
    }

    fn resolve_numeric_operator<'a: 'b, 'b>(
        &'a self,
        lhs: ResolveResult<'a>,
        state: &mut ExpressionExecutionState<'b, '_>,
    ) -> Result<ResolveResult<'b>, TransformError> {
        let lhs = lhs.try_as_number(&self.descriptor, &self.span)?;
        let rhs = self.elements[1]
            .resolve(state)?
            .try_as_number(&self.descriptor, &self.span)?;

        let res = match &self.operator {
            Operator::Plus => lhs.try_add(rhs, &self.span)?,
            Operator::Minus => lhs.try_sub(rhs, &self.span)?,
            Operator::Multiply => lhs.try_mul(rhs, &self.span)?,
            Operator::Divide => lhs.try_div(rhs, &self.span)?,
            Operator::GreaterThan
            | Operator::LessThan
            | Operator::GreaterThanEquals
            | Operator::LessThanEquals => {
                return Ok(ResolveResult::Owned(Value::Bool(lhs.cmp(
                    self.operator,
                    rhs,
                    &self.span,
                ))))
            }
            Operator::Equals => {
                return Ok(ResolveResult::Owned(Value::Bool(lhs.eq(rhs, &self.span))))
            }
            Operator::NotEquals => {
                return Ok(ResolveResult::Owned(Value::Bool(!lhs.eq(rhs, &self.span))))
            }
            Operator::Modulo => lhs.try_mod(rhs, &self.span)?,
            _ => {
                return Err(TransformError::new_invalid_operation(
                    format!("Operator {} not applicable to numbers", &self.operator),
                    &self.span,
                ))
            }
        };
        Ok(ResolveResult::Owned(res.try_into_json().ok_or_else(
            || {
                TransformError::new_conversion_failed(
                    format!(
                        "Failed to convert result of operator {} to number",
                        &self.descriptor
                    ),
                    &self.span,
                )
            },
        )?))
    }
}

#[derive(Debug, Clone)]
pub struct UnaryOpExpression {
    operator: UnaryOperator,
    descriptor: String,
    element: Box<ExpressionType>,
    span: Span,
}

impl Display for UnaryOpExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.operator, self.element)
    }
}

impl<'a: 'c, 'c> Expression<'a, 'c> for UnaryOpExpression {
    fn resolve(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let rhs = self.element.resolve(state)?;
        match self.operator {
            UnaryOperator::Negate => Ok(ResolveResult::Owned(Value::Bool(!rhs.as_bool()))),
            UnaryOperator::Minus => {
                let val = rhs.try_as_number(&self.descriptor, &self.span)?;
                Ok(ResolveResult::Owned(
                    // This being option shouldn't be possible. We should never be able to get a NaN here.
                    val.neg().try_into_json().unwrap_or_default(),
                ))
            }
        }
    }
}

impl ExpressionMeta for UnaryOpExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new([self.element.as_mut()].into_iter())
    }
}

impl UnaryOpExpression {
    pub fn new(op: UnaryOperator, el: ExpressionType, span: Span) -> Result<Self, BuildError> {
        el.fail_if_lambda()?;
        Ok(Self {
            operator: op,
            descriptor: format!("'{}'", &op),
            element: Box::new(el),
            span,
        })
    }
}
