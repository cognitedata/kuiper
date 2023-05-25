use std::fmt::Display;

use logos::Span;
use serde_json::Value;

use crate::compiler::BuildError;

use super::{
    base::{
        get_boolean_from_value, get_number_from_value, get_string_from_value, Expression,
        ExpressionExecutionState, ExpressionMeta, ExpressionType, ResolveResult,
    },
    transform_error::TransformError,
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
}

impl Display for UnaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Negate => write!(f, "!"),
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
        &self,
        state: &ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let lhs = self.elements[0].resolve(state)?;
        if matches!(self.operator, Operator::Is) {
            return self.resolve_is(&lhs, state);
        }

        if lhs.is_number() {
            self.resolve_numeric_operator(&lhs, state)
        } else if lhs.is_string()
            && !matches!(
                self.operator,
                Operator::And | Operator::Or | Operator::Equals | Operator::NotEquals
            )
        {
            self.resolve_string_operator(&lhs, state)
        } else if matches!(self.operator, Operator::And | Operator::Or) {
            self.resolve_boolean_operator(&lhs, state)
        } else {
            self.resolve_generic_operator(&lhs, state)
        }
    }
}

impl ExpressionMeta for OpExpression {
    fn num_children(&self) -> usize {
        2
    }

    fn get_child(&self, idx: usize) -> Option<&ExpressionType> {
        if idx > 1 {
            None
        } else {
            Some(&self.elements[idx])
        }
    }

    fn get_child_mut(&mut self, idx: usize) -> Option<&mut ExpressionType> {
        if idx > 1 {
            None
        } else {
            Some(&mut self.elements[idx])
        }
    }

    fn set_child(&mut self, idx: usize, item: ExpressionType) {
        if idx > 1 {
            return;
        }
        self.elements[idx] = Box::new(item);
    }
}

impl OpExpression {
    pub fn new(
        op: Operator,
        lhs: ExpressionType,
        rhs: ExpressionType,
        span: Span,
    ) -> Result<Self, BuildError> {
        for item in &[&lhs, &rhs] {
            if let ExpressionType::Lambda(lambda) = &item {
                return Err(BuildError::unexpected_lambda(&lambda.span));
            }
        }
        Ok(Self {
            operator: op,
            descriptor: format!("'{}'", &op),
            elements: [Box::new(lhs), Box::new(rhs)],
            span,
        })
    }

