use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    types::{Object, ObjectField, Type},
    BuildError, TransformError,
};

function_def!(SelectFunction, "select", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for SelectFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;
        match source.into_owned() {
            Value::Object(x) => {
                let mut output = Map::new();
                match &*self.args[1] {
                    crate::ExpressionType::Lambda(expr) => {
                        for (k, v) in x {
                            let should_add = expr
                                .call(state, &[&v, &Value::String(k.to_owned())])?
                                .as_bool();
                            if should_add {
                                output.insert(k, v);
                            }
                        }
                        Ok(ResolveResult::Owned(Value::Object(output)))
                    }
                    expr => {
                        let res = expr.resolve(state)?;
                        match res.into_owned() {
                            Value::Array(arr) => {
                                for f in arr {
                                    let (should_add, k, v) = match f {
                                        Value::String(k) => match x.get(&k) {
                                            Some(val) => Ok((true, k, val.to_owned())),
                                            None => Ok((false, k, Value::Null)),
                                        },
                                        x => Err(TransformError::new_incorrect_type(
                                            "Filter values should be of type string",
                                            "string",
                                            TransformError::value_desc(&x),
                                            &self.span,
                                        )),
                                    }?;
                                    if should_add {
                                        output.insert(k, v);
                                    }
                                }
                                Ok(ResolveResult::Owned(Value::Object(output)))
                            }
                            x => Err(TransformError::new_incorrect_type(
                                "Incorrect input passed as second argument to except",
                                "array, lambda",
                                TransformError::value_desc(&x),
                                &self.span,
                            )),
                        }
                    }
                }
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input passed as first argument to except",
                "object",
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
        let mut source_obj = source.try_as_object(&self.span)?;

        let mut res_fields = HashMap::<ObjectField, Type>::with_capacity(source_obj.fields.len());
        match &*self.args[1] {
            crate::ExpressionType::Lambda(expr) => {
                for (k, v) in source_obj.fields {
                    let key = match k.clone() {
                        ObjectField::Constant(r) => Type::from_const(Value::String(r)),
                        ObjectField::Generic => Type::String,
                    };
                    let should_add = expr.call_types(state, &[&v, &key])?;
                    match should_add.truthyness() {
                        crate::types::Truthy::Always => {
                            if let Some(old) = res_fields.remove(&k) {
                                res_fields.insert(k, old.union_with(v));
                            } else {
                                res_fields.insert(k, v);
                            }
                        }
                        crate::types::Truthy::Never => {}
                        crate::types::Truthy::Maybe => {
                            if let Some(old) = res_fields.remove(&k) {
                                res_fields.insert(k, old.union_with(v).union_with(Type::null()));
                            } else {
                                res_fields.insert(k, v.union_with(Type::null()));
                            }
                        }
                    }
                }
            }
            expr => {
                let filter = expr.resolve_types(state)?;
                let filter_arr = filter.try_as_array(&self.span)?;
                let mut is_checking = true;
                for elem in filter_arr
                    .elements
                    .iter()
                    .chain(filter_arr.end_dynamic.iter().map(|r| r.as_ref()))
                {
                    elem.assert_assignable_to(&Type::String, &self.span)?;
                    if !is_checking {
                        continue;
                    }
                    if let Type::Constant(Value::String(k)) = elem {
                        if let Some(v) = source_obj.fields.remove(&ObjectField::Constant(k.clone()))
                        {
                            res_fields.insert(ObjectField::Constant(k.clone()), v.clone());
                        }
                    } else {
                        if !source_obj.fields.is_empty() {
                            res_fields.insert(ObjectField::Generic, source_obj.element_union());
                        }
                        is_checking = false;
                        continue;
                    }
                }
            }
        }

        Ok(Type::Object(Object { fields: res_fields }))
    }
}

impl LambdaAcceptFunction for SelectFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
        _num_args: usize,
    ) -> Result<(), BuildError> {
        if idx != 1 {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }
        let nargs = lambda.input_names.len();
        if !(1..=2).contains(&nargs) {
            return Err(BuildError::n_function_args(
                lambda.span.clone(),
                "select takes a function with 1 or 2 arguments",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use logos::Span;

    use crate::{compile_expression, CompileError, TransformError};

    #[test]
    fn test_select() {
        let expr =
            compile_expression(r#"select({'a': 1, 'b': 2, 'c': 3}, (v) => v > 1)"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();

        assert_eq!(val.len(), 2);
        assert_eq!(val.get("b").unwrap(), 2);
        assert_eq!(val.get("c").unwrap(), 3);
    }

    #[test]
    fn test_select_filter_by_key() {
        let expr = compile_expression(
            r#"select({'a': 1, 'b': 2, 'c': 3}, (_, k) => k != 'a')"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();

        assert_eq!(val.len(), 2);
        assert_eq!(val.get("b").unwrap(), 2);
        assert_eq!(val.get("c").unwrap(), 3);
    }

    #[test]
    fn test_select_fails_for_other_types() {
        match compile_expression(r#"select({'a':1}, ['a',2,3])"#, &[]) {
            Ok(_) => panic!("Should not be able to resolve"),
            Err(err) => match err {
                CompileError::Optimizer(TransformError::IncorrectTypeInField(t_err)) => {
                    assert_eq!(
                        t_err.desc,
                        "Filter values should be of type string. Got number, expected string"
                    );
                    assert_eq!(t_err.span, Span { start: 0, end: 26 })
                }
                _ => panic!("Should be an optimizer error"),
            },
        }
    }
}
