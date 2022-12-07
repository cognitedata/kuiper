use std::fmt::Display;

use logos::Span;
use serde_json::Value;

use super::{
    base::{
        get_boolean_from_value, get_number_from_value, Expression, ExpressionExecutionState,
        ExpressionMeta, ExpressionType, JsonNumber, ResolveResult,
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
        }
    }
}

impl Operator {
    /// Get the operator priority. Higher numbers should be calculated last.
    /// This is roughly based on operator precedence in C++, which is what pretty much every language uses.
    pub fn priority(&self) -> i32 {
        match self {
            Self::Plus => 1,
            Self::Minus => 1,
            Self::Multiply => 2,
            Self::Divide => 2,
            Self::Equals => 4,
            Self::NotEquals => 4,
            Self::GreaterThan => 5,
            Self::LessThan => 5,
            Self::LessThanEquals => 5,
            Self::GreaterThanEquals => 5,
            Self::And => 6,
            Self::Or => 7,
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

impl<'a> Expression<'a> for OpExpression {
    fn resolve(
        &self,
        state: &ExpressionExecutionState,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let res = self.elements[0].resolve(state)?;
        let lhs = res.as_ref();
        if lhs.is_number() {
            self.resolve_numeric_operator(lhs, state)
        } else if matches!(
            self.operator,
            Operator::And | Operator::Or | Operator::Equals | Operator::NotEquals
        ) {
            self.resolve_boolean_operator(lhs, state)
        } else {
            Err(TransformError::new_invalid_operation(
                format!(
                    "Operator {} not applicable to {}",
                    &self.operator,
                    TransformError::value_desc(lhs)
                ),
                &self.span,
                state.id,
            ))
        }
    }
}

impl<'a> ExpressionMeta<'a> for OpExpression {
    fn num_children(&self) -> usize {
        1
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
    pub fn new(op: Operator, lhs: ExpressionType, rhs: ExpressionType, span: Span) -> Self {
        Self {
            operator: op,
            descriptor: format!("'{}'", &op),
            elements: [Box::new(lhs), Box::new(rhs)],
            span,
        }
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
            Operator::Equals => lhs == rhs,
            Operator::NotEquals => lhs != rhs,
            _ => {
                return Err(TransformError::new_invalid_operation(
                    format!("Operator {} not applicable to booleans", &self.operator),
                    &self.span,
                    state.id,
                ))
            }
        };

        Ok(ResolveResult::Value(Value::Bool(res)))
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
                return Ok(ResolveResult::Value(Value::Bool(lhs.cmp(
                    self.operator,
                    rhs,
                    &self.span,
                    state.id,
                ))))
            }
            Operator::Equals => {
                return Ok(ResolveResult::Value(Value::Bool(
                    lhs.eq(rhs, &self.span, state.id),
                )))
            }
            Operator::NotEquals => {
                return Ok(ResolveResult::Value(Value::Bool(
                    !lhs.eq(rhs, &self.span, state.id),
                )))
            }
            _ => {
                return Err(TransformError::new_invalid_operation(
                    format!("Operator {} not applicable to numbers", &self.operator),
                    &self.span,
                    state.id,
                ))
            }
        };
        Ok(ResolveResult::Value(res.try_into_json().ok_or_else(
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

impl<'a> Expression<'a> for UnaryOpExpression {
    fn resolve(
        &'a self,
        state: &'a ExpressionExecutionState,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let val = get_boolean_from_value(self.element.resolve(state)?.as_ref());
        match self.operator {
            UnaryOperator::Negate => Ok(ResolveResult::Value(Value::Bool(!val))),
        }
    }
}

impl<'a> ExpressionMeta<'a> for UnaryOpExpression {
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
    pub fn new(op: UnaryOperator, el: ExpressionType, span: Span) -> Self {
        Self {
            operator: op,
            descriptor: format!("'{}'", &op),
            element: Box::new(el),
            span,
        }
    }
}

impl JsonNumber {
    fn try_add(self, rhs: JsonNumber, span: &Span, id: &str) -> Result<JsonNumber, TransformError> {
        match (self, rhs) {
            (JsonNumber::PosInteger(x), JsonNumber::PosInteger(y)) => {
                Ok(JsonNumber::PosInteger(x + y))
            }
            (JsonNumber::NegInteger(x), JsonNumber::NegInteger(y)) => {
                Ok(JsonNumber::NegInteger(x + y))
            }
            (JsonNumber::Float(x), _) => Ok(JsonNumber::Float(x + rhs.as_f64())),
            (JsonNumber::NegInteger(x), JsonNumber::PosInteger(_)) => {
                Ok(JsonNumber::NegInteger(x + rhs.try_as_i64(span, id)?))
            }
            (_, JsonNumber::Float(y)) => Ok(JsonNumber::Float(self.as_f64() + y)),
            (JsonNumber::PosInteger(_), JsonNumber::NegInteger(y)) => {
                Ok(JsonNumber::NegInteger(self.try_as_i64(span, id)? + y))
            }
        }
    }
    fn try_sub(self, rhs: JsonNumber, span: &Span, id: &str) -> Result<JsonNumber, TransformError> {
        match (self, rhs) {
            (JsonNumber::PosInteger(x), JsonNumber::PosInteger(y)) => {
                if x >= y {
                    Ok(JsonNumber::PosInteger(x - y))
                } else {
                    Ok(JsonNumber::NegInteger(-((y - x).try_into()
                        .map_err(|_| TransformError::new_conversion_failed(
                            "Failed to convert result into negative integer, cannot produce a negative integer smaller than -9223372036854775808".to_string(), span, id))?)))
                }
            }
            (JsonNumber::NegInteger(x), JsonNumber::NegInteger(y)) => {
                Ok(JsonNumber::NegInteger(x - y))
            }
            (JsonNumber::Float(x), _) => Ok(JsonNumber::Float(x - rhs.as_f64())),
            (JsonNumber::NegInteger(x), JsonNumber::PosInteger(_)) => {
                Ok(JsonNumber::NegInteger(x - rhs.try_as_i64(span, id)?))
            }
            (_, JsonNumber::Float(y)) => Ok(JsonNumber::Float(self.as_f64() - y)),
            (JsonNumber::PosInteger(_), JsonNumber::NegInteger(y)) => {
                Ok(JsonNumber::NegInteger(self.try_as_i64(span, id)? - y))
            }
        }
    }
    fn try_mul(self, rhs: JsonNumber, span: &Span, id: &str) -> Result<JsonNumber, TransformError> {
        match (self, rhs) {
            (JsonNumber::PosInteger(x), JsonNumber::PosInteger(y)) => {
                Ok(JsonNumber::PosInteger(x * y))
            }
            (JsonNumber::NegInteger(x), JsonNumber::NegInteger(y)) => {
                Ok(JsonNumber::NegInteger(x * y))
            }
            (JsonNumber::Float(x), _) => Ok(JsonNumber::Float(x * rhs.as_f64())),
            (JsonNumber::NegInteger(x), JsonNumber::PosInteger(_)) => {
                Ok(JsonNumber::NegInteger(x * rhs.try_as_i64(span, id)?))
            }
            (_, JsonNumber::Float(y)) => Ok(JsonNumber::Float(self.as_f64() * y)),
            (JsonNumber::PosInteger(_), JsonNumber::NegInteger(y)) => {
                Ok(JsonNumber::NegInteger(self.try_as_i64(span, id)? * y))
            }
        }
    }
    fn try_div(self, rhs: JsonNumber, span: &Span, id: &str) -> Result<JsonNumber, TransformError> {
        if rhs.as_f64() == 0.0f64 {
            return Err(TransformError::new_invalid_operation(
                "Divide by zero".to_string(),
                span,
                id,
            ));
        }
        Ok(JsonNumber::Float(self.as_f64() / rhs.as_f64()))
    }

    fn cmp(self, op: Operator, rhs: JsonNumber, span: &Span, id: &str) -> bool {
        match (self, rhs) {
            (JsonNumber::PosInteger(x), JsonNumber::PosInteger(y)) => match op {
                Operator::LessThan => x < y,
                Operator::GreaterThan => x > y,
                Operator::LessThanEquals => x <= y,
                Operator::GreaterThanEquals => x >= y,
                _ => unreachable!(),
            },
            (JsonNumber::NegInteger(x), JsonNumber::NegInteger(y)) => match op {
                Operator::LessThan => x < y,
                Operator::GreaterThan => x > y,
                Operator::LessThanEquals => x <= y,
                Operator::GreaterThanEquals => x >= y,
                _ => unreachable!(),
            },
            (JsonNumber::Float(x), _) => match op {
                Operator::LessThan => x < rhs.as_f64(),
                Operator::GreaterThan => x > rhs.as_f64(),
                Operator::LessThanEquals => x <= rhs.as_f64(),
                Operator::GreaterThanEquals => x >= rhs.as_f64(),
                _ => unreachable!(),
            },
            (JsonNumber::NegInteger(x), JsonNumber::PosInteger(_)) => {
                let y = match rhs.try_as_i64(span, id) {
                    Ok(y) => y,
                    Err(_) => return matches!(op, Operator::LessThan | Operator::LessThanEquals),
                };
                match op {
                    Operator::LessThan => x < y,
                    Operator::GreaterThan => x > y,
                    Operator::LessThanEquals => x <= y,
                    Operator::GreaterThanEquals => x >= y,
                    _ => unreachable!(),
                }
            }
            (_, JsonNumber::Float(y)) => match op {
                Operator::LessThan => self.as_f64() < y,
                Operator::GreaterThan => self.as_f64() > y,
                Operator::LessThanEquals => self.as_f64() <= y,
                Operator::GreaterThanEquals => self.as_f64() >= y,
                _ => unreachable!(),
            },
            (JsonNumber::PosInteger(_), JsonNumber::NegInteger(y)) => {
                let x = match self.try_as_i64(span, id) {
                    Ok(x) => x,
                    Err(_) => {
                        return matches!(op, Operator::GreaterThan | Operator::GreaterThanEquals)
                    }
                };
                match op {
                    Operator::LessThan => x < y,
                    Operator::GreaterThan => x > y,
                    Operator::LessThanEquals => x <= y,
                    Operator::GreaterThanEquals => x >= y,
                    _ => unreachable!(),
                }
            }
        }
    }

    fn eq(self, rhs: JsonNumber, span: &Span, id: &str) -> bool {
        match (self, rhs) {
            (JsonNumber::PosInteger(x), JsonNumber::PosInteger(y)) => x == y,
            (JsonNumber::NegInteger(x), JsonNumber::NegInteger(y)) => x == y,
            (JsonNumber::Float(x), _) => x == rhs.as_f64(),
            (JsonNumber::NegInteger(x), JsonNumber::PosInteger(_)) => {
                match rhs.try_as_i64(span, id) {
                    Ok(y) => x == y,
                    Err(_) => false,
                }
            }
            (_, JsonNumber::Float(y)) => self.as_f64() == y,
            (JsonNumber::PosInteger(_), JsonNumber::NegInteger(y)) => {
                match self.try_as_i64(span, id) {
                    Ok(x) => x == y,
                    Err(_) => false,
                }
            }
        }
    }
}
