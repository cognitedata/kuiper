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
