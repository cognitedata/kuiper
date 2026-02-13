use serde_json::Value;

use crate::{
    compiler::BuildError,
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    types::{Array, Truthy, Type},
    TransformError,
};

function_def!(FilterFunction, "filter", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for FilterFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.into_owned() {
            Value::Array(x) => {
                let mut res = Vec::with_capacity(x.len());
                for item in x {
                    let should_add = self.args[1].call(state, &[&item])?.as_bool();

                    if should_add {
                        res.push(item);
                    }
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to filter",
                "array",
                TransformError::value_desc(&x),
                &self.span,
            )),
        }
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let source = self.args[0].resolve_types(state)?;
        let arr = source.try_as_array(&self.span)?;

        // We can only add elements to the final array if we are sure that every
        // previous element has been added or excluded. If there's uncertainty, we can
        // only add every possible element to the dynamic end of the array.

        let mut end_dynamic = Type::never();
        let mut all_known = true;
        let mut final_elements = Vec::new();
        for item in arr.elements {
            let should_add = self.args[1].call_types(state, &[&item])?.truthyness();
            match should_add {
                Truthy::Never => (),
                Truthy::Always if all_known => {
                    final_elements.push(item);
                }
                _ => {
                    all_known = false;
                    end_dynamic = end_dynamic.union_with(item);
                }
            }
        }
        if let Some(old_end_dynamic) = arr.end_dynamic {
            match self.args[1]
                .call_types(state, &[&*old_end_dynamic])?
                .truthyness()
            {
                Truthy::Never => (),
                _ => {
                    end_dynamic = end_dynamic.union_with(*old_end_dynamic);
                }
            }
        }
        Ok(Type::Array(Array {
            elements: final_elements,
            end_dynamic: if end_dynamic.is_never() {
                None
            } else {
                Some(Box::new(end_dynamic))
            },
        }))
    }
}

impl LambdaAcceptFunction for FilterFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
        _num_args: usize,
    ) -> Result<(), BuildError> {
        if idx != 1 {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }
        let nargs = lambda.input_names.len();
        if nargs != 1 {
            return Err(BuildError::n_function_args(
                lambda.span.clone(),
                "filter takes a function with one argument",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        compile_expression,
        types::{Array, Type},
    };

    #[test]
    pub fn test_simple_filter() {
        let expr = compile_expression("[1, 2, 3, 4, 5, 6].filter((i) => i >= 4)", &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(3, val_arr.len());
        assert_eq!(val_arr.first().unwrap().as_u64().unwrap(), 4);
        assert_eq!(val_arr.get(1).unwrap().as_u64().unwrap(), 5);
        assert_eq!(val_arr.get(2).unwrap().as_u64().unwrap(), 6);
    }

    #[test]
    fn test_filter_types() {
        let expr = compile_expression("input.filter(i => i == 'foo')", &["input"]).unwrap();
        let res = expr
            .run_types([Type::Array(Array {
                elements: vec![Type::String],
                end_dynamic: None,
            })])
            .unwrap();
        assert_eq!(res, Type::array_of_type(Type::String));

        let res = expr
            .run_types([Type::Array(Array {
                elements: vec![
                    Type::from_const("foo"),
                    Type::Integer,
                    Type::from_const("bar"),
                    Type::String,
                ],
                end_dynamic: Some(Box::new(Type::Float)),
            })])
            .unwrap();
        assert_eq!(
            res,
            Type::Array(Array {
                elements: vec![Type::from_const("foo")],
                end_dynamic: Some(Box::new(Type::String))
            })
        );
    }
}
