use serde_json::{Number, Value};

use super::{
    base::{Expression, ExpressionExecutionState},
    transform_error::TransformError,
};

#[derive(Debug, PartialEq)]
pub enum Operator {
    Plus,
    Minus,
    Multiply,
    Divide,
}

struct OpExpression {
    operator: Operator,
    descriptor: String,
    elements: [Box<dyn Expression>; 2],
}

impl Expression for OpExpression {
    fn resolve(&self, state: &ExpressionExecutionState) -> Result<Value, TransformError> {
        let lhs = self.get_number_from_value(self.elements[0].resolve(state)?)?;
        let rhs = self.get_number_from_value(self.elements[1].resolve(state)?)?;

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
    fn get_number_from_value(&self, val: Value) -> Result<f64, TransformError> {
        let v = match val {
            Value::Number(n) => n,
            _ => {
                return Err(TransformError::new_incorrect_type(
                    &self.descriptor,
                    "number",
                    &val,
                ))
            }
        };
        return v.as_f64().ok_or_else(|| {
            TransformError::ConversionFailed(format!(
                "Failed to convert field into number for operator {}",
                &self.descriptor
            ))
        });
    }
}
