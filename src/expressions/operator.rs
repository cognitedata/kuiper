use std::fmt::Display;

use serde_json::{Number, Value};

use super::{
    base::{get_number_from_value, Expression, ExpressionExecutionState, ExpressionType},
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

pub struct OpExpression {
    operator: Operator,
    descriptor: String,
    elements: [Box<ExpressionType>; 2],
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

impl Expression for OpExpression {
    fn resolve(&self, state: &ExpressionExecutionState) -> Result<Value, TransformError> {
        let lhs = get_number_from_value(&self.descriptor, self.elements[0].resolve(state)?)?;
        let rhs = get_number_from_value(&self.descriptor, self.elements[1].resolve(state)?)?;

        let res = match &self.operator {
            Operator::Plus => lhs + rhs,
            Operator::Minus => lhs - rhs,
            Operator::Multiply => lhs * rhs,
            Operator::Divide => lhs / rhs,
        };
        Ok(Value::Number(Number::from_f64(res).ok_or_else(|| {
            TransformError::ConversionFailed(format!(
                "Failed to convert result of operator {} to number",
                &self.descriptor
            ))
        })?))
    }
}

impl OpExpression {
    pub fn new(op: Operator, lhs: ExpressionType, rhs: ExpressionType) -> Self {
        Self {
            operator: op,
            descriptor: "".to_string(), //TODO: Make this actually useful
            elements: [Box::new(lhs), Box::new(rhs)],
        }
    }
}
