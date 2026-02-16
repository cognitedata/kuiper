use crate::expressions::functions::LambdaAcceptFunction;
use crate::expressions::{Expression, ExpressionExecutionState, ResolveResult};
use crate::types::Type;
use crate::{BuildError, TransformError};
use serde_json::Value;

function_def!(ReduceFunction, "reduce", 3, lambda);

impl Expression for ReduceFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Array(xs) => {
                let mut value = self.args[2].resolve(state)?.into_owned();
                for x in xs {
                    let res = self.args[1].call(state, &[&value, x])?;
                    value = res.into_owned();
                }
                Ok(ResolveResult::Owned(value))
            }

            non_array => Err(TransformError::new_incorrect_type(
                "Incorrect input to reduce",
                "array",
                TransformError::value_desc(non_array),
                &self.span,
            )),
        }
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let source = self.args[0].resolve_types(state)?;
        let source_arr = source.try_as_array(&self.span)?;

        let mut acc_type = self.args[2].resolve_types(state)?;

        for elem in source_arr.all_elements() {
            let res = self.args[1].call_types(state, &[&acc_type, elem])?;
            acc_type = res;
        }

        // Check if we have converged to a single type, by seeing what happens if we try to pass the final result
        // with end_dynamic to the lambda again. If we haven't, there's some form of dependency on the
        // number of inputs in the array, and we can't be sure of the output type.
        if let Some(end_dynamic) = source_arr.end_dynamic {
            let Ok(res) = self.args[1].call_types(state, &[&acc_type, &end_dynamic]) else {
                // There may only ever be one value of end_dynamic, but that's fine, the type is
                // ok to use.
                return Ok(acc_type);
            };
            // We don't converge, so the return type is indeterminable.
            if res != acc_type {
                return Ok(Type::Any);
            }
        }

        Ok(acc_type)
    }
}

impl LambdaAcceptFunction for ReduceFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
        _num_args: usize,
    ) -> Result<(), BuildError> {
        if idx != 1 {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }
        let nargs = lambda.input_names.len();
        if nargs != 2 {
            return Err(BuildError::n_function_args(
                lambda.span.clone(),
                "reduce takes a function with two arguments",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{compile_expression, types::Type};

    #[test]
    pub fn test_reduce_strings() {
        let expr = compile_expression(r#"['a', 'b', 'c'].reduce((a, b) => concat(a, b), '')"#, &[])
            .unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_str().unwrap();
        assert_eq!(val_arr, "abc");
    }

    #[test]
    pub fn test_reduce_numbers() {
        let expr = compile_expression(r#"[1, 2, 3, 4].reduce((a, b) => a+b, 0)"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_i64().unwrap();
        assert_eq!(val_arr, 10);
    }

    #[test]
    fn test_reduce_types() {
        let expr = compile_expression("input.reduce((acc, v) => acc + v, 0)", &["input"]).unwrap();
        let res = expr
            .run_types([Type::array_of_type(Type::Integer)])
            .unwrap();
        assert_eq!(res, Type::Integer);
        let res = expr.run_types([Type::array_of_type(Type::Float)]).unwrap();
        assert_eq!(res, Type::Float);

        let r = expr.run_types([Type::Any]).unwrap();
        assert_eq!(r, Type::number());
    }
}
