use crate::expressions::functions::LambdaAcceptFunction;
use crate::expressions::{Expression, ExpressionExecutionState, ResolveResult};
use crate::types::Type;
use crate::{BuildError, TransformError};
use serde_json::Value;

function_def!(ReduceFunction, "reduce", 3, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for ReduceFunction {
    fn resolve(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
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
        let source_seq = source.try_as_array(&self.span)?;

        let mut value = self.args[2].resolve_types(state)?;
        for arg in &source_seq.elements {
            value = self.args[1].call_types(state, &[&value, arg])?;
        }
        // It's possible to create a sequence of types with an indeterminate type,
        // so if the value changes here we just set it to Any.
        if let Some(end_dynamic) = source_seq.end_dynamic {
            let next = self.args[1].call_types(state, &[&value, &end_dynamic])?;
            if value != next {
                value = Type::Any;
            }
        }

        Ok(value)
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
    use crate::compile_expression;

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
}
