use logos::Span;
use serde_json::{Number, Value};

use crate::{
    expressions::{numbers::JsonNumber, Expression, ResolveResult},
    types::{Type, TypeError},
    ExpressionType, TransformError,
};

/// Macro that creates a math function of the type `my_float.func(arg)`, which becomes `func(my_float, arg)`
/// in the expression language.
macro_rules! arg2_math_func {
    ($typ:ident, $name:expr, $rname:ident) => {
        function_def!($typ, $name, 2);

        impl $crate::expressions::base::Expression for $typ {
            fn resolve<'a>(
                &'a self,
                state: &mut $crate::expressions::base::ExpressionExecutionState<'a, '_>,
            ) -> Result<$crate::expressions::ResolveResult<'a>, $crate::expressions::transform_error::TransformError> {
                let lhs = self.args[0].resolve(state)?.try_as_number(
                    &<Self as $crate::expressions::functions::FunctionExpression>::INFO.name,
                    &self.span,
                )?
                .as_f64();
                let rhs = self.args[1].resolve(state)?.try_as_number(
                    &<Self as $crate::expressions::functions::FunctionExpression>::INFO.name,
                    &self.span,
                )?
                .as_f64();

                let res = lhs.$rname(rhs);

                Ok($crate::expressions::ResolveResult::Owned(
                    serde_json::Value::Number(serde_json::Number::from_f64(res).ok_or_else(
                        || {
                            $crate::expressions::transform_error::TransformError::new_conversion_failed(
                                format!(
                                    "Failed to convert result of operator {} to number at {}",
                                    $name, self.span.start
                                ),
                                &self.span,
                            )
                        },
                    )?),
                ))
            }

            fn resolve_types(
                &self,
                state: &mut $crate::types::TypeExecutionState<'_, '_>,
            ) -> Result<$crate::types::Type, $crate::types::TypeError> {
                for arg in &self.args {
                    let arg = arg.resolve_types(state)?;
                    arg.assert_assignable_to(&$crate::types::Type::number(), &self.span)?;
                }

                Ok($crate::types::Type::Float)
            }
        }
    };
}