    fn resolve_is<'a>(
        &self,
        lhs: &Value,
        state: &ExpressionExecutionState,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let rhs = self.elements[1].resolve(state)?;
        let rhs_ref = rhs.as_ref().as_str();
        let Some(rhs_ref) = rhs_ref else {
            return Err(TransformError::new_incorrect_type("Right hand side of `is` operator", "string", TransformError::value_desc(&rhs), &self.span, state.id));
        };
        let res = match rhs_ref {
            "null" => lhs.is_null(),
            "object" => lhs.is_object(),
            "array" => lhs.is_array(),
            "string" => lhs.is_string(),
            "number" => lhs.is_number(),
            "float" => lhs.is_f64(),
            "int" => lhs.is_i64() || lhs.is_u64(),
            "bool" => lhs.is_boolean(),
            x => return Err(TransformError::new_invalid_operation(format!("{x} is not a valid type, expected 'null', 'object', 'array', 'string', 'number', 'float', 'int' or 'bool"), &self.span, state.id))
        };
        Ok(ResolveResult::Owned(Value::Bool(res)))
    }

    fn resolve_generic_operator<'a>(
        &self,
        lhs: &Value,
        state: &ExpressionExecutionState,
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
                    state.id,
                ))
            }
        };

        Ok(ResolveResult::Owned(Value::Bool(res)))
    }

    fn resolve_boolean_operator<'a>(
        &self,
        lhs: &Value,
        state: &ExpressionExecutionState,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let lhs = get_boolean_from_value(lhs);
        let rhs = get_boolean_from_value(self.elements[1].resolve(state)?.as_ref());

        let res = match &self.operator {
            Operator::And => lhs && rhs,
            Operator::Or => lhs || rhs,
            _ => {
                return Err(TransformError::new_invalid_operation(
                    format!("Operator {} not applicable to booleans", &self.operator),
                    &self.span,
                    state.id,
                ))
            }
        };

        Ok(ResolveResult::Owned(Value::Bool(res)))
    }

    fn resolve_string_operator<'a>(
        &self,
        lhs: &Value,
        state: &ExpressionExecutionState,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let lhs = get_string_from_value(&self.descriptor, lhs, &self.span, state.id)?;
        let rhs = self.elements[1].resolve(state)?;
        let rhs = get_string_from_value(&self.descriptor, &rhs, &self.span, state.id)?;

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
                    state.id,
                ))
            }
        };
        Ok(ResolveResult::Owned(Value::Bool(res)))
    }

    fn resolve_numeric_operator<'a>(
        &self,
        lhs: &Value,
        state: &ExpressionExecutionState,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let lhs = get_number_from_value(&self.descriptor, lhs, &self.span, state.id)?;
        let rhs = get_number_from_value(
            &self.descriptor,
            self.elements[1].resolve(state)?.as_ref(),
            &self.span,
            state.id,
        )?;

        let res = match &self.operator {
            Operator::Plus => lhs.try_add(rhs, &self.span, state.id)?,
            Operator::Minus => lhs.try_sub(rhs, &self.span, state.id)?,
            Operator::Multiply => lhs.try_mul(rhs, &self.span, state.id)?,
            Operator::Divide => lhs.try_div(rhs, &self.span, state.id)?,
            Operator::GreaterThan
            | Operator::LessThan
            | Operator::GreaterThanEquals
            | Operator::LessThanEquals => {
                return Ok(ResolveResult::Owned(Value::Bool(lhs.cmp(
                    self.operator,
                    rhs,
                    &self.span,
                    state.id,
                ))))
            }
            Operator::Equals => {
                return Ok(ResolveResult::Owned(Value::Bool(
                    lhs.eq(rhs, &self.span, state.id),
                )))
            }
            Operator::NotEquals => {
                return Ok(ResolveResult::Owned(Value::Bool(
                    !lhs.eq(rhs, &self.span, state.id),
                )))
            }
            Operator::Modulo => lhs.try_mod(rhs, &self.span, state.id)?,
            _ => {
                return Err(TransformError::new_invalid_operation(
                    format!("Operator {} not applicable to numbers", &self.operator),
                    &self.span,
                    state.id,
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
                    state.id,
                )
            },
        )?))
    }
}

#[derive(Debug, Clone)]
pub struct UnaryOpExpression {
    operator: UnaryOperator,
    #[allow(dead_code)]
    descriptor: String,
    element: Box<ExpressionType>,
    #[allow(dead_code)]
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
        state: &ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let val = get_boolean_from_value(self.element.resolve(state)?.as_ref());
        match self.operator {
            UnaryOperator::Negate => Ok(ResolveResult::Owned(Value::Bool(!val))),
        }
    }
}

impl ExpressionMeta for UnaryOpExpression {
    fn num_children(&self) -> usize {
        1
    }

    fn get_child(&self, idx: usize) -> Option<&ExpressionType> {
        if idx > 0 {
            None
        } else {
            Some(&self.element)
        }
    }

    fn get_child_mut(&mut self, idx: usize) -> Option<&mut ExpressionType> {
        if idx > 0 {
            None
        } else {
            Some(&mut self.element)
        }
    }

    fn set_child(&mut self, idx: usize, item: ExpressionType) {
        if idx > 0 {
            return;
        }
        self.element = Box::new(item);
    }
}

impl UnaryOpExpression {
    pub fn new(op: UnaryOperator, el: ExpressionType, span: Span) -> Result<Self, BuildError> {
        if let ExpressionType::Lambda(lambda) = &el {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }
        Ok(Self {
            operator: op,
            descriptor: format!("'{}'", &op),
            element: Box::new(el),
            span,
        })
    }
}
