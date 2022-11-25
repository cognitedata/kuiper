use std::{fmt::Display, ops::Range};

use serde_json::{Number, Value};

use super::{
    base::{get_number_from_value, ExpressionType},
    transform_error::TransformError,
    Expression,
};

pub struct FunctionInfo {
    pub minargs: usize,
    pub maxargs: Option<usize>,
    pub name: &'static str,
}

impl FunctionInfo {
    pub fn validate(&self, num_args: usize) -> bool {
        if num_args < self.minargs {
            return false;
        }
        match self.maxargs {
            Some(x) if num_args > x => false,
            _ => true,
        }
    }

    pub fn num_args_desc(&self) -> String {
        match self.maxargs {
            Some(x) => format!(
                "function {} takes {} to {} arguments",
                self.name, self.minargs, x
            ),
            None => format!(
                "function {} takes at least {} arguments",
                self.name, self.minargs
            ),
        }
    }
}

pub trait FunctionExpression: Expression {
    const INFO: FunctionInfo;
}

pub struct PowFunction {
    base: Box<ExpressionType>,
    exponent: Box<ExpressionType>,
}

impl PowFunction {
    pub fn new(base: ExpressionType, exponent: ExpressionType) -> Self {
        Self {
            base: Box::new(base),
            exponent: Box::new(exponent),
        }
    }
}

impl Display for PowFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pow({}, {})", self.base, self.exponent)
    }
}

impl Expression for PowFunction {
    fn resolve(
        &self,
        state: &super::base::ExpressionExecutionState,
    ) -> Result<serde_json::Value, super::transform_error::TransformError> {
        let lhs = get_number_from_value("", self.base.resolve(state)?)?;
        let rhs = get_number_from_value("", self.exponent.resolve(state)?)?;

        let res = lhs.powf(rhs);

        Ok(Value::Number(Number::from_f64(res).ok_or_else(|| {
            TransformError::ConversionFailed(format!(
                "Failed to convert result of operator pow to number",
            ))
        })?))
    }
}

impl FunctionExpression for PowFunction {
    const INFO: FunctionInfo = FunctionInfo {
        minargs: 2,
        maxargs: Some(2),
        name: "pow",
    };
}
