use std::ops::Neg;

use logos::Span;
use serde_json::{Number, Value};

use crate::TransformError;

use super::Operator;

/// Our representation of a number in JSON. Contains methods for doing arithmatic safely, which is somewhat complicated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JsonNumber {
    /// A negative integer, stored as an i64.
    /// This can in some cases be positive, and code should not assume it is always negative.
    NegInteger(i64),
    /// A strictly positive integer.
    PosInteger(u64),
    /// A floating point number.
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
    /// Try to create a JsonNumber from a JSON value. This will fail if the value is not a number.
    pub fn try_from(value: &Value, desc: &str, span: &Span) -> Result<Self, TransformError> {
        match value {
            Value::Number(v) => Ok(JsonNumber::from(v)),
            _ => Err(TransformError::new_incorrect_type(
                desc,
                "number",
                TransformError::value_desc(value),
                span,
            )),
        }
    }

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
                    format!("Failed to convert positive integer {x} to signed integer: {e}"),
                    span,
                )
            }),
            Self::NegInteger(x) => Ok(x),
            Self::Float(x) => {
                if x.fract() != 0.0f64 {
                    Err(TransformError::new_conversion_failed(
                        format!("Failed to convert floating point number {x} to integer: not a whole number"),
                        span,
                    ))
                } else if x <= i64::MAX as f64 && x >= i64::MIN as f64 {
                    Ok(x as i64)
                } else {
                    Err(TransformError::new_conversion_failed(
                        format!("Failed to convert floating point number {x} to integer: number does not fit within (-9223372036854775808, 9223372036854775807)"), span))
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

    /// Get the maximum of two numbers. If both numers are integers, the result will be an integer.
    pub fn max(self, other: JsonNumber, span: &Span) -> JsonNumber {
        if self.cmp(Operator::GreaterThan, other, span) {
            self
        } else {
            other
        }
    }

    /// Get the minimum of two numbers. If both numers are integers, the result will be an integer.
    pub fn min(self, other: JsonNumber, span: &Span) -> JsonNumber {
        if self.cmp(Operator::LessThan, other, span) {
            self
        } else {
            other
        }
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

    /// Try to compute the modulus of self by rhs, this will fail if rhs is zero, or if the result cannot be represented as a JsonNumber.
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

    /// Negate the number. This will never fail, but can create a floating point number if
    /// the result is too large to fit in an integer.
    pub fn neg_impl(self) -> JsonNumber {
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

impl Neg for JsonNumber {
    type Output = Self;

    fn neg(self) -> Self::Output {
        self.neg_impl()
    }
}

#[cfg(test)]
mod tests {
    use super::JsonNumber;
    use logos::Span;

    #[test]
    fn test_max_function() {
        let a = JsonNumber::PosInteger(5);
        let b = JsonNumber::PosInteger(10);
        match a.max(b, &Span::default()) {
            JsonNumber::PosInteger(x) => assert_eq!(x, 10),
            _ => panic!("Expected PosInteger"),
        }

        let a = JsonNumber::NegInteger(-5);
        let b = JsonNumber::NegInteger(-10);
        match a.max(b, &Span::default()) {
            JsonNumber::NegInteger(x) => assert_eq!(x, -5),
            _ => panic!("Expected NegInteger"),
        }

        let a = JsonNumber::PosInteger(5);
        let b = JsonNumber::NegInteger(-10);
        match a.max(b, &Span::default()) {
            JsonNumber::PosInteger(x) => assert_eq!(x, 5),
            _ => panic!("Expected PosInteger"),
        }

        let a = JsonNumber::Float(5.0);
        let b = JsonNumber::Float(10.0);
        match a.max(b, &Span::default()) {
            JsonNumber::Float(x) => assert_eq!(x, 10.0),
            _ => panic!("Expected Float"),
        }

        let a = JsonNumber::PosInteger(5);
        let b = JsonNumber::Float(10.0);
        match a.max(b, &Span::default()) {
            JsonNumber::Float(x) => assert_eq!(x, 10.0),
            _ => panic!("Expected Float"),
        }

        let a = JsonNumber::Float(5.0);
        let b = JsonNumber::PosInteger(10);
        match a.max(b, &Span::default()) {
            JsonNumber::PosInteger(x) => assert_eq!(x, 10),
            _ => panic!("Expected PosInteger"),
        }

        let a = JsonNumber::Float(-5.0);
        let b = JsonNumber::PosInteger(10);
        match a.max(b, &Span::default()) {
            JsonNumber::PosInteger(x) => assert_eq!(x, 10),
            _ => panic!("Expected PosInteger"),
        }

        let a = JsonNumber::Float(-5.0);
        let b = JsonNumber::NegInteger(-1);
        match a.max(b, &Span::default()) {
            JsonNumber::NegInteger(x) => assert_eq!(x, -1),
            _ => panic!("Expected NegInteger"),
        }
    }

    #[test]
    fn test_min_function() {
        let a = JsonNumber::PosInteger(5);
        let b = JsonNumber::PosInteger(10);
        match a.min(b, &Span::default()) {
            JsonNumber::PosInteger(x) => assert_eq!(x, 5),
            _ => panic!("Expected PosInteger"),
        }

        let a = JsonNumber::NegInteger(-5);
        let b = JsonNumber::NegInteger(-10);
        match a.min(b, &Span::default()) {
            JsonNumber::NegInteger(x) => assert_eq!(x, -10),
            _ => panic!("Expected NegInteger"),
        }

        let a = JsonNumber::PosInteger(5);
        let b = JsonNumber::NegInteger(-10);
        match a.min(b, &Span::default()) {
            JsonNumber::NegInteger(x) => assert_eq!(x, -10),
            _ => panic!("Expected NegInteger"),
        }

        let a = JsonNumber::Float(5.0);
        let b = JsonNumber::Float(10.0);
        match a.min(b, &Span::default()) {
            JsonNumber::Float(x) => assert_eq!(x, 5.0),
            _ => panic!("Expected Float"),
        }

        let a = JsonNumber::PosInteger(5);
        let b = JsonNumber::Float(10.0);
        match a.min(b, &Span::default()) {
            JsonNumber::PosInteger(x) => assert_eq!(x, 5),
            _ => panic!("Expected PosInteger"),
        }

        let a = JsonNumber::Float(5.0);
        let b = JsonNumber::PosInteger(10);
        match a.min(b, &Span::default()) {
            JsonNumber::Float(x) => assert_eq!(x, 5.0),
            _ => panic!("Expected Float"),
        }

        let a = JsonNumber::Float(-5.0);
        let b = JsonNumber::PosInteger(10);
        match a.min(b, &Span::default()) {
            JsonNumber::Float(x) => assert_eq!(x, -5.0),
            _ => panic!("Expected Float"),
        }

        let a = JsonNumber::Float(-5.0);
        let b = JsonNumber::NegInteger(-1);
        match a.min(b, &Span::default()) {
            JsonNumber::Float(x) => assert_eq!(x, -5.0),
            _ => panic!("Expected Float"),
        }
    }

    #[test]
    fn test_try_as_u64() {
        let n = JsonNumber::NegInteger(10);
        assert_eq!(10u64, n.try_as_u64(&Span::default()).unwrap());

        let n = JsonNumber::PosInteger(10);
        assert_eq!(10u64, n.try_as_u64(&Span::default()).unwrap());

        let n = JsonNumber::Float(10.0);
        assert_eq!(10u64, n.try_as_u64(&Span::default()).unwrap());

        let n = JsonNumber::Float(10.5);
        assert_eq!(
            "Failed to convert floating point number 10.5 to integer: not a whole number at 0..0",
            n.try_as_u64(&Span::default()).unwrap_err().to_string()
        );

        let n = JsonNumber::Float(-5.0);
        assert_eq!(
            "Failed to convert floating point number -5 to positive integer: number does not fit within (0, 18446744073709551615) at 0..0",
            n.try_as_u64(&Span::default()).unwrap_err().to_string()
        );
        let n = JsonNumber::Float(1e20);
        assert_eq!(
            "Failed to convert floating point number 100000000000000000000 to positive integer: number does not fit within (0, 18446744073709551615) at 0..0",
            n.try_as_u64(&Span::default()).unwrap_err().to_string()
        );
        let n = JsonNumber::Float(-1e20);
        assert_eq!(
            "Failed to convert floating point number -100000000000000000000 to positive integer: number does not fit within (0, 18446744073709551615) at 0..0",
            n.try_as_u64(&Span::default()).unwrap_err().to_string()
        );
        let n = JsonNumber::Float(f64::INFINITY);
        assert_eq!(
            "Failed to convert floating point number inf to integer: not a whole number at 0..0",
            n.try_as_u64(&Span::default()).unwrap_err().to_string()
        );

        let n = JsonNumber::NegInteger(-10);
        assert_eq!(
            "Failed to convert negative integer -10 to unsigned: out of range integral type conversion attempted at 0..0",
            n.try_as_u64(&Span::default()).unwrap_err().to_string()
        );
    }

    #[test]
    fn test_try_as_i64() {
        let n = JsonNumber::NegInteger(-10);
        assert_eq!(-10i64, n.try_as_i64(&Span::default()).unwrap());

        let n = JsonNumber::PosInteger(10);
        assert_eq!(10i64, n.try_as_i64(&Span::default()).unwrap());

        let n = JsonNumber::Float(10.0);
        assert_eq!(10i64, n.try_as_i64(&Span::default()).unwrap());

        let n = JsonNumber::PosInteger(u64::MAX);
        assert_eq!(
            "Failed to convert positive integer 18446744073709551615 to signed integer: out of range integral type conversion attempted at 0..0",
            n.try_as_i64(&Span::default()).unwrap_err().to_string()
        );

        let n = JsonNumber::Float(10.5);
        assert_eq!(
            "Failed to convert floating point number 10.5 to integer: not a whole number at 0..0",
            n.try_as_i64(&Span::default()).unwrap_err().to_string()
        );

        let n = JsonNumber::Float(1e20);
        assert_eq!(
            "Failed to convert floating point number 100000000000000000000 to integer: number does not fit within (-9223372036854775808, 9223372036854775807) at 0..0",
            n.try_as_i64(&Span::default()).unwrap_err().to_string()
        );
        let n = JsonNumber::Float(-1e20);
        assert_eq!(
            "Failed to convert floating point number -100000000000000000000 to integer: number does not fit within (-9223372036854775808, 9223372036854775807) at 0..0",
            n.try_as_i64(&Span::default()).unwrap_err().to_string()
        );
        let n = JsonNumber::Float(f64::INFINITY);
        assert_eq!(
            "Failed to convert floating point number inf to integer: not a whole number at 0..0",
            n.try_as_i64(&Span::default()).unwrap_err().to_string()
        );
    }

    #[test]
    fn test_try_cast_integer() {
        assert_eq!(
            JsonNumber::PosInteger(10),
            JsonNumber::PosInteger(10)
                .try_cast_integer(&Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::NegInteger(-10),
            JsonNumber::NegInteger(-10)
                .try_cast_integer(&Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::PosInteger(10),
            JsonNumber::Float(10.5)
                .try_cast_integer(&Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::NegInteger(-10),
            JsonNumber::Float(-10.5)
                .try_cast_integer(&Span::default())
                .unwrap()
        );
        assert_eq!(
            "Failed to convert floating point number 100000000000000000000 to integer, too large. at 0..0",
            JsonNumber::Float(100000000000000000000.0)
                .try_cast_integer(&Span::default())
                .unwrap_err()
                .to_string()
        );
    }

    #[test]
    fn test_try_add() {
        assert_eq!(
            JsonNumber::PosInteger(5),
            JsonNumber::PosInteger(2)
                .try_add(JsonNumber::PosInteger(3), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::NegInteger(-5),
            JsonNumber::NegInteger(-2)
                .try_add(JsonNumber::NegInteger(-3), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::Float(5.5),
            JsonNumber::Float(2.0)
                .try_add(JsonNumber::Float(3.5), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::NegInteger(-1),
            JsonNumber::NegInteger(-2)
                .try_add(JsonNumber::PosInteger(1), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::Float(5.0),
            JsonNumber::Float(2.0)
                .try_add(JsonNumber::PosInteger(3), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::Float(5.0),
            JsonNumber::PosInteger(2)
                .try_add(JsonNumber::Float(3.0), &Span::default())
                .unwrap()
        );

        assert_eq!(
            "Arithmetic overflow at 0..0",
            JsonNumber::PosInteger(u64::MAX - 1)
                .try_add(JsonNumber::PosInteger(2), &Span::default())
                .unwrap_err()
                .to_string()
        );
        assert_eq!(
            "Arithmetic overflow at 0..0",
            JsonNumber::NegInteger(i64::MIN + 1)
                .try_add(JsonNumber::NegInteger(-2), &Span::default())
                .unwrap_err()
                .to_string()
        );
        assert_eq!(
            "Arithmetic overflow at 0..0",
            JsonNumber::NegInteger(i64::MIN + 1)
                .try_add(JsonNumber::PosInteger(u64::MAX), &Span::default())
                .unwrap_err()
                .to_string()
        );
        assert_eq!(
            "Failed to convert positive integer 18446744073709551615 to signed integer: out of range integral type conversion attempted at 0..0",
            JsonNumber::PosInteger(u64::MAX)
                .try_add(JsonNumber::NegInteger(i64::MIN + 1), &Span::default())
                .unwrap_err()
                .to_string()
        );
    }

    #[test]
    fn test_try_sub() {
        assert_eq!(
            JsonNumber::PosInteger(2),
            JsonNumber::PosInteger(5)
                .try_sub(JsonNumber::PosInteger(3), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::NegInteger(-2),
            JsonNumber::NegInteger(-5)
                .try_sub(JsonNumber::NegInteger(-3), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::Float(2.0),
            JsonNumber::Float(5.0)
                .try_sub(JsonNumber::Float(3.0), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::NegInteger(-3),
            JsonNumber::NegInteger(-2)
                .try_sub(JsonNumber::PosInteger(1), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::Float(-1.0),
            JsonNumber::Float(2.0)
                .try_sub(JsonNumber::PosInteger(3), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::Float(-1.0),
            JsonNumber::PosInteger(2)
                .try_sub(JsonNumber::Float(3.0), &Span::default())
                .unwrap()
        );

        assert_eq!(
            "Failed to convert result into negative integer, cannot produce a negative integer smaller than -9223372036854775808 at 0..0",
            JsonNumber::PosInteger(1)
                .try_sub(JsonNumber::PosInteger(u64::MAX), &Span::default())
                .unwrap_err()
                .to_string()
        );
        assert_eq!(
            "Arithmetic overflow at 0..0",
            JsonNumber::NegInteger(i64::MIN + 1)
                .try_sub(JsonNumber::NegInteger(i64::MAX), &Span::default())
                .unwrap_err()
                .to_string()
        );
        assert_eq!(
            "Arithmetic overflow at 0..0",
            JsonNumber::NegInteger(i64::MIN + 1)
                .try_sub(JsonNumber::PosInteger(u64::MAX), &Span::default())
                .unwrap_err()
                .to_string()
        );
        assert_eq!(
            "Failed to convert positive integer 18446744073709551615 to signed integer: out of range integral type conversion attempted at 0..0",
            JsonNumber::PosInteger(u64::MAX)
                .try_sub(JsonNumber::NegInteger(i64::MIN + 1), &Span::default())
                .unwrap_err()
                .to_string()
        );
    }

    #[test]
    fn test_try_mul() {
        assert_eq!(
            JsonNumber::PosInteger(6),
            JsonNumber::PosInteger(2)
                .try_mul(JsonNumber::PosInteger(3), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::NegInteger(6),
            JsonNumber::NegInteger(-2)
                .try_mul(JsonNumber::NegInteger(-3), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::Float(6.0),
            JsonNumber::Float(2.0)
                .try_mul(JsonNumber::Float(3.0), &Span::default())
                .unwrap()
        );
        assert_eq!(
            JsonNumber::NegInteger(-2),
            JsonNumber::NegInteger(-2)
                .try_mul(JsonNumber::PosInteger(1), &Span::default())
                .unwrap()
        );
        assert_eq!(
            "Failed to convert positive integer 18446744073709551615 to signed integer: out of range integral type conversion attempted at 0..0",
            JsonNumber::PosInteger(u64::MAX)
                .try_mul(JsonNumber::NegInteger(i64::MIN + 1), &Span::default())
                .unwrap_err()
                .to_string()
        );
        assert_eq!(
            "Arithmetic overflow at 0..0",
            JsonNumber::NegInteger(i64::MIN + 1)
                .try_mul(JsonNumber::NegInteger(i64::MAX), &Span::default())
                .unwrap_err()
                .to_string()
        );
        assert_eq!(
            "Arithmetic overflow at 0..0",
            JsonNumber::NegInteger(i64::MIN + 1)
                .try_mul(JsonNumber::PosInteger(5), &Span::default())
                .unwrap_err()
                .to_string()
        );
        assert_eq!(
            "Arithmetic overflow at 0..0",
            JsonNumber::PosInteger(u64::MAX - 1)
                .try_mul(JsonNumber::PosInteger(2), &Span::default())
                .unwrap_err()
                .to_string()
        );
    }
}
