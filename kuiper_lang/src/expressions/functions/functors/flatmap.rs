use serde_json::Value;

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    types::Type,
    BuildError, TransformError,
};

function_def!(FlatMapFunction, "flatmap", 2, lambda);

impl Expression for FlatMapFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<crate::expressions::ResolveResult<'a>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Array(x) => {
                let mut res = crate::Vec::with_capacity(x.len());
                for val in x {
                    let res_inner = self.args[1].call(state, &[val])?.into_owned();
                    match res_inner {
                        Value::Array(y) => {
                            for item in y {
                                res.push(item);
                            }
                        }
                        _x => res.push(_x),
                    };
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to flatmap",
                "array",
                TransformError::value_desc(x),
                &self.span,
            )),
        }
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let source = self.args[0].resolve_types(state)?;
        let arr = source.try_as_array(&self.span)?;

        let mut all_known = true;
        let mut final_elements = crate::Vec::new();

        let mut end_dynamic = Type::never();

        for item in arr.elements {
            let res = self.args[1].call_types(state, &[&item])?;
            if let Ok(r) = res.try_as_array(&self.span) {
                // If this might _not_ be an array, we need to consider the case where
                // a non-array value is returned. Since this is uncertain, we no longer know
                // the sequence of elements with any certainty.
                if !res.is_array() {
                    all_known = false;
                    end_dynamic = end_dynamic.union_with(res.clone().except_array());
                }

                if all_known {
                    final_elements.extend(r.elements);
                } else {
                    end_dynamic = end_dynamic.union_with(r.element_union());
                }
                if let Some(end_dyn) = r.end_dynamic {
                    all_known = false;
                    end_dynamic = end_dynamic.union_with(*end_dyn);
                }
            } else if all_known {
                final_elements.push(res);
            } else {
                end_dynamic = end_dynamic.union_with(res);
            }
        }

        if let Some(arr_end_dynamic) = arr.end_dynamic {
            let res = self.args[1].call_types(state, &[&*arr_end_dynamic])?;
            if let Ok(r) = res.try_as_array(&self.span) {
                end_dynamic = end_dynamic.union_with(r.element_union());
            }
            if !res.is_array() {
                end_dynamic = end_dynamic.union_with(res.except_array());
            }
        }

        Ok(Type::Array(crate::types::Array {
            elements: final_elements,
            end_dynamic: if end_dynamic.is_never() {
                None
            } else {
                Some(alloc::boxed::Box::new(end_dynamic))
            },
        }))
    }
}

impl LambdaAcceptFunction for FlatMapFunction {
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
                "flatmap takes a function with one argument",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{compile_expression, types::Type};

    #[test]
    fn test_flatmap() {
        let expr = compile_expression(r#"flatmap([1,2,3], a => [a + a])"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(val_arr.len(), 3);
        assert_eq!(val_arr.first().unwrap(), 2);
        assert_eq!(val_arr.get(1).unwrap(), 4);
        assert_eq!(val_arr.get(2).unwrap(), 6);
    }

    #[test]
    fn test_flatmap_where_include_single() {
        let expr = compile_expression(r#"flatmap([1,2,3, [4, 5]], a => a)"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(val_arr.len(), 5);
        assert_eq!(val_arr.first().unwrap(), 1);
        assert_eq!(val_arr.get(1).unwrap(), 2);
        assert_eq!(val_arr.get(2).unwrap(), 3);
        assert_eq!(val_arr.get(3).unwrap(), 4);
        assert_eq!(val_arr.get(4).unwrap(), 5);
    }

    #[test]
    fn test_flatmap_types() {
        let expr = compile_expression("flatmap(input, it => it)", &["input"]).unwrap();
        let res = expr
            .run_types([Type::Array(crate::types::Array {
                elements: alloc::vec![
                    Type::Array(crate::types::Array {
                        elements: alloc::vec![Type::String],
                        end_dynamic: None,
                    }),
                    Type::from_const(5),
                    Type::Array(crate::types::Array {
                        elements: alloc::vec![
                            Type::from_const(1),
                            Type::from_const(2),
                            Type::array_of_type(Type::String),
                        ],
                        end_dynamic: None,
                    }),
                    Type::array_of_type(Type::String),
                    Type::from_const(3),
                ],
                end_dynamic: Some(Box::new(Type::array_of_type(Type::Float))),
            })])
            .unwrap();
        assert_eq!(
            res,
            Type::Array(crate::types::Array {
                elements: alloc::vec![
                    Type::String,
                    Type::from_const(5),
                    Type::from_const(1),
                    Type::from_const(2),
                    Type::array_of_type(Type::String),
                ],
                end_dynamic: Some(Box::new(
                    Type::String
                        .union_with(Type::from_const(3))
                        .union_with(Type::Float)
                )),
            })
        );

        let res = expr.run_types([Type::Any]).unwrap();
        assert_eq!(res, Type::array_of_type(Type::Any));
    }
}
