use serde_json::Value;

use crate::{
    expressions::{Expression, ResolveResult},
    types::Type,
    TransformError,
};

function_def!(JoinFunction, "join", 2, None);

impl<'a: 'c, 'c> Expression<'a, 'c> for JoinFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.into_owned() {
            Value::Object(x) => {
                let mut res = x;
                for arg in self.args.iter().skip(1) {
                    let res_inner = arg.resolve(state)?;
                    let mut res_inner = res_inner.into_owned();
                    let value = match res_inner {
                        Value::Object(ref mut inner) => Ok(inner),
                        y => Err(TransformError::new_incorrect_type(
                            "Incorrect type provided for join",
                            "object",
                            TransformError::value_desc(&y),
                            &self.span,
                        )),
                    }?;
                    res.append(value);
                }
                Ok(ResolveResult::Owned(Value::Object(res)))
            }
            Value::Array(x) => {
                let mut res = x;
                for arg in self.args.iter().skip(1) {
                    let res_inner = arg.resolve(state)?;
                    let mut res_inner = res_inner.into_owned();
                    let value = match res_inner {
                        Value::Array(ref mut inner) => Ok(inner),
                        y => Err(TransformError::new_incorrect_type(
                            "Incorrect type provided for join",
                            "array",
                            TransformError::value_desc(&y),
                            &self.span,
                        )),
                    }?;
                    res.append(value);
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            x_val => Err(TransformError::new_incorrect_type(
                "Incorrect input to join",
                "object or array",
                TransformError::value_desc(&x_val),
                &self.span,
            )),
        }
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let source = self.args[0].resolve_types(state)?;
        let mut res_type = Type::never();

        let args = self
            .args
            .iter()
            .skip(1)
            .map(|a| a.resolve_types(state))
            .collect::<Result<Vec<_>, _>>()?;

        // Could this be an object? If so, we merge all the fields together.
        if source.is_assignable_to(&Type::any_object())
            && args.iter().all(|a| a.is_assignable_to(&Type::any_object()))
        {
            let obj = source.try_as_object(&self.span)?;
            let mut res_fields = obj.fields;
            for arg in &args {
                let res_inner_obj = arg.try_as_object(&self.span)?;
                for (k, v) in res_inner_obj.fields {
                    match k {
                        crate::types::ObjectField::Constant(r) => {
                            res_fields.insert(crate::types::ObjectField::Constant(r), v);
                        }
                        crate::types::ObjectField::Generic => {
                            if let Some(old) =
                                res_fields.remove(&crate::types::ObjectField::Generic)
                            {
                                res_fields
                                    .insert(crate::types::ObjectField::Generic, old.union_with(v));
                            } else {
                                res_fields.insert(crate::types::ObjectField::Generic, v);
                            }
                        }
                    }
                }
            }
            res_type =
                res_type.union_with(Type::Object(crate::types::Object { fields: res_fields }));
        }

        // Same for arrays, we merge the element types together.
        if source.is_assignable_to(&Type::any_array())
            && args.iter().all(|a| a.is_assignable_to(&Type::any_array()))
        {
            let arr = source.try_as_array(&self.span)?;
            let mut res_elements = arr.elements;
            let mut res_end_dynamic = arr.end_dynamic.map(|e| *e);
            for arg in &args {
                let res_inner_arr = arg.try_as_array(&self.span)?;
                if let Some(end_dynamic) = res_end_dynamic {
                    let mut dynamic = end_dynamic;
                    for elem in res_inner_arr.elements {
                        dynamic = dynamic.union_with(elem);
                    }
                    if let Some(res_inner_dynamic) = res_inner_arr.end_dynamic {
                        dynamic = dynamic.union_with(*res_inner_dynamic);
                    }
                    res_end_dynamic = Some(dynamic);
                } else {
                    res_elements.extend(res_inner_arr.elements);
                    res_end_dynamic = res_inner_arr.end_dynamic.map(|e| *e);
                }
            }
            res_type = res_type.union_with(Type::Array(crate::types::Array {
                elements: res_elements,
                end_dynamic: res_end_dynamic.map(Box::new),
            }));
        }

        Ok(res_type)
    }
}

#[cfg(test)]
mod tests {
    use logos::Span;

    use crate::{
        compile_expression_test,
        types::{Object, ObjectField, Type},
        CompileError, TransformError,
    };

    #[test]
    fn test_join() {
        let expr = compile_expression_test(r#"join({'a': 1}, {'b': 2})"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();

        assert_eq!(val.len(), 2);
        assert_eq!(val.get("a").unwrap(), 1);
        assert_eq!(val.get("b").unwrap(), 2);
    }

    #[test]
    fn test_join_multiple() {
        let expr = compile_expression_test(r#"join({'a':1}, {'b': 2}, {'c': 3})"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();
        assert_eq!(val.len(), 3);
        assert_eq!(val.get("a").unwrap(), 1);
        assert_eq!(val.get("b").unwrap(), 2);
        assert_eq!(val.get("c").unwrap(), 3);
    }

    #[test]
    fn test_join_overwrites() {
        let expr = compile_expression_test(r#"join({'a':1}, {'a': 2})"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();
        assert_eq!(val.len(), 1);
        assert_eq!(val.get("a").unwrap(), 2);
    }

    #[test]
    fn test_join_fails_for_other_types() {
        match crate::compile_expression(r#"join({'a':1}, [1,2,3])"#, &[]) {
            Ok(_) => panic!("Should not be able to resolve"),
            Err(err) => match err {
                CompileError::Optimizer(TransformError::IncorrectTypeInField(t_err)) => {
                    assert_eq!(
                        t_err.desc,
                        "Incorrect type provided for join. Got array, expected object"
                    );
                    assert_eq!(t_err.span, Span { start: 0, end: 22 })
                }
                _ => panic!("Should be an optimizer error"),
            },
        }
    }

    #[test]
    fn test_join_arrays() {
        let expr = compile_expression_test(r#"join([1, 2, 3], [4, 5], [6, 7, 8])"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_array().unwrap();
        assert_eq!(val.len(), 8);

        for (i, item) in val.iter().enumerate() {
            assert_eq!(item.as_u64().unwrap(), (i + 1) as u64);
        }
    }

    #[test]
    fn test_join_types() {
        let expr =
            compile_expression_test(r#"join(input1, input2)"#, &["input1", "input2"]).unwrap();

        let t = expr
            .run_types([
                Type::Object(Object {
                    fields: vec![(ObjectField::Constant("a".to_string()), Type::Integer)]
                        .into_iter()
                        .collect(),
                }),
                Type::Object(Object {
                    fields: vec![(ObjectField::Constant("b".to_string()), Type::String)]
                        .into_iter()
                        .collect(),
                }),
            ])
            .unwrap();
        let expected = Type::Object(Object {
            fields: vec![
                (ObjectField::Constant("a".to_string()), Type::Integer),
                (ObjectField::Constant("b".to_string()), Type::String),
            ]
            .into_iter()
            .collect(),
        });
        assert_eq!(t, expected);
    }
}
