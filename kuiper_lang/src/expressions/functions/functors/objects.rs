use serde_json::{Map, Value};

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    types::{Object, Type},
    BuildError, TransformError,
};

function_def!(ToObjectFunction, "to_object", 2, Some(3), lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for ToObjectFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        let Value::Array(arr) = source.as_ref() else {
            return Err(TransformError::new_incorrect_type(
                "Incorrect input to to_object",
                "array",
                TransformError::value_desc(source.as_ref()),
                &self.span,
            ));
        };

        let mut res = Map::with_capacity(arr.len());
        for elem in arr {
            let key = self.args[1].call(state, &[elem])?;
            let key = key.try_into_string("to_object", &self.span)?.into_owned();

            let value = if let Some(value_lambda) = self.args.get(2) {
                value_lambda.call(state, &[elem])?.into_owned()
            } else {
                elem.clone()
            };
            res.insert(key, value);
        }

        Ok(ResolveResult::Owned(Value::Object(res)))
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let source = self.args[0].resolve_types(state)?;
        let source_arr = source.try_as_array(&self.span)?;
        let mut res_obj = Object::default();
        for elem in source_arr.all_elements() {
            let key = self.args[1].call_types(state, &[elem])?;
            let value = if let Some(value_lambda) = self.args.get(2) {
                value_lambda.call_types(state, &[elem])?
            } else {
                elem.clone()
            };

            if let Type::Constant(Value::String(s)) = &key {
                res_obj.push_field(crate::types::ObjectField::Constant(s.to_owned()), value);
            } else {
                key.assert_assignable_to(&Type::stringifyable(), &self.span)?;
                res_obj.push_field(crate::types::ObjectField::Generic, value);
            }
        }
        Ok(Type::Object(res_obj))
    }
}

impl LambdaAcceptFunction for ToObjectFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
        _num_args: usize,
    ) -> Result<(), crate::BuildError> {
        if idx == 0 {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }

        if lambda.input_names.len() != 1 {
            return Err(BuildError::n_function_args(
                lambda.span.clone(),
                "to_object takes one or two lambdas with exactly one argument each",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        compile_expression,
        types::{Array, Object, Type},
    };

    #[test]
    pub fn test_to_object_implicit_value() {
        let expr = compile_expression(
            r#"
            [{"test": 1, "key": "v1"}, {"test2": 2, "key": "v2"}]
                .to_object(v => v.key)
        "#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val_obj = res.as_object().unwrap();
        assert_eq!(2, val_obj.len());
        assert_eq!(
            1,
            val_obj["v1"].as_object().unwrap()["test"].as_i64().unwrap()
        );
        assert_eq!(
            2,
            val_obj["v2"].as_object().unwrap()["test2"]
                .as_i64()
                .unwrap()
        );
    }

    #[test]
    pub fn test_to_object() {
        let expr = compile_expression(
            r#"
            [{"test": 1, "key": "v1"}, {"test": 2, "key": "v2"}]
                .to_object(v => v.key, v => v.test)
        "#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val_obj = res.as_object().unwrap();
        assert_eq!(2, val_obj.len());
        assert_eq!(1, val_obj["v1"].as_i64().unwrap());
        assert_eq!(2, val_obj["v2"].as_i64().unwrap());
    }

    #[test]
    fn test_to_object_types() {
        let expr = compile_expression("to_object(input, v => v)", &["input"]).unwrap();
        let res = expr.run_types([Type::array_of_type(Type::String)]).unwrap();
        assert_eq!(res, Type::object_of_type(Type::String));

        let res = expr
            .run_types([Type::Array(Array {
                elements: vec![
                    Type::from_const("foo"),
                    Type::from_const("bar"),
                    Type::String,
                ],
                end_dynamic: Some(Box::new(Type::from_const("baz"))),
            })])
            .unwrap();
        assert_eq!(
            res,
            Type::Object(
                Object::default()
                    .with_field("foo", Type::from_const("foo"))
                    .with_field("bar", Type::from_const("bar"))
                    .with_field("baz", Type::from_const("baz"))
                    .with_generic_field(Type::String)
            )
        );

        assert!(expr
            .run_types([Type::array_of_type(Type::array_of_type(Type::String))])
            .is_err(),);
        assert!(expr.run_types([Type::Integer]).is_err());
    }

    #[test]
    fn test_to_object_types_value() {
        let expr = compile_expression("to_object(input, v => v, v => int(v))", &["input"]).unwrap();
        let res = expr.run_types([Type::array_of_type(Type::String)]).unwrap();
        assert_eq!(res, Type::object_of_type(Type::Integer));
    }
}
