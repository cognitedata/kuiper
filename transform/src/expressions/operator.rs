use std::fmt::Display;

use logos::Span;

use super::{
    base::{
        get_number_from_value, Expression, ExpressionExecutionState, ExpressionType, JsonNumber,
        ResolveResult,
    },
    transform_error::TransformError,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Operator {
    Plus,
    Minus,
    Multiply,
    Divide,
}

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operator::Plus => write!(f, "+"),
            Operator::Minus => write!(f, "-"),
            Operator::Multiply => write!(f, "*"),
            Operator::Divide => write!(f, "/"),
        }
    }
}

impl Operator {
    /// Get the operator priority. Higher numbers should be calculated last.
    pub fn priority(&self) -> i32 {
        match self {
            Self::Plus => 1,
            Self::Minus => 1,
            Self::Multiply => 2,
            Self::Divide => 2,
        }
    }
}

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
        let lhs = get_number_from_value(
            &self.descriptor,
            self.elements[0].resolve(state)?.as_ref(),
            &self.span,
            state.id,
        )?;
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
}
