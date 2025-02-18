use logos::Span;
use serde_json::{Number, Value};

use crate::TransformError;

use super::Operator;

/// Our representation of a number in JSON. Contains methods for doing arithmatic safely, which is somewhat complicated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum JsonNumber {
    NegInteger(i64),
    PosInteger(u64),
    Float(f64),
}

impl From<&serde_json::Number> for JsonNumber {
    fn from(v: &serde_json::Number) -> Self {
        v.as_u64()
            .map(JsonNumber::PosInteger)
            .or_else(|| v.as_i64().map(JsonNumber::NegInteger))
            .or_else(|| v.as_f64().map(JsonNumber::Float))
            .unwrap()
    }
}

impl From<serde_json::Number> for JsonNumber {
    fn from(v: serde_json::Number) -> Self {
        Self::from(&v)
    }
}

impl From<i64> for JsonNumber {
    fn from(value: i64) -> Self {
        if value < 0 {
            Self::NegInteger(value)
        } else {
            Self::PosInteger(value as u64)
        }
    }
}

impl From<u64> for JsonNumber {
    fn from(value: u64) -> Self {
        Self::PosInteger(value)
    }
}

impl From<f64> for JsonNumber {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl JsonNumber {
    /// Convert to a float, this cannot fail, but it can lose precision.
    pub fn as_f64(self) -> f64 {
        match self {
            Self::NegInteger(x) => x as f64,
            Self::PosInteger(x) => x as f64,
            Self::Float(x) => x,
        }
    }

    /// Try to convert the number to an unsigned integer. This will fail if the number
    /// is negative, if it's not a whole number, or if it is larger than u64::MAX.
    #[allow(dead_code)]
    pub fn try_as_u64(self, span: &Span) -> Result<u64, TransformError> {
        match self {
            Self::NegInteger(x) => x.try_into().map_err(|e| {
                TransformError::new_conversion_failed(
                    format!("Failed to convert negative integer {x} to unsigned: {e}"),
                    span,
                )
            }),
            Self::PosInteger(x) => Ok(x),
            Self::Float(x) => {
                if x.fract() != 0.0f64 {
                    Err(TransformError::new_conversion_failed(
                        format!("Failed to convert floating point number {x} to integer: not a whole number"),
                        span,
                    ))
                } else if x <= u64::MAX as f64 && x >= u64::MIN as f64 {
                    Ok(x as u64)
                } else {
                    Err(TransformError::new_conversion_failed(
                        format!("Failed to convert floating point number {x} to positive integer: number does not fit within (0, 18446744073709551615)"), span))
                }
            }
        }
    }

    /// Try to convert the number to a signed integer. This will fail if the number
    /// is out of bounds, or not a whole number.
    pub fn try_as_i64(self, span: &Span) -> Result<i64, TransformError> {
        match self {
            Self::PosInteger(x) => x.try_into().map_err(|e| {
                TransformError::new_conversion_failed(
                    format!("Failed to convert positive integer to signed integer: {e}"),
                    span,
                )
            }),
            Self::NegInteger(x) => Ok(x),
            Self::Float(x) => {
                if x.fract() != 0.0f64 {
                    Err(TransformError::new_conversion_failed(
                        "Failed to convert floating point number to integer: not a whole number"
                            .to_string(),
                        span,
                    ))
                } else if x <= i64::MAX as f64 && x >= i64::MIN as f64 {
                    Ok(x as i64)
                } else {
                    Err(TransformError::new_conversion_failed(
                        "Failed to convert floating point number to integer: number does not fit within (-9223372036854775808, 9223372036854775807)".to_string(), span))
                }
            }
        }
    }

    /// Try to convert the number into JSON, this will only fail if it is NaN or Infinity.
    pub fn try_into_json(self) -> Option<Value> {
        match self {
            Self::NegInteger(x) => Some(Value::Number(x.into())),
            Self::PosInteger(x) => Some(Value::Number(x.into())),
            Self::Float(x) => Number::from_f64(x).map(Value::Number),
        }
    }

