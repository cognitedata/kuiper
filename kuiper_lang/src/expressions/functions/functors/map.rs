use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::{
    compiler::BuildError,
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    types::{Array, Object, ObjectField, Type},
    TransformError,
};

function_def!(MapFunction, "map", 2, lambda);

impl Expression for MapFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<crate::expressions::ResolveResult<'a>, TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Array(x) => {
                let mut res = Vec::with_capacity(x.len());
                for (idx, val) in x.iter().enumerate() {
                    res.push(
                        self.args[1]
                            .call(state, &[val, &Value::Number(idx.into())])?
                            .into_owned(),
                    );
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            Value::Object(x) => {
                let mut res = Map::with_capacity(x.len());
                for (k, v) in x {
                    let new_val = self.args[1]
                        .call(state, &[v, &Value::String(k.to_owned())])?
                        .into_owned();
                    res.insert(k.to_owned(), new_val);
                }
                Ok(ResolveResult::Owned(Value::Object(res)))
            }
            Value::Null => Ok(ResolveResult::Owned(Value::Null)),
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to map",
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

        let obj_res = source
            .try_as_object(&self.span)
            .ok()
            .map(|o| self.resolve_types_as_object(state, o));

        let arr_res = source
            .try_as_array(&self.span)
            .ok()
            .map(|a| self.resolve_types_as_array(state, a));

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
                source,
                self.span.clone(),
            )),
        }
    }
}

impl LambdaAcceptFunction for MapFunction {
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
                "map takes a function with one or two arguments",
            ));
        }
        Ok(())
    }
}

impl MapFunction {
    fn resolve_types_as_array(
        &'_ self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
        item_arr: crate::types::Array,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let mut elements = Vec::new();
        for (idx, item) in item_arr.elements.into_iter().enumerate() {
            let res = self.args[1].call_types(state, &[&item, &Type::from_const(idx)])?;
            elements.push(res);
        }
        let end_dynamic = if let Some(arr_end_dynamic) = item_arr.end_dynamic {
            Some(Box::new(
                self.args[1].call_types(state, &[&*arr_end_dynamic, &Type::Integer])?,
            ))
        } else {
            None
        };
        Ok(Type::Array(Array {
            elements,
            end_dynamic,
        }))
    }

    fn resolve_types_as_object(
        &'_ self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
        item_obj: crate::types::Object,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let mut fields = BTreeMap::new();
        for (k, v) in item_obj.fields {
            let arg = match &k {
                ObjectField::Constant(v) => Type::from_const(v.to_owned()),
                ObjectField::Generic => Type::String,
            };
            let res = self.args[1].call_types(state, &[&v, &arg])?;
            fields.insert(k, res);
        }
        Ok(Type::Object(Object { fields }))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        compile_expression_test,
        types::{Array, Object, ObjectField, Type},
    };

    #[test]
    pub fn test_simple_map() {
        let expr = compile_expression_test(r#"map([1, 2, 3, 4], (i) => pow(i, 2))"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(4, val_arr.len());
        assert_eq!(val_arr.first().unwrap().as_f64().unwrap(), 1.0);
        assert_eq!(val_arr.get(1).unwrap().as_f64().unwrap(), 4.0);
        assert_eq!(val_arr.get(2).unwrap().as_f64().unwrap(), 9.0);
        assert_eq!(val_arr.get(3).unwrap().as_f64().unwrap(), 16.0);
    }

    #[test]
    pub fn test_map_with_index() {
        let expr =
            compile_expression_test(r#"map(["a", "b", "c"], (it, index) => index)"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(3, val_arr.len());
        assert_eq!(0, val_arr.first().unwrap().as_u64().unwrap());
        assert_eq!(1, val_arr.get(1).unwrap().as_u64().unwrap());
        assert_eq!(2, val_arr.get(2).unwrap().as_u64().unwrap());
    }

    #[test]
    pub fn test_map_object() {
        let expr = compile_expression_test(
            r#"
        { "v1": 1, "v2": 2, "v3": 3 }.map(val => val * 2)
        "#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val_obj = res.as_object().unwrap();
        assert_eq!(2, val_obj["v1"].as_i64().unwrap());
        assert_eq!(4, val_obj["v2"].as_i64().unwrap());
        assert_eq!(6, val_obj["v3"].as_i64().unwrap());
    }

    #[test]
    pub fn test_map_object_with_key() {
        let expr = compile_expression_test(
            r#"
        { "v1": 1, "v2": 2, "v3": 3 }.map((val, key) => concat(val, key))
        "#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val_obj = res.as_object().unwrap();
        assert_eq!("1v1", val_obj["v1"].as_str().unwrap());
        assert_eq!("2v2", val_obj["v2"].as_str().unwrap());
        assert_eq!("3v3", val_obj["v3"].as_str().unwrap());
    }

    #[test]
    fn test_map_array_types() {
        let expr = compile_expression_test("map(input, it => string(it))", &["input"]).unwrap();
        let res = expr
            .run_types([Type::Array(Array {
                elements: vec![Type::String, Type::Float, Type::from_const(3)],
                end_dynamic: None,
            })])
            .unwrap();
        assert_eq!(
            res,
            Type::Array(Array {
                elements: vec![Type::String, Type::String, Type::String],
                end_dynamic: None
            })
        );

        let expr = compile_expression_test("map(input, (it, idx) => idx)", &["input"]).unwrap();
        let res = expr
            .run_types([Type::Array(Array {
                elements: vec![Type::String, Type::Float, Type::from_const(3)],
                end_dynamic: None,
            })])
            .unwrap();
        assert_eq!(
            res,
            Type::Array(Array {
                elements: vec![
                    Type::from_const(0),
                    Type::from_const(1),
                    Type::from_const(2)
                ],
                end_dynamic: None
            })
        );
    }

    #[test]
    fn test_map_object_types() {
        let expr =
            compile_expression_test("map(input, (val, key) => string(key))", &["input"]).unwrap();
        let res = expr
            .run_types([Type::Object(Object {
                fields: BTreeMap::from([
                    (ObjectField::Constant("a".to_string()), Type::String),
                    (ObjectField::Constant("b".to_string()), Type::Integer),
                    (ObjectField::Generic, Type::Float),
                ]),
            })])
            .unwrap();
        assert_eq!(
            res,
            Type::Object(Object {
                fields: BTreeMap::from([
                    (
                        ObjectField::Constant("a".to_string()),
                        Type::from_const("a"),
                    ),
                    (
                        ObjectField::Constant("b".to_string()),
                        Type::from_const("b"),
                    ),
                    (ObjectField::Generic, Type::String),
                ]),
            })
        );

        let res = expr.run_types([Type::Any]);
        assert_eq!(
            Type::object_of_type(Type::String).union_with(Type::array_of_type(Type::String)),
            res.unwrap()
        );
    }

    #[test]
    fn test_map_only_array_is_valid() {
        // The lambda produces an error if `it` is an integer, but not if it's an array,
        // so the result should only contain the array case, since the object case would always fail.
        // In general, type checking should do two things:
        //   - If we can prove that the expression _must_ fail, we should report that as an error.
        //   - If not, return the possible types of the expression if it passes.
        let expr =
            compile_expression_test("map(input, it => it.flatmap(a => a))", &["input"]).unwrap();
        let res = expr
            .run_types([Type::array_of_type(Type::array_of_type(Type::Integer))
                .union_with(Type::object_of_type(Type::Integer))])
            .unwrap();
        assert_eq!(Type::array_of_type(Type::array_of_type(Type::Integer)), res);
    }
}
