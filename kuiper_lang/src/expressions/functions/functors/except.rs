use serde_json::Value;

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    types::{Object, ObjectField, Truthy, Type},
    BuildError, TransformError,
};

function_def!(ExceptFunction, "except", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for ExceptFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;
        let source = source.into_owned();
        match source {
            Value::Object(x) => {
                let mut output = x.to_owned();
                match &*self.args[1] {
                    crate::ExpressionType::Lambda(expr) => {
                        for (k, v) in x {
                            let should_remove = expr
                                .call(state, &[&v, &Value::String(k.to_owned())])?
                                .as_bool();
                            if should_remove {
                                output.remove(&k);
                            }
                        }
                        Ok(ResolveResult::Owned(Value::Object(output)))
                    }
                    expr => {
                        let res = expr.resolve(state)?;
                        match res.as_ref() {
                            Value::Array(arr) => {
                                for f in arr {
                                    match f {
                                        Value::String(s) => {
                                            output.remove(s);
                                            Ok(())
                                        }
                                        x => Err(TransformError::new_incorrect_type(
                                            "Filter values should be of type string",
                                            "string",
                                            TransformError::value_desc(x),
                                            &self.span,
                                        )),
                                    }?
                                }
                                Ok(ResolveResult::Owned(Value::Object(output)))
                            }
                            x => Err(TransformError::new_incorrect_type(
                                "Incorrect input passed as second argument to except",
                                "array, lambda",
                                TransformError::value_desc(x),
                                &self.span,
                            )),
                        }
                    }
                }
            }
            x => Err(TransformError::new_incorrect_type(
                "The first argument to except should be an object",
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
                    let should_remove = lambda.call_types(state, &[&v, &key_arg])?;
                    match should_remove.truthyness() {
                        Truthy::Always => (),
                        Truthy::Maybe => {
                            // Known fields are only for fields we're confident are present,
                            // so maybe needs to be treated as generic.
                            res_obj.push_field(ObjectField::Generic, v);
                        }
                        Truthy::Never => {
                            res_obj.push_field(k, v);
                        }
                    }
                }
                Ok(Type::Object(res_obj))
            }
            expr => {
                let arr = expr.resolve_types(state)?;
                let arr = arr.try_as_array(&self.span)?;
                let mut all_constant = true;
                for elem in arr.all_elements() {
                    if let Type::Constant(Value::String(s)) = elem {
                        item_obj.fields.remove(&ObjectField::Constant(s.clone()));
                    } else {
                        all_constant = false;
                        elem.assert_assignable_to(&Type::String, &self.span)?;
                    }
                }
                if all_constant {
                    Ok(Type::Object(item_obj))
                } else {
                    // Some dynamic keys, so we have to treat the rest as generic.
                    Ok(Type::object_of_type(item_obj.element_union()))
                }
            }
        }
    }
}

impl LambdaAcceptFunction for ExceptFunction {
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
                "except takes a function with a maximum of 2 argument",
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
    fn test_except() {
        let expr =
            compile_expression_test(r#"except({'a': 1, 'b': 2, 'c': 3}, (v) => v > 1)"#, &[])
                .unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();

        assert_eq!(val.len(), 1);
        assert_eq!(val.get("a").unwrap(), 1);
    }

    #[test]
    fn test_except_filter_by_key() {
        let expr = compile_expression_test(
            r#"except({'a': 1, 'b': 2, 'c': 3}, (_, k) => k != 'a')"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();

        assert_eq!(val.len(), 1);
        assert_eq!(val.get("a").unwrap(), 1);
    }

    #[test]
    fn test_except_fails_for_other_types() {
        match crate::compile_expression(r#"except({'a':1}, [1,2,3])"#, &[]) {
            Ok(_) => panic!("Should not be able to resolve"),
            Err(err) => match err {
                CompileError::Optimizer(TransformError::IncorrectTypeInField(t_err)) => {
                    assert_eq!(
                        t_err.desc,
                        "Filter values should be of type string. Got number, expected string"
                    );
                    assert_eq!(t_err.span, Span { start: 0, end: 24 })
                }
                _ => panic!("Should be an optimizer error"),
            },
        }
    }

    #[test]
    fn test_except_lambda_types() {
        let r = compile_expression_test("input.except(a => a is float)", &["input"]).unwrap();
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
    fn test_except_array_types() {
        let r = compile_expression_test("input.except(['k2', 'k3'])", &["input"]).unwrap();
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
                    .with_generic_field(Type::from_const(5))
            )
        );
    }
}