    /// Try to cast into an integer, either positive or negative. This will remove any fractional part if it is a floating point number.
    pub fn try_cast_integer(self, span: &Span) -> Result<JsonNumber, TransformError> {
        match self {
            JsonNumber::NegInteger(_) | JsonNumber::PosInteger(_) => Ok(self),
            JsonNumber::Float(x) => {
                if x >= 0.0 && x <= u64::MAX as f64 {
                    Ok(JsonNumber::PosInteger(x as u64))
                } else if x < 0.0 && x >= i64::MIN as f64 {
                    Ok(JsonNumber::NegInteger(x as i64))
                } else {
                    Err(TransformError::new_conversion_failed(
                        format!(
                            "Failed to convert floating point number {x} to integer, too large."
                        ),
                        span,
                    ))
                }
            }
        }
    }

    /// Try to add two numbers, the result type depends on the input.
    pub fn try_add(self, rhs: JsonNumber, span: &Span) -> Result<JsonNumber, TransformError> {
        match (self, rhs) {
            (JsonNumber::PosInteger(x), JsonNumber::PosInteger(y)) => Ok(JsonNumber::PosInteger(
                x.checked_add(y)
                    .ok_or_else(|| TransformError::new_arith_overflow(span))?,
            )),
            (JsonNumber::NegInteger(x), JsonNumber::NegInteger(y)) => Ok(JsonNumber::NegInteger(
                x.checked_add(y)
                    .ok_or_else(|| TransformError::new_arith_overflow(span))?,
            )),
            (JsonNumber::Float(x), _) => Ok(JsonNumber::Float(x + rhs.as_f64())),
            (JsonNumber::NegInteger(x), JsonNumber::PosInteger(y)) => Ok(JsonNumber::NegInteger(
                x.checked_add_unsigned(y)
                    .ok_or_else(|| TransformError::new_arith_overflow(span))?,
            )),
            (_, JsonNumber::Float(y)) => Ok(JsonNumber::Float(self.as_f64() + y)),
            (JsonNumber::PosInteger(_), JsonNumber::NegInteger(y)) => Ok(JsonNumber::NegInteger(
                self.try_as_i64(span)?
                    .checked_add(y)
                    .ok_or_else(|| TransformError::new_arith_overflow(span))?,
            )),
        }
    }

    /// Try to subtract a number from self, result depends on input types.
    pub fn try_sub(self, rhs: JsonNumber, span: &Span) -> Result<JsonNumber, TransformError> {
        match (self, rhs) {
            (JsonNumber::PosInteger(x), JsonNumber::PosInteger(y)) => {
                if x >= y {
                    Ok(JsonNumber::PosInteger(x - y))
                } else {
                    Ok(JsonNumber::NegInteger(-((y - x).try_into()
                        .map_err(|_| TransformError::new_conversion_failed(
                            "Failed to convert result into negative integer, cannot produce a negative integer smaller than -9223372036854775808".to_string(), span))?)))
                }
            }
            (JsonNumber::NegInteger(x), JsonNumber::NegInteger(y)) => Ok(JsonNumber::NegInteger(
                x.checked_sub(y)
                    .ok_or_else(|| TransformError::new_arith_overflow(span))?,
            )),
            (JsonNumber::Float(x), _) => Ok(JsonNumber::Float(x - rhs.as_f64())),
            (JsonNumber::NegInteger(x), JsonNumber::PosInteger(y)) => Ok(JsonNumber::NegInteger(
                x.checked_sub_unsigned(y)
                    .ok_or_else(|| TransformError::new_arith_overflow(span))?,
            )),
            (_, JsonNumber::Float(y)) => Ok(JsonNumber::Float(self.as_f64() - y)),
            (JsonNumber::PosInteger(_), JsonNumber::NegInteger(y)) => Ok(JsonNumber::NegInteger(
                self.try_as_i64(span)?
                    .checked_sub(y)
                    .ok_or_else(|| TransformError::new_arith_overflow(span))?,
            )),
        }
    }

