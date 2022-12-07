use serde_json::{Number, Value};

use crate::{
    expressions::{base::get_number_from_value, Expression, ResolveResult},
    TransformError,
};

use super::FunctionExpression;

/// Macro that creates a math function of the type `my_float.func(arg)`, which becomes `func(my_float, arg)`
/// in the expression language.
macro_rules! arg2_math_func {
    ($typ:ident, $name:expr, $rname:ident) => {
        function_def!($typ, $name, 2);

        impl<'a> $crate::expressions::base::Expression<'a> for $typ {
            fn resolve(
                &self,
                state: &$crate::expressions::base::ExpressionExecutionState,
            ) -> Result<$crate::expressions::base::ResolveResult<'a>, $crate::expressions::transform_error::TransformError> {
                let lhs = $crate::expressions::base::get_number_from_value(
                    &format!("{} argument 1", <Self as $crate::expressions::functions::FunctionExpression<'a>>::INFO.name),
                    self.args[0].resolve(state)?.as_ref(),
                    &self.span,
                    state.id,
                )?
                .as_f64();
                let rhs = $crate::expressions::base::get_number_from_value(
                    &format!("{} argument 2", <Self as $crate::expressions::functions::FunctionExpression<'a>>::INFO.name),
                    self.args[1].resolve(state)?.as_ref(),
                    &self.span,
                    state.id,
                )?
                .as_f64();

                let res = lhs.$rname(rhs);

                Ok($crate::expressions::base::ResolveResult::Value(
                    serde_json::Value::Number(serde_json::Number::from_f64(res).ok_or_else(
                        || {
                            $crate::expressions::transform_error::TransformError::new_conversion_failed(
                                format!(
                                    "Failed to convert result of operator {} to number at {}",
                                    $name, self.span.start
                                ),
                                &self.span,
                                state.id,
                            )
                        },
                    )?),
                ))
            }
        }
    };
}

/// Macro that creates a math function of the type `my_float.func()`, which becomes `func(my_float)`
/// in the expression language.
macro_rules! arg1_math_func {
    ($typ:ident, $name:expr, $rname:ident) => {
        function_def!($typ, $name, 1);

        impl<'a> $crate::expressions::base::Expression<'a> for $typ {
            fn resolve(
                &self,
                state: &$crate::expressions::base::ExpressionExecutionState,
            ) -> Result<
                $crate::expressions::base::ResolveResult<'a>,
                $crate::expressions::transform_error::TransformError,
            > {
                let arg = $crate::expressions::base::get_number_from_value(
                    <Self as $crate::expressions::functions::FunctionExpression<'a>>::INFO.name,
                    self.args[0].resolve(state)?.as_ref(),
                    &self.span,
                    state.id,
                )?
                .as_f64();

                let res = arg.$rname();

                Ok($crate::expressions::base::ResolveResult::Value(
                    serde_json::Value::Number(serde_json::Number::from_f64(res).ok_or_else(|| {
                        $crate::expressions::transform_error::TransformError::new_conversion_failed(
                            format!(
                                "Failed to convert result of operator {} to number at {}",
                                $name, self.span.start
                            ),
                            &self.span,
                            state.id,
                        )
                    })?),
                ))
            }
        }
    };
}

arg2_math_func!(PowFunction, "pow", powf);
arg2_math_func!(LogFunction, "log", log);
arg2_math_func!(Atan2Function, "atan2", atan2);
arg1_math_func!(FloorFunction, "floor", floor);
arg1_math_func!(CeilFunction, "ceil", ceil);
arg1_math_func!(RoundFunction, "round", round);

function_def!(IntFunction, "int", 1);

// Cast and math functions tend to get a bit involved, the reason is that
// we want to be able to handle fairly large numbers, since those will be involved in timestamps. If we just cast to float, we might not be able to handle
// timestamp - timestamp that well, for example, which is important. So we have to carefully track the type of number, and do conversions where possible.
impl<'a> Expression<'a> for IntFunction {
    fn resolve(
        &'a self,
        state: &'a crate::expressions::ExpressionExecutionState,
    ) -> Result<crate::expressions::ResolveResult<'a>, crate::TransformError> {
        let dat = self.args[0].resolve(state)?;
        let val = dat.as_ref();
        let res = match val {
            Value::Null => {
                return Err(TransformError::new_conversion_failed(
                    "Cannot convert null to integer in int() function".to_string(),
                    &self.span,
                    state.id,
                ))
            }
            Value::Bool(x) => {
                if *x {
                    Value::Number(Number::from(1))
                } else {
                    Value::Number(Number::from(0))
                }
            }
            Value::Number(_) => get_number_from_value(
                <Self as FunctionExpression<'a>>::INFO.name,
                val,
                &self.span,
                state.id,
            )?
            .try_cast_integer(&self.span, state.id)?
            .try_into_json()
            .ok_or_else(|| {
                TransformError::new_conversion_failed(
                    format!(
                        "Failed to convert result of int() to number at {}",
                        self.span.start
                    ),
                    &self.span,
                    state.id,
                )
            })?,
            Value::String(s) => {
                if s.starts_with('-') {
                    let res: i64 = s.parse().map_err(|e| {
                        TransformError::new_conversion_failed(
                            format!("Failed to convert string {} to integer: {}", s, e),
                            &self.span,
                            state.id,
                        )
                    })?;
                    Value::Number(Number::from(res))
                } else {
                    let res: u64 = s.parse().map_err(|e| {
                        TransformError::new_conversion_failed(
                            format!("Failed to convert string {} to integer: {}", s, e),
                            &self.span,
                            state.id,
                        )
                    })?;
                    Value::Number(Number::from(res))
                }
            }
            Value::Array(_) | Value::Object(_) => {
                return Err(TransformError::new_invalid_operation(
                    format!(
                        "Cannot to convert {} to integer",
                        TransformError::value_desc(val)
                    ),
                    &self.span,
                    state.id,
                ))
            }
        };
        Ok(ResolveResult::Value(res))
    }
}