/// Macro that creates a math function of the type `my_float.func()`, which becomes `func(my_float)`
/// in the expression language.
macro_rules! arg1_math_func {
    ($typ:ident, $name:expr, $rname:ident) => {
        function_def!($typ, $name, 1);

        impl $crate::expressions::base::Expression for $typ {
            fn resolve<'a>(
                &'a self,
                state: &mut $crate::expressions::base::ExpressionExecutionState<'a, '_>,
            ) -> Result<
                $crate::expressions::ResolveResult<'a>,
                $crate::expressions::transform_error::TransformError,
            > {
                let arg = self.args[0].resolve(state)?.try_as_number(
                    <Self as $crate::expressions::functions::FunctionExpression>::INFO.name,
                    &self.span,
                )?
                .as_f64();

                let res = arg.$rname();

                Ok($crate::expressions::ResolveResult::Owned(
                    serde_json::Value::Number(serde_json::Number::from_f64(res).ok_or_else(|| {
                        $crate::expressions::transform_error::TransformError::new_conversion_failed(
                            format!(
                                "Failed to convert result of operator {} to number at {}",
                                $name, self.span.start
                            ),
                            &self.span,
                        )
                    })?),
                ))
            }

            fn resolve_types(
                &self,
                state: &mut $crate::types::TypeExecutionState<'_, '_>,
            ) -> Result<$crate::types::Type, $crate::types::TypeError> {
                let arg = self.args[0].resolve_types(state)?;
                arg.assert_assignable_to(
                    &$crate::types::Type::number(), &self.span
                )?;
                Ok($crate::types::Type::Float)
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
arg1_math_func!(SqrtFunction, "sqrt", sqrt);
arg1_math_func!(ExpFunction, "exp", exp);
arg1_math_func!(SinFunction, "sin", sin);
arg1_math_func!(CosFunction, "cos", cos);
arg1_math_func!(TanFunction, "tan", tan);
arg1_math_func!(AsinFunction, "asin", asin);
arg1_math_func!(AcosFunction, "acos", acos);
arg1_math_func!(AtanFunction, "atan", atan);

function_def!(IntFunction, "int", 1);

// Cast and math functions tend to get a bit involved, the reason is that
// we want to be able to handle fairly large numbers, since those will be involved in timestamps. If we just cast to float, we might not be able to handle
// (timestamp - timestamp) that well, for example, which is important. So we have to carefully track the type of number, and do conversions where possible.
impl Expression for IntFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<crate::expressions::ResolveResult<'a>, crate::TransformError> {
        let dat = self.args[0].resolve(state)?;
        let val = dat.as_ref();
        let res = match val {
            Value::Null => {
                return Err(TransformError::new_conversion_failed(
                    "Cannot convert null to integer in int() function".to_string(),
                    &self.span,
                ))
            }
            Value::Bool(x) => {
                if *x {
                    Value::Number(Number::from(1))
                } else {
                    Value::Number(Number::from(0))
                }
            }
            Value::Number(n) => JsonNumber::from(n)
                .try_cast_integer(&self.span)?
                .try_into_json()
                .ok_or_else(|| {
                    TransformError::new_conversion_failed(
                        format!(
                            "Failed to convert result of int() to number at {}",
                            self.span.start
                        ),
                        &self.span,
                    )
                })?,
            Value::String(s) => {
                if s.starts_with('-') {
                    let res: i64 = s.parse().map_err(|e| {
                        TransformError::new_conversion_failed(
                            format!("Failed to convert string {s} to integer: {e}"),
                            &self.span,
                        )
                    })?;
                    Value::Number(Number::from(res))
                } else {
                    let res: u64 = s.parse().map_err(|e| {
                        TransformError::new_conversion_failed(
                            format!("Failed to convert string {s} to integer: {e}"),
                            &self.span,
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
                ))
            }
        };
        Ok(ResolveResult::Owned(res))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let arg = self.args[0].resolve_types(state)?;
        arg.assert_assignable_to(
            &Type::number()
                .union_with(Type::String)
                .union_with(Type::Boolean),
            &self.span,
        )?;
        Ok(Type::Integer)
    }
}

function_def!(FloatFunction, "float", 1);

impl Expression for FloatFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<crate::expressions::ResolveResult<'a>, crate::TransformError> {
        let dat = self.args[0].resolve(state)?;
        let val = dat.as_ref();
        let res = match val {
            Value::Null => {
                return Err(TransformError::new_conversion_failed(
                    "Cannot convert null to float in float() function".to_string(),
                    &self.span,
                ))
            }
            Value::Bool(x) => {
                if *x {
                    1.0
                } else {
                    0.0
                }
            }
            Value::Number(n) => JsonNumber::from(n).as_f64(),
            Value::String(s) => s.parse().map_err(|e| {
                TransformError::new_conversion_failed(
                    format!("Failed to convert string {s} to float: {e}"),
                    &self.span,
                )
            })?,
            Value::Array(_) | Value::Object(_) => {
                return Err(TransformError::new_invalid_operation(
                    format!(
                        "Cannot to convert {} to float",
                        TransformError::value_desc(val)
                    ),
                    &self.span,
                ))
            }
        };
        Ok(ResolveResult::Owned(Value::Number(
            Number::from_f64(res).unwrap_or_else(|| Number::from_f64(0.0).unwrap()),
        )))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let arg = self.args[0].resolve_types(state)?;
        arg.assert_assignable_to(
            &Type::number()
                .union_with(Type::String)
                .union_with(Type::Boolean),
            &self.span,
        )?;
        Ok(Type::Float)
    }
}

fn flatten_args<'a>(
    args: &'a Vec<ResolveResult<'a>>,
    desc: &'a str,
    span: &'a Span,
) -> Box<dyn Iterator<Item = Result<JsonNumber, TransformError>> + 'a> {
    match args.len() {
        0 => Box::new(std::iter::once(Err(TransformError::new_invalid_operation(
            format!("{desc} function requires at least one argument"),
            span,
        )))),
        1 => {
            if let Some(array) = args[0].as_array() {
                if array.is_empty() {
                    Box::new(std::iter::once(Err(TransformError::new_invalid_operation(
                        format!("{desc} of an empty array is undefined"),
                        span,
                    ))))
                } else {
                    Box::new(array.iter().map(|x| JsonNumber::try_from(x, desc, span)))
                }
            } else {
                Box::new(std::iter::once(Err(TransformError::new_invalid_operation(
                        format!("If only one argument is supplied to the {desc} function, it must be an array of numbers"),
                        span,
                    ))))
            }
        }
        _ => Box::new(args.iter().map(|x| x.try_as_number(desc, span))),
    }
}