    /// Try to multiply two numbers, result depends on input types.
    pub fn try_mul(self, rhs: JsonNumber, span: &Span) -> Result<JsonNumber, TransformError> {
        match (self, rhs) {
            (JsonNumber::PosInteger(x), JsonNumber::PosInteger(y)) => Ok(JsonNumber::PosInteger(
                x.checked_mul(y)
                    .ok_or_else(|| TransformError::new_arith_overflow(span))?,
            )),
            (JsonNumber::NegInteger(x), JsonNumber::NegInteger(y)) => Ok(JsonNumber::NegInteger(
                x.checked_mul(y)
                    .ok_or_else(|| TransformError::new_arith_overflow(span))?,
            )),
            (JsonNumber::Float(x), _) => Ok(JsonNumber::Float(x * rhs.as_f64())),
            (JsonNumber::NegInteger(x), JsonNumber::PosInteger(_)) => Ok(JsonNumber::NegInteger(
                x.checked_mul(rhs.try_as_i64(span)?)
                    .ok_or_else(|| TransformError::new_arith_overflow(span))?,
            )),
            (_, JsonNumber::Float(y)) => Ok(JsonNumber::Float(self.as_f64() * y)),
            (JsonNumber::PosInteger(_), JsonNumber::NegInteger(y)) => Ok(JsonNumber::NegInteger(
                self.try_as_i64(span)?
                    .checked_mul(y)
                    .ok_or_else(|| TransformError::new_arith_overflow(span))?,
            )),
        }
    }

    /// Try to divide self by a number, result is floating point.
    pub fn try_div(self, rhs: JsonNumber, span: &Span) -> Result<JsonNumber, TransformError> {
        if rhs.as_f64() == 0.0f64 {
            return Err(TransformError::new_invalid_operation(
                "Divide by zero".to_string(),
                span,
            ));
        }
        Ok(JsonNumber::Float(self.as_f64() / rhs.as_f64()))
    }

    /// Try to perform a comparison between two numbers, this cannot fail.
    /// Operator must be either LessThan, GreaterThan, LessThanEquals, or GreaterThanEquals.
    pub fn cmp(self, op: Operator, rhs: JsonNumber, span: &Span) -> bool {
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
                let y = match rhs.try_as_i64(span) {
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
                let x = match self.try_as_i64(span) {
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

    /// Check if self is equal to rhs, will do casts and conversions as necessary, but it will avoid
    /// anything that reduces precision.
    pub fn eq(self, rhs: JsonNumber, span: &Span) -> bool {
        match (self, rhs) {
            (JsonNumber::PosInteger(x), JsonNumber::PosInteger(y)) => x == y,
            (JsonNumber::NegInteger(x), JsonNumber::NegInteger(y)) => x == y,
            (JsonNumber::Float(x), _) => x == rhs.as_f64(),
            (JsonNumber::NegInteger(x), JsonNumber::PosInteger(_)) => match rhs.try_as_i64(span) {
                Ok(y) => x == y,
                Err(_) => false,
            },
            (_, JsonNumber::Float(y)) => self.as_f64() == y,
            (JsonNumber::PosInteger(_), JsonNumber::NegInteger(y)) => match self.try_as_i64(span) {
                Ok(x) => x == y,
                Err(_) => false,
            },
        }
    }

    pub fn try_mod(self, rhs: JsonNumber, span: &Span) -> Result<JsonNumber, TransformError> {
        if rhs.as_f64() == 0.0f64 {
            return Err(TransformError::new_invalid_operation(
                "Divide by zero".to_string(),
                span,
            ));
        }
        match (self, rhs) {
            (JsonNumber::PosInteger(x), JsonNumber::PosInteger(y)) => {
                Ok(JsonNumber::PosInteger(x % y))
            }
            (JsonNumber::NegInteger(x), JsonNumber::NegInteger(y)) => {
                Ok(JsonNumber::NegInteger(x % y))
            }
            (JsonNumber::Float(x), f) => Ok(JsonNumber::Float(x % f.as_f64())),
            (JsonNumber::PosInteger(_), JsonNumber::NegInteger(y)) => {
                Ok(JsonNumber::NegInteger(self.try_as_i64(span)? % y))
            }
            (_, JsonNumber::Float(y)) => Ok(JsonNumber::Float(self.as_f64() % y)),
            (JsonNumber::NegInteger(x), JsonNumber::PosInteger(_)) => {
                Ok(JsonNumber::NegInteger(x % rhs.try_as_i64(span)?))
            }
        }
    }

    pub fn neg(self) -> JsonNumber {
        match self {
            JsonNumber::NegInteger(x) => {
                if x < 0 {
                    JsonNumber::PosInteger((-x) as u64)
                } else {
                    JsonNumber::NegInteger(-x)
                }
            }
            JsonNumber::PosInteger(x) => {
                if x <= i64::MAX as u64 {
                    JsonNumber::NegInteger(-(x as i64))
                } else {
                    JsonNumber::Float(-(x as f64))
                }
            }
            JsonNumber::Float(x) => JsonNumber::Float(-x),
        }
    }
}
