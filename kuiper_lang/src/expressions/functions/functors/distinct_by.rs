use std::collections::HashSet;

use serde_json::{Map, Value};

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    types::{ObjectField, Type},
    BuildError, TransformError,
};

function_def!(DistinctByFunction, "distinct_by", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for DistinctByFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Array(x) => {
                let mut res: Vec<Value> = Vec::new();
                let mut found: HashSet<String> = HashSet::new();
                for val in x {
                    let res_inner = self.args[1].call(state, &[val])?;
                    if found.insert(res_inner.to_string()) {
                        res.push(val.to_owned());
                    }
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            Value::Object(x) => {
                let mut res: Map<String, Value> = Map::new();
                let mut found: HashSet<String> = HashSet::new();
                for (k, v) in x {
                    let res_inner = self.args[1].call(state, &[v, &Value::String(k.to_owned())])?;
                    if found.insert(res_inner.to_string()) {
                        res.insert(k.to_owned(), v.to_owned());
                    }
                }
                Ok(ResolveResult::Owned(Value::Object(res)))
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to distinct_by",
                "array or object",
                TransformError::value_desc(x),
                &self.span,
            )),
        }
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let item = self.args[0].resolve_types(state)?;

        // We don't do a lot of type checking here. Instead, we just check
        // whether the input could be an array or an object, and create types for both cases.
        // In general, we lose information about the structure of objects and arrays here,
        // because we don't have enough information to know whether an item will be kept or not.

        let obj_res = item
            .try_as_object(&self.span)
            .ok()
            .map(|o| self.resolve_types_as_object(state, &o));

        let arr_res = item.try_as_array(&self.span).ok().map(|a| {
            self.resolve_types_as_array(state, &a, obj_res.as_ref().is_some_and(|r| r.is_ok()))
        });

        // If the input can be _both_ an array and an object, we can't fail eagerly on either one, since
        // there may be runtime assumptions that make the expression valid only for one of the types,
        // so we have to try to resolve both, and combine the results.
        // If both fail, we can report the error from one of them.
        // If neither are _possible_ (i.e. both return None), we report an error about the input type.
        match (obj_res, arr_res) {
            (Some(Ok(obj_type)), Some(Ok(arr_type))) => Ok(obj_type.union_with(arr_type)),
            (Some(Ok(ty)), _) | (_, Some(Ok(ty))) => Ok(ty),
            (Some(Err(e)), _) | (_, Some(Err(e))) => Err(e),
            (None, None) => Err(crate::types::TypeError::expected_type(
                Type::any_array().union_with(Type::any_object()),
                item,
                self.span.clone(),
            )),
        }
    }
}

impl LambdaAcceptFunction for DistinctByFunction {
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
                "distict_by takes a function with one argument",
            ));
        }
        Ok(())
    }
}

impl DistinctByFunction {
    fn resolve_types_as_array<'a>(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'a, '_>,
        item_arr: &crate::types::Array,
        allows_object: bool,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        for item in item_arr.all_elements() {
            self.args[1].call_types(
                state,
                &[
                    item,
                    &if allows_object {
                        Type::String
                    } else {
                        Type::null()
                    },
                ],
            )?;
        }
        Ok(Type::array_of_type(item_arr.element_union()))
    }

    fn resolve_types_as_object<'a>(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'a, '_>,
        item_obj: &crate::types::Object,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        for (k, v) in &item_obj.fields {
            let arg = match k {
                ObjectField::Constant(v) => Type::from_const(v.to_owned()),
                ObjectField::Generic => Type::String,
            };
            self.args[1].call_types(state, &[v, &arg])?;
        }
        Ok(Type::object_of_type(item_obj.element_union()))
    }
}

#[cfg(test)]
mod tests {
    use logos::Span;

    use crate::{
        compile_expression_test,
        types::{Array, Object, ObjectField, Type},
        CompileError, TransformError,
    };

    #[test]
    fn test_distinct_by_fails_for_unknown_types() {
        match crate::compile_expression(r#"distinct_by(1234567890, (a) => a)"#, &[]) {
            Ok(_) => panic!("Should not be able to resolve"),
            Err(err) => match err {
                CompileError::Optimizer(TransformError::IncorrectTypeInField(t_err)) => {
                    assert_eq!(
                        t_err.desc,
                        "Incorrect input to distinct_by. Got number, expected array or object"
                    );
                    assert_eq!(t_err.span, Span { start: 0, end: 33 })
                }
                _ => panic!("Should be an optimizer error"),
            },
        }
    }

    #[test]
    fn test_distinct_by_for_arrays() {
        let expr =
            compile_expression_test(r#"distinct_by(["sheep", "apple", "sheep"], a => a)"#, &[])
                .unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(val_arr.len(), 2);
        assert_eq!(val_arr.first().unwrap(), "sheep");
        assert_eq!(val_arr.get(1).unwrap(), "apple");
    }

    #[test]
    fn test_distinct_by_for_objects() {
        let expr = compile_expression_test(
            r#"distinct_by({'x': 'y', 'a': 'b', 'c': 'b'}, (a, b) => a)"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();
        assert_eq!(val.len(), 2);
        assert_eq!(val.get("x").unwrap().to_owned(), "y".to_string());
        assert_eq!(val.get("a").unwrap().to_owned(), "b".to_string());
    }

    #[test]
    fn test_distinct_by_types() {
        let r = compile_expression_test(r#"distinct_by(input, (a) => a)"#, &["input"]).unwrap();
        let t = r
            .run_types([Type::Array(Array {
                elements: vec![Type::Integer, Type::String],
                end_dynamic: Some(Box::new(Type::Boolean)),
            })])
            .unwrap();
        assert_eq!(
            t,
            Type::array_of_type(
                Type::Integer
                    .union_with(Type::String)
                    .union_with(Type::Boolean)
            )
        );

        let t = r
            .run_types([Type::Object(Object {
                fields: [
                    (ObjectField::Constant("k1".to_owned()), Type::Integer),
                    (ObjectField::Constant("k2".to_owned()), Type::String),
                    (ObjectField::Generic, Type::Boolean),
                ]
                .into_iter()
                .collect(),
            })])
            .unwrap();
        assert_eq!(
            t,
            Type::object_of_type(
                Type::Integer
                    .union_with(Type::String)
                    .union_with(Type::Boolean)
            )
        );

        let t = r.run_types([Type::Any]).unwrap();
        assert_eq!(t, Type::any_object().union_with(Type::any_array()));

        assert!(r.run_types([Type::Integer]).is_err());
    }

    #[test]
    fn test_distinct_by_types_inner_error() {
        // If the lambda is called with impossible types we should get an error about that.
        let r = compile_expression_test("distinct_by(input, a => a + 1)", &["input"]).unwrap();
        let err = r
            .run_types([Type::Array(Array {
                elements: vec![Type::String],
                end_dynamic: None,
            })])
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Expected Union<Integer, Float> but got String"
        );
    }
}
