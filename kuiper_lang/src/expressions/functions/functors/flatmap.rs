use serde_json::Value;

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    types::Type,
    BuildError, TransformError,
};

function_def!(FlatMapFunction, "flatmap", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for FlatMapFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Array(x) => {
                let mut res = Vec::with_capacity(x.len());
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
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let input = self.args[0].resolve_types(state)?;
        let input_seq = input.try_as_array(&self.span)?;
        let mut out = Vec::with_capacity(input_seq.elements.len());
        let mut end_dynamic: Option<Type> = None;
        for arg in input_seq.elements {
            let res = self.args[1].call_types(state, &[&arg])?;
            if let Ok(res_seq) = res.try_as_array(&self.span) {
                if let Some(dynamic) = end_dynamic {
                    let mut dyn_res = dynamic;
                    if let Some(dy) = res_seq.end_dynamic {
                        dyn_res = dyn_res.union_with(*dy);
                    }
                    for ty in res_seq.elements {
                        dyn_res = dyn_res.union_with(ty);
                    }
                    end_dynamic = Some(dyn_res);
                } else {
                    out.extend(res_seq.elements);
                }
            } else {
                if let Some(dynamic) = end_dynamic {
                    end_dynamic = Some(dynamic.union_with(arg));
                } else {
                    out.push(arg);
                }
            }
        }
        if let Some(ty) = input_seq.end_dynamic {
            let res = self.args[1].call_types(state, &[&*ty])?;
            let mut l_dynamic = if let Some(dynamic) = end_dynamic {
                dynamic
            } else {
                Type::never()
            };

            if let Ok(res_seq) = res.try_as_array(&self.span) {
                for ty in res_seq.elements {
                    l_dynamic = l_dynamic.union_with(ty);
                }
                if let Some(dy) = res_seq.end_dynamic {
                    l_dynamic = l_dynamic.union_with(*dy);
                }
            } else {
                l_dynamic = l_dynamic.union_with(res);
            }
            if !l_dynamic.is_never() {
                end_dynamic = Some(l_dynamic);
            } else {
                end_dynamic = None;
            }
        }

        Ok(crate::types::Type::Sequence(crate::types::Sequence {
            elements: out,
            end_dynamic: end_dynamic.map(Box::new),
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
    use crate::compile_expression;

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
}