function_def!(FloatFunction, "float", 1);

impl<'a> Expression<'a> for FloatFunction {
    fn resolve(
        &'a self,
        state: &'a crate::expressions::ExpressionExecutionState,
    ) -> Result<crate::expressions::ResolveResult<'a>, crate::TransformError> {
        let dat = self.args[0].resolve(state)?;
        let val = dat.as_ref();
        let res = match val {
            Value::Null => {
                return Err(TransformError::new_conversion_failed(
                    "Cannot convert null to float in float() function".to_string(),
                    &self.span,
                    state.id,
                ))
            }
            Value::Bool(x) => {
                if *x {
                    1.0
                } else {
                    0.0
                }
            }
            Value::Number(_) => get_number_from_value(
                <Self as FunctionExpression<'a>>::INFO.name,
                val,
                &self.span,
                state.id,
            )?
            .as_f64(),
            Value::String(s) => s.parse().map_err(|e| {
                TransformError::new_conversion_failed(
                    format!("Failed to convert string {} to float: {}", s, e),
                    &self.span,
                    state.id,
                )
            })?,
            Value::Array(_) | Value::Object(_) => {
                return Err(TransformError::new_invalid_operation(
                    format!(
                        "Cannot to convert {} to float",
                        TransformError::value_desc(val)
                    ),
                    &self.span,
                    state.id,
                ))
            }
        };
        Ok(ResolveResult::Value(Value::Number(
            Number::from_f64(res).unwrap_or_else(|| Number::from_f64(0.0).unwrap()),
        )))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::Program;

    #[test]
    pub fn test_pow_function() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "pow",
                "inputs": ["input"],
                "transform": {
                    "res": "pow(2, 2)",
                    "res2": "pow($input.val1, $input.val2)"
                },
                "type": "map"
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program
            .execute(&json!({
                "val1": 10,
                "val2": 4
            }))
            .unwrap();
        assert_eq!(res.len(), 1);
        let val = res.get(0).unwrap();
        assert_eq!(4.0, val.get("res").unwrap().as_f64().unwrap());
        assert_eq!(10_000.0, val.get("res2").unwrap().as_f64().unwrap());
    }

    #[test]
    pub fn test_log_function() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "pow",
                "inputs": ["input"],
                "transform": {
                    "res": "log(2, 2)",
                    "res2": "log($input.val1, $input.val2)"
                },
                "type": "map"
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program
            .execute(&json!({
                "val1": 1000,
                "val2": 10
            }))
            .unwrap();
        assert_eq!(res.len(), 1);
        let val = res.get(0).unwrap();
        assert_eq!(1.0, val.get("res").unwrap().as_f64().unwrap());
        // Yes, this does yield 2.9999999999999996, blame computers.
        assert!((3.0 - val.get("res2").unwrap().as_f64().unwrap()).abs() < 0.00000001);
    }

    #[test]
    pub fn test_int_function() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "pow",
                "inputs": ["input"],
                "transform": {
                    "res": "int('123')",
                    "res2": "int('-1234')"
                },
                "type": "map"
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program
            .execute(&json!({
                "val1": 1000,
                "val2": 10
            }))
            .unwrap();
        assert_eq!(res.len(), 1);
        let val = res.get(0).unwrap();
        assert_eq!(123, val.get("res").unwrap().as_u64().unwrap());
        assert_eq!(-1234, val.get("res2").unwrap().as_i64().unwrap());
    }

    #[test]
    pub fn test_float_function() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "pow",
                "inputs": ["input"],
                "transform": {
                    "res": "float('123')",
                    "res2": "float('-1234')",
                    "res3": "float('-1234.123')",
                    "res4": "float('1234.1234')"
                },
                "type": "map"
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program
            .execute(&json!({
                "val1": 1000,
                "val2": 10
            }))
            .unwrap();
        assert_eq!(res.len(), 1);
        let val = res.get(0).unwrap();
        assert_eq!(123.0, val.get("res").unwrap().as_f64().unwrap());
        assert_eq!(-1234.0, val.get("res2").unwrap().as_f64().unwrap());
        assert_eq!(-1234.123, val.get("res3").unwrap().as_f64().unwrap());
        assert_eq!(1234.1234, val.get("res4").unwrap().as_f64().unwrap());
    }
}
