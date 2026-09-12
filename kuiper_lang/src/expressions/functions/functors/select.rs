use serde_json::{Map, Value};

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    types::{Object, ObjectField, Truthy, Type},
    BuildError, TransformError,
};

function_def!(SelectFunction, "select", 2, lambda);

impl Expression for SelectFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<crate::expressions::ResolveResult<'a>, crate::TransformError> {
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
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let item = self.args[0].resolve_types(state)?;
        let mut item_obj = item.try_as_object(&self.span)?;
        match &self.args[1].as_ref() {
            crate::ExpressionType::Lambda(lambda) => {
                let mut res_obj = Object::default();
                for (k, v) in item_obj.fields {
                    let key_arg = match &k {
                        ObjectField::Constant(v) => Type::from_const(v.clone()),
                        ObjectField::Generic => Type::String,
                    };
                    let should_keep = lambda.call_types(state, &[&v, &key_arg])?;
                    match should_keep.truthyness() {
                        Truthy::Always => {
                            res_obj.push_field(k, v);
                        }
                        Truthy::Maybe => {
                            // Known fields are only for fields we're confident are present,
                            // so maybe needs to be treated as generic.
                            res_obj.push_field(ObjectField::Generic, v);
                        }
                        Truthy::Never => (),
                    }
                }
                Ok(Type::Object(res_obj))
            }
            expr => {
                let arr = expr.resolve_types(state)?;
                let arr = arr.try_as_array(&self.span)?;
                let mut all_constant = true;
                let mut res_obj = Object::default();
                for elem in arr.all_elements() {
                    if let Type::Constant(Value::String(s)) = elem {
                        let key = ObjectField::Constant(s.clone());
                        if let Some(field_type) = item_obj.fields.remove(&key) {
                            res_obj.push_field(key, field_type);
                        }
                    } else {
                        all_constant = false;
                        elem.assert_assignable_to(&Type::String, &self.span)?;
                    }
                }
                // If not all fields are constant, we need to add a generic field to account for
                // possible remaining keys.
                if !all_constant {
                    res_obj.push_field(ObjectField::Generic, item_obj.element_union());
                }

                Ok(Type::Object(res_obj))
            }
        }
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

    use crate::{
        compile_expression_test,
        types::{Object, Type},
        CompileError, TransformError,
    };

    #[test]
    fn test_select() {
        let expr =
            compile_expression_test(r#"select({'a': 1, 'b': 2, 'c': 3}, (v) => v > 1)"#, &[])
                .unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();

        assert_eq!(val.len(), 2);
        assert_eq!(val.get("b").unwrap(), 2);
        assert_eq!(val.get("c").unwrap(), 3);
    }

    #[test]
    fn test_select_filter_by_key() {
        let expr = compile_expression_test(
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
        match crate::compile_expression(r#"select({'a':1}, ['a',2,3])"#, &[]) {
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

    #[test]
    fn test_select_lambda_types() {
        let r = compile_expression_test("input.select(a => a is not float)", &["input"]).unwrap();
        let t = r.run_types([Type::object_of_type(Type::Integer)]).unwrap();
        assert_eq!(t, Type::object_of_type(Type::Integer));

        let t = r
            .run_types([Type::Object(
                Object::default()
                    .with_field("k1", Type::String)
                    .with_field("k2", Type::from_const(3))
                    .with_field("k3", Type::from_const(1.5))
                    .with_generic_field(Type::from_const(5)),
            )])
            .unwrap();

        assert_eq!(
            t,
            Type::Object(
                Object::default()
                    .with_field("k1", Type::String)
                    .with_field("k2", Type::from_const(3))
                    .with_generic_field(Type::from_const(5))
            )
        );
    }

    #[test]
    fn test_select_array_types() {
        let r = compile_expression_test("input.select(['k2', 'k3'])", &["input"]).unwrap();
        let t = r
            .run_types([Type::Object(
                Object::default()
                    .with_field("k1", Type::String)
                    .with_field("k2", Type::from_const(3))
                    .with_field("k3", Type::from_const(1.5))
                    .with_generic_field(Type::from_const(5)),
            )])
            .unwrap();

        assert_eq!(
            t,
            Type::Object(
                Object::default()
                    .with_field("k2", Type::from_const(3))
                    .with_field("k3", Type::from_const(1.5))
            )
        );
    }
}