function_def!(MaxFunction, "max", 1, None);

impl Expression for MaxFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let args = self
            .args
            .iter()
            .map(|x| x.resolve(state))
            .collect::<Result<Vec<_>, _>>()?;

        // Max either accepts many numbers as distinct args, or a single arg which is an array of numbers.
        // Flatten it to a single iterator of numbers.
        let mut items = flatten_args(&args, "max", &self.span);

        // Get the first item as the initial max. The unwrap is safe, since we know the iterator from flatten_args is not empty.
        let first = items.next().unwrap()?;
        let mut max: JsonNumber = first;

        for item in items {
            max = max.max(item?, &self.span);
        }

        Ok(ResolveResult::Owned(max.try_into_json().ok_or_else(
            || {
                TransformError::new_conversion_failed(
                    "Failed to convert result of max to a number",
                    &self.span,
                )
            },
        )?))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let args = flatten_type_args(&self.args, state, &self.span)?;

        for arg in args {
            arg.assert_assignable_to(&Type::number(), &self.span)?;
        }

        Ok(Type::number())
    }
}

fn flatten_type_args(
    args: &[ExpressionType],
    state: &mut crate::types::TypeExecutionState<'_, '_>,
    span: &Span,
) -> Result<Vec<Type>, TypeError> {
    if args.len() == 1 {
        let ty = args[0].resolve_types(state)?;
        let arr = ty.try_as_array(span)?;
        return Ok(arr.all_elements().cloned().collect());
    }
    args.iter().map(|x| x.resolve_types(state)).collect()
}

function_def!(MinFunction, "min", 1, None);

impl Expression for MinFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let args = self
            .args
            .iter()
            .map(|x| x.resolve(state))
            .collect::<Result<Vec<_>, _>>()?;

        // Min either accepts many numbers as distinct args, or a single arg which is an array of numbers.
        // Flatten it to a single iterator of numbers.
        let mut items = flatten_args(&args, "min", &self.span);

        // Get the first item as the initial min. The unwrap is safe, since we know the iterator from flatten_args is not empty.
        let first = items.next().unwrap()?;
        let mut min: JsonNumber = first;

        for item in items {
            min = min.min(item?, &self.span);
        }

        Ok(ResolveResult::Owned(min.try_into_json().ok_or_else(
            || {
                TransformError::new_conversion_failed(
                    "Failed to convert result of min to a number",
                    &self.span,
                )
            },
        )?))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let args = flatten_type_args(&self.args, state, &self.span)?;

        for arg in args {
            arg.assert_assignable_to(&Type::number(), &self.span)?;
        }

        Ok(Type::number())
    }
}

function_def!(RandomFunction, "random", 0);

impl Expression for RandomFunction {
    fn is_deterministic(&self) -> bool {
        false
    }

