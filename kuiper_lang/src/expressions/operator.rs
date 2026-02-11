use std::fmt::Display;

use logos::Span;
use serde_json::Value;

use crate::{
    compiler::BuildError,
    types::{Truthy, Type},
};

use super::{
    base::{Expression, ExpressionExecutionState, ExpressionMeta, ExpressionType},
    transform_error::TransformError,
    ResolveResult,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// A binary operator supported by kuiper.
pub enum Operator {
    /// Addition operator.
    Plus,
    /// Subtraction operator.
    Minus,
    /// Multiplication operator.
    Multiply,
    /// Division operator.
    Divide,
    /// Boolean AND operator.
    And,
    /// Boolean OR operator.
    Or,
    /// Strict equality operator.
    Equals,
    /// Strict inequality operator.
    NotEquals,
    /// Greater than operator.
    GreaterThan,
    /// Less than operator.
    LessThan,
    /// Greater than or equals operator.
    GreaterThanEquals,
    /// Less than or equals operator.
    LessThanEquals,
    /// Modulo (remainder) operator.
    Modulo,
    /// Type checking operator.
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
/// A unary operator supported by kuiper.
pub enum UnaryOperator {
    /// Logical negation operator, i.e. !true == false.
    Negate,
    /// Numeric negation operator. i.e. -5
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

        if matches!(self.operator, Operator::Equals | Operator::NotEquals) {
            self.resolve_equality(lhs, state)
        } else if matches!(self.operator, Operator::And | Operator::Or) {
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
            let rhs = self.elements[1].resolve(state)?;
            let rhs_ref = rhs.as_ref();
            Err(TransformError::new_invalid_operation(
                format!(
                    "Operator {} not applicable to {} and {}",
                    &self.operator,
                    TransformError::value_desc(&lhs),
                    TransformError::value_desc(rhs_ref)
                ),
                &self.span,
            ))
        }
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let lh = self.elements[0].resolve_types(state)?;
        let rh = self.elements[1].resolve_types(state)?;
        match self.operator {
            Operator::Plus | Operator::Minus | Operator::Multiply | Operator::Modulo => {
                lh.assert_assignable_to(&Type::number(), &self.span)?;
                rh.assert_assignable_to(&Type::number(), &self.span)?;

                if lh.is_integer() && rh.is_integer() {
                    Ok(Type::Integer)
                } else if lh.is_float() || rh.is_float() {
                    Ok(Type::Float)
                } else {
                    Ok(Type::number())
                }
            }
            Operator::Divide => {
                lh.assert_assignable_to(&Type::number(), &self.span)?;
                rh.assert_assignable_to(&Type::number(), &self.span)?;

                Ok(Type::Float)
            }
            Operator::And => {
                let lh = lh.truthyness();
                let rh = rh.truthyness();
                match (lh, rh) {
                    (Truthy::Always, Truthy::Always) => Ok(Type::from_const(true)),
                    (Truthy::Never, _) | (_, Truthy::Never) => Ok(Type::from_const(false)),
                    _ => Ok(Type::Boolean),
                }
            }
            Operator::Or => {
                let lh = lh.truthyness();
                let rh = rh.truthyness();
                match (lh, rh) {
                    (Truthy::Always, _) | (_, Truthy::Always) => Ok(Type::from_const(true)),
                    (Truthy::Never, Truthy::Never) => Ok(Type::from_const(false)),
                    _ => Ok(Type::Boolean),
                }
            }
            Operator::Equals => {
                if !lh.is_assignable_to(&rh) && (!lh.is_numeric() || !rh.is_numeric()) {
                    return Ok(Type::from_const(false));
                }
                if let Some(v) = lh.const_equality(&rh) {
                    return Ok(Type::from_const(v));
                }
                Ok(Type::Boolean)
            }
            Operator::NotEquals => {
                if !lh.is_assignable_to(&rh) && (!lh.is_numeric() || !rh.is_numeric()) {
                    return Ok(Type::from_const(true));
                }
                if let Some(v) = lh.const_equality(&rh) {
                    return Ok(Type::from_const(!v));
                }
                Ok(Type::Boolean)
            }
            Operator::GreaterThan
            | Operator::LessThan
            | Operator::GreaterThanEquals
            | Operator::LessThanEquals => {
                lh.assert_assignable_to(&Type::number(), &self.span)?;
                rh.assert_assignable_to(&Type::number(), &self.span)?;
                Ok(Type::Boolean)
            }
            Operator::Is => Ok(Type::Boolean),
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

    fn resolve_equality<'a: 'b, 'b>(
        &'a self,
        lhs: ResolveResult<'a>,
        state: &mut ExpressionExecutionState<'b, '_>,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let rhs = self.elements[1].resolve(state)?;

        let is_equal = if lhs.is_number() && rhs.is_number() {
            let lhs = lhs.try_as_number(&self.descriptor, &self.span)?;
            let rhs = rhs.try_as_number(&self.descriptor, &self.span)?;
            lhs.eq(rhs, &self.span)
        } else {
            lhs.eq(&*rhs)
        };

        match &self.operator {
            Operator::Equals => Ok(ResolveResult::Owned(Value::Bool(is_equal))),
            Operator::NotEquals => Ok(ResolveResult::Owned(Value::Bool(!is_equal))),
            _ => unreachable!(),
        }
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

    fn resolve_types(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let rhs = self.element.resolve_types(state)?;
        match self.operator {
            UnaryOperator::Negate => {
                let rhs = rhs.truthyness();
                match rhs {
                    Truthy::Always => Ok(Type::from_const(false)),
                    Truthy::Never => Ok(Type::from_const(true)),
                    Truthy::Maybe => Ok(Type::Boolean),
                }
            }
            UnaryOperator::Minus => {
                rhs.assert_assignable_to(&Type::number(), &self.span)?;
                let is_float = rhs.is_assignable_to(&Type::Float);
                let is_int = rhs.is_assignable_to(&Type::Integer);

                if is_float && is_int {
                    Ok(Type::number())
                } else if is_float {
                    Ok(Type::Float)
                } else {
                    Ok(Type::Integer)
                }
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

#[cfg(test)]
mod tests {
    use crate::{compile_expression, types::Type};

    #[test]
    fn test_arith_expr_types() {
        let expr = compile_expression("1 + input", &["input"]).unwrap();
        assert_eq!(expr.run_types([Type::Integer]).unwrap(), Type::Integer);
        assert_eq!(expr.run_types([Type::Float]).unwrap(), Type::Float);
        assert_eq!(expr.run_types([Type::number()]).unwrap(), Type::number());
        assert_eq!(
            expr.run_types([Type::from_const(15)]).unwrap(),
            Type::Integer
        );
        assert_eq!(expr.run_types([Type::Any]).unwrap(), Type::number());

        let err = expr.run_types([Type::String]).unwrap_err();
        assert_eq!(
            "Expected Union<Integer, Float> but got String",
            err.to_string()
        );
    }

    #[test]
    fn test_div_expr_types() {
        let expr = compile_expression("1 / input", &["input"]).unwrap();
        assert_eq!(expr.run_types([Type::Integer]).unwrap(), Type::Float);
        assert_eq!(expr.run_types([Type::Float]).unwrap(), Type::Float);
        assert_eq!(expr.run_types([Type::number()]).unwrap(), Type::Float);
        assert_eq!(expr.run_types([Type::from_const(15)]).unwrap(), Type::Float);
        assert_eq!(expr.run_types([Type::Any]).unwrap(), Type::Float);

        let err = expr.run_types([Type::String]).unwrap_err();
        assert_eq!(
            "Expected Union<Integer, Float> but got String",
            err.to_string()
        );
    }

    #[test]
    fn test_and_expr_types() {
        let expr = compile_expression("input1 && input2", &["input1", "input2"]).unwrap();
        assert_eq!(
            expr.run_types([Type::from_const(true), Type::from_const(true)])
                .unwrap(),
            Type::from_const(true)
        );
        assert_eq!(
            expr.run_types([Type::from_const(true), Type::from_const(false)])
                .unwrap(),
            Type::from_const(false)
        );
        assert_eq!(
            expr.run_types([Type::from_const(false), Type::from_const(false)])
                .unwrap(),
            Type::from_const(false)
        );
        assert_eq!(
            expr.run_types([Type::Boolean, Type::Boolean]).unwrap(),
            Type::Boolean
        );
        assert_eq!(
            expr.run_types([Type::Integer, Type::Integer]).unwrap(),
            Type::from_const(true)
        );
        assert_eq!(
            expr.run_types([Type::null(), Type::Boolean]).unwrap(),
            Type::from_const(false)
        );
    }

    #[test]
    fn test_or_expr_types() {
        let expr = compile_expression("input1 || input2", &["input1", "input2"]).unwrap();
        assert_eq!(
            expr.run_types([Type::from_const(true), Type::from_const(true)])
                .unwrap(),
            Type::from_const(true)
        );
        assert_eq!(
            expr.run_types([Type::from_const(true), Type::from_const(false)])
                .unwrap(),
            Type::from_const(true)
        );
        assert_eq!(
            expr.run_types([Type::from_const(false), Type::from_const(false)])
                .unwrap(),
            Type::from_const(false)
        );
        assert_eq!(
            expr.run_types([Type::Boolean, Type::Boolean]).unwrap(),
            Type::Boolean
        );
        assert_eq!(
            expr.run_types([Type::Integer, Type::Integer]).unwrap(),
            Type::from_const(true)
        );
        assert_eq!(
            expr.run_types([Type::null(), Type::Boolean]).unwrap(),
            Type::Boolean
        );
    }

    #[test]
    fn test_equality_expr_types() {
        let expr = compile_expression("input1 == input2", &["input1", "input2"]).unwrap();
        assert_eq!(
            expr.run_types([Type::from_const(5), Type::from_const(5)])
                .unwrap(),
            Type::from_const(true)
        );
        assert_eq!(
            expr.run_types([Type::from_const(5), Type::from_const(6)])
                .unwrap(),
            Type::from_const(false)
        );
        assert_eq!(
            expr.run_types([Type::Integer, Type::Integer]).unwrap(),
            Type::Boolean
        );
        assert_eq!(
            expr.run_types([Type::Integer, Type::Float]).unwrap(),
            Type::Boolean
        );
        assert_eq!(
            expr.run_types([Type::String, Type::Integer]).unwrap(),
            Type::from_const(false)
        );
    }

    #[test]
    fn test_negate_expr_types() {
        let expr = compile_expression("!input", &["input"]).unwrap();
        assert_eq!(
            expr.run_types([Type::from_const(true)]).unwrap(),
            Type::from_const(false)
        );
        assert_eq!(
            expr.run_types([Type::from_const(false)]).unwrap(),
            Type::from_const(true)
        );
        assert_eq!(expr.run_types([Type::Boolean]).unwrap(), Type::Boolean);
        assert_eq!(
            expr.run_types([Type::Integer]).unwrap(),
            Type::from_const(false)
        );
    }

    #[test]
    fn test_minus_expr_types() {
        let expr = compile_expression("-input", &["input"]).unwrap();
        assert_eq!(
            expr.run_types([Type::from_const(5)]).unwrap(),
            Type::Integer
        );
        assert_eq!(
            expr.run_types([Type::from_const(-3.5)]).unwrap(),
            Type::Float
        );
        assert_eq!(expr.run_types([Type::Integer]).unwrap(), Type::Integer);
        assert_eq!(expr.run_types([Type::Float]).unwrap(), Type::Float);
        assert_eq!(expr.run_types([Type::number()]).unwrap(), Type::number());
        assert_eq!(expr.run_types([Type::Any]).unwrap(), Type::number());
    }
}