    fn resolve<'a>(
        &'a self,
        _state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<crate::expressions::ResolveResult<'a>, crate::TransformError> {
        let res: f64 = rand::random();
        Ok(ResolveResult::Owned(Value::Number(
            Number::from_f64(res).ok_or_else(|| {
                TransformError::new_conversion_failed(
                    "Failed to convert random result to number",
                    &self.span,
                )
            })?,
        )))
    }

    fn resolve_types(
        &self,
        _state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        Ok(Type::Float)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        compile_expression_test,
        types::{Array, Type},
    };

    const TINY: f64 = 0.00000001;

    #[test]
    pub fn test_max_function() {
        let expr = compile_expression_test(
            r#"{
            "res": max(2, 2),  // Simple base case
            "res2": max(input.val1, input.val2),  // Max of input (so computation happens at runtime)
            "res3": max(2, 3, 4, 5, 6, 7), // Max of many numbers
            "res4": max([1, 2, 3, 4, 5, 6]),  // Max of list
            "res5": max(7, 3, 5, 8.1, 4),  // Max of mixed floats and ints should be float
            "res6": max(-4, -10)  // Max of negative numbers
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({
            "val1": 10,
            "val2": 4
        });
        let res = expr.run([&inp]).unwrap();

        assert_eq!(2.0, res.get("res").unwrap().as_f64().unwrap());
        assert_eq!(10.0, res.get("res2").unwrap().as_f64().unwrap());
        assert_eq!(7, res.get("res3").unwrap().as_u64().unwrap());
        assert_eq!(6, res.get("res4").unwrap().as_u64().unwrap());
        assert_eq!(8.1, res.get("res5").unwrap().as_f64().unwrap());
        assert_eq!(-4, res.get("res6").unwrap().as_i64().unwrap());

        // Make sure the types of the max are correct:
        //  - If everything is postitive and integer, return an u64
        //  - If something is negative and everything is integer, return an i64
        //  - If something is float, return an f64
        assert!(res.get("res4").unwrap().is_u64());
        assert!(res.get("res5").unwrap().is_f64());
        assert!(res.get("res6").unwrap().is_i64());

        let no_args = compile_expression_test(
            r#"{
            "res": max() // No args should yield an error
        }"#,
            &[],
        );
        assert!(no_args.is_err());

        let empty_list = compile_expression_test(
            r#"{
            "res": max([]) // Empty list should yield an error
        }"#,
            &[],
        );
        assert!(empty_list.is_err());

        let non_list = compile_expression_test(
            r#"{
            "res": max(1) // If the first arg isn't a list it should yield an error
        }"#,
            &[],
        );
        assert!(non_list.is_err());

        let non_number = compile_expression_test(
            r#"{
            "res": max([1, 2, 3, 'a']) // If the list contains a non-number it should yield an error
        }"#,
            &[],
        );
        assert!(non_number.is_err());
    }

    #[test]
    pub fn test_min_function() {
        let expr = compile_expression_test(
            r#"{
            "res": min(2, 2),  // Simple base case
            "res2": min(input.val1, input.val2),  // Min of input (so computation happens at runtime)
            "res3": min(2, 3, 4, 5, 6, 7), // Min of many numbers
            "res4": min([1, 2, 3, 4, 5, 6]),  // Min of list
            "res5": min(7, 3.1, 5, 8, 4),  // Min of mixed floats and ints should be float
            "res6": min(-4, -10)  // Min of negative numbers
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({
            "val1": 10,
            "val2": 4
        });
        let res = expr.run([&inp]).unwrap();

        assert_eq!(2.0, res.get("res").unwrap().as_f64().unwrap());
        assert_eq!(4.0, res.get("res2").unwrap().as_f64().unwrap());
        assert_eq!(2, res.get("res3").unwrap().as_u64().unwrap());
        assert_eq!(1, res.get("res4").unwrap().as_u64().unwrap());
        assert_eq!(3.1, res.get("res5").unwrap().as_f64().unwrap());
        assert_eq!(-10, res.get("res6").unwrap().as_i64().unwrap());

        // Make sure the types of the min are correct:
        //  - If everything is postitive and integer, return an u64
        //  - If something is negative and everything is integer, return an i64
        //  - If something is float, return an f64
        assert!(res.get("res4").unwrap().is_u64());
        assert!(res.get("res5").unwrap().is_f64());
        assert!(res.get("res6").unwrap().is_i64());

        let no_args = compile_expression_test(
            r#"{
            "res": min() // No args should yield an error
        }"#,
            &[],
        );
        assert!(no_args.is_err());

        let empty_list = compile_expression_test(
            r#"{
            "res": min([]) // Empty list should yield an error
        }"#,
            &[],
        );
        assert!(empty_list.is_err());

        let non_list = compile_expression_test(
            r#"{
            "res": min(1) // If the first arg isn't a list it should yield an error
        }"#,
            &[],
        );
        assert!(non_list.is_err());

        let non_number = compile_expression_test(
            r#"{
            "res": min([1, 2, 3, 'a']) // If the list contains a non-number it should yield an error
        }"#,
            &[],
        );
        assert!(non_number.is_err());
    }

    #[test]
    pub fn test_pow_function() {
        let expr = compile_expression_test(
            r#"{
            "res": pow(2, 2),
            "res2": pow(input.val1, input.val2)
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({
            "val1": 10,
            "val2": 4
        });
        let res = expr.run([&inp]).unwrap();
        assert_eq!(4.0, res.get("res").unwrap().as_f64().unwrap());
        assert_eq!(10_000.0, res.get("res2").unwrap().as_f64().unwrap());
    }

    #[test]
    pub fn test_log_function() {
        let expr = compile_expression_test(
            r#"{
            "res": log(2, 2),
            "res2": log(input.val1, input.val2)
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({
            "val1": 1000,
            "val2": 10
        });
        let res = expr.run([&inp]).unwrap();
        assert_eq!(1.0, res.get("res").unwrap().as_f64().unwrap());
        // Yes, this does yield 2.9999999999999996, blame computers.
        assert!((3.0 - res.get("res2").unwrap().as_f64().unwrap()).abs() < TINY);
    }

    #[test]
    pub fn test_int_function() {
        let expr = compile_expression_test(
            r#"{
            "res": int('123'),
            "res2": int('-1234')
        }"#,
            &[],
        )
        .unwrap();

        let inp = json!({
            "val1": 1000,
            "val2": 10
        });
        let res = expr.run([&inp]).unwrap();
        assert_eq!(123, res.get("res").unwrap().as_u64().unwrap());
        assert_eq!(-1234, res.get("res2").unwrap().as_i64().unwrap());
    }

    #[test]
    pub fn test_float_function() {
        let expr = compile_expression_test(
            r#"{
            "res": float('123'),
            "res2": float('-1234'),
            "res3": float('-1234.123'),
            "res4": float('1234.1234')
        }"#,
            &[],
        )
        .unwrap();

        let inp = json!({
            "val1": 1000,
            "val2": 10
        });
        let res = expr.run([&inp]).unwrap();
        assert_eq!(123.0, res.get("res").unwrap().as_f64().unwrap());
        assert_eq!(-1234.0, res.get("res2").unwrap().as_f64().unwrap());
        assert_eq!(-1234.123, res.get("res3").unwrap().as_f64().unwrap());
        assert_eq!(1234.1234, res.get("res4").unwrap().as_f64().unwrap());
    }

    #[test]
    pub fn test_sqrt_function() {
        let expr = compile_expression_test(
            r#"{
            "res": sqrt(4),
            "res2": sqrt(input.val1)
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({
            "val1": 100
        });
        let res = expr.run([&inp]).unwrap();
        assert!((2.0 - res.get("res").unwrap().as_f64().unwrap()).abs() < TINY);
        assert!((10.0 - res.get("res2").unwrap().as_f64().unwrap()).abs() < TINY);

        assert!(compile_expression_test(r#"{"res": sqrt(-1)}"#, &[],).is_err());
        // sqrt(-1) is undefined, should yield an error
    }

    #[test]
    pub fn test_exp_function() {
        let expr = compile_expression_test(
            r#"{
            "res": exp(1),
            "res2": exp(input.val1)
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({
            "val1": 2
        });
        let res = expr.run([&inp]).unwrap();
        assert!((std::f64::consts::E - res.get("res").unwrap().as_f64().unwrap()).abs() < TINY);
        assert!(
            (std::f64::consts::E.powi(2) - res.get("res2").unwrap().as_f64().unwrap()).abs() < TINY
        );
    }

    #[test]
    pub fn test_trig_functions() {
        let expr = compile_expression_test(
            r#"{
            "res0": sin(0),
            "res1": sin(input),
            "res2": cos(0),
            "res3": cos(input),
            "res4": tan(0),
            "res5": asin(0),
            "res6": asin(input),
            "res7": acos(0),
            "res8": acos(input),
            "res9": atan(0),
            "res10": atan(input)
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!(0.5);
        let res = expr.run([&inp]).unwrap();
        assert!((0.0 - res.get("res0").unwrap().as_f64().unwrap()).abs() < TINY);
        assert!((0.479425538604203 - res.get("res1").unwrap().as_f64().unwrap()).abs() < TINY);
        assert!((1.0 - res.get("res2").unwrap().as_f64().unwrap()).abs() < TINY);
        assert!((0.8775825618903726 - res.get("res3").unwrap().as_f64().unwrap()).abs() < TINY);
        assert!((0.0 - res.get("res4").unwrap().as_f64().unwrap()).abs() < TINY);
        assert!((0.0 - res.get("res5").unwrap().as_f64().unwrap()).abs() < TINY);
        assert!((0.5235987755982989 - res.get("res6").unwrap().as_f64().unwrap()).abs() < TINY);
        assert!((1.5707963267948966 - res.get("res7").unwrap().as_f64().unwrap()).abs() < TINY);
        assert!((1.0471975511965979 - res.get("res8").unwrap().as_f64().unwrap()).abs() < TINY);
        assert!((0.0 - res.get("res9").unwrap().as_f64().unwrap()).abs() < TINY);
        assert!((0.4636476090008061 - res.get("res10").unwrap().as_f64().unwrap()).abs() < TINY);

        assert!(compile_expression_test(r#"{"res": asin(2)}"#, &[],).is_err()); // asin(2) is undefined, should yield an error
        assert!(compile_expression_test(r#"{"res": acos(2)}"#, &[],).is_err()); // acos(2) is undefined, should yield an error
    }

    #[test]
    fn test_one_arg_math_function_types() {
        for func in [
            "floor", "ceil", "round", "sqrt", "exp", "sin", "cos", "tan", "asin", "acos", "atan",
        ] {
            let expr = compile_expression_test(&format!("{}(input)", func), &["input"]).unwrap();

            // The argument can be either an integer or a float, and the result should be a float.
            let ty = expr.run_types([Type::Integer]).unwrap();
            assert_eq!(Type::Float, ty);

            let ty = expr.run_types([Type::Float]).unwrap();
            assert_eq!(Type::Float, ty);

            let ty = expr.run_types([Type::Any]).unwrap();
            assert_eq!(Type::Float, ty);

            assert!(expr.run_types([Type::String]).is_err());
        }
    }

    #[test]
    fn test_two_arg_math_function_types() {
        for func in ["pow", "log", "atan2"] {
            let expr = compile_expression_test(
                &format!("{}(input1, input2)", func),
                &["input1", "input2"],
            )
            .unwrap();

            // The arguments can be either integers or floats, and the result should be a float.
            let ty = expr.run_types([Type::Integer, Type::Integer]).unwrap();
            assert_eq!(Type::Float, ty);

            let ty = expr.run_types([Type::Float, Type::Float]).unwrap();
            assert_eq!(Type::Float, ty);

            let ty = expr.run_types([Type::Integer, Type::Float]).unwrap();
            assert_eq!(Type::Float, ty);

            let ty = expr.run_types([Type::Float, Type::Integer]).unwrap();
            assert_eq!(Type::Float, ty);

            let ty = expr.run_types([Type::Any, Type::Any]).unwrap();
            assert_eq!(Type::Float, ty);

            assert!(expr.run_types([Type::String, Type::Float]).is_err());
            assert!(expr.run_types([Type::Float, Type::String]).is_err());
        }
    }

    #[test]
    fn test_int_function_types() {
        let expr = compile_expression_test("int(input)", &["input"]).unwrap();

        // The argument can be a number, string, or boolean, and the result should be an integer.
        let ty = expr.run_types([Type::Integer]).unwrap();
        assert_eq!(Type::Integer, ty);

        let ty = expr.run_types([Type::Float]).unwrap();
        assert_eq!(Type::Integer, ty);

        let ty = expr.run_types([Type::String]).unwrap();
        assert_eq!(Type::Integer, ty);

        let ty = expr.run_types([Type::Boolean]).unwrap();
        assert_eq!(Type::Integer, ty);

        let ty = expr.run_types([Type::Any]).unwrap();
        assert_eq!(Type::Integer, ty);

        assert!(expr.run_types([Type::null()]).is_err());
    }

    #[test]
    fn test_float_function_types() {
        let expr = compile_expression_test("float(input)", &["input"]).unwrap();

        // The argument can be a number, string, or boolean, and the result should be a float.
        let ty = expr.run_types([Type::Integer]).unwrap();
        assert_eq!(Type::Float, ty);

        let ty = expr.run_types([Type::Float]).unwrap();
        assert_eq!(Type::Float, ty);

        let ty = expr.run_types([Type::String]).unwrap();
        assert_eq!(Type::Float, ty);

        let ty = expr.run_types([Type::Boolean]).unwrap();
        assert_eq!(Type::Float, ty);

        let ty = expr.run_types([Type::Any]).unwrap();
        assert_eq!(Type::Float, ty);

        assert!(expr.run_types([Type::null()]).is_err());
    }

    #[test]
    fn test_min_max_function_types() {
        for func in ["min", "max"] {
            let expr = compile_expression_test(&format!("{}(input)", func), &["input"]).unwrap();

            // The argument can be either an array of numbers, or many numbers as distinct args. The result should be a number.
            let ty = expr
                .run_types([Type::Array(Array {
                    elements: vec![Type::number(), Type::number(), Type::number()],
                    end_dynamic: Some(Box::new(Type::number())),
                })])
                .unwrap();
            assert_eq!(Type::number(), ty);

            let ty = expr.run_types([Type::Any]).unwrap();
            assert_eq!(Type::number(), ty);

            assert!(expr
                .run_types([Type::Array(Array {
                    elements: vec![Type::number(), Type::number()],
                    end_dynamic: Some(Box::new(Type::String)),
                })])
                .is_err());

            let expr = compile_expression_test(
                &format!("{}(input1, input2)", func),
                &["input1", "input2"],
            )
            .unwrap();

            let ty = expr.run_types([Type::number(), Type::number()]).unwrap();
            assert_eq!(Type::number(), ty);

            let ty = expr.run_types([Type::Any, Type::Any]).unwrap();
            assert_eq!(Type::number(), ty);

            assert!(expr.run_types([Type::String, Type::number()]).is_err());
        }
    }

    #[test]
    fn test_random() {
        let expr = compile_expression("random()", &[]).unwrap();
        for _ in 0..10 {
            let result = expr.run(std::iter::empty::<&serde_json::Value>()).unwrap();
            let val = result.as_f64().expect("random() should return a float");
            assert!(
                val >= 0.0 && val < 1.0,
                "random() returned {val}, expected [0.0, 1.0)"
            );
        }
    }

    #[test]
    fn test_random_function_types() {
        let expr = compile_expression("random()", &[]).unwrap();
        let ty = expr.run_types(std::iter::empty::<Type>()).unwrap();
        assert_eq!(Type::Float, ty);
    }
}
