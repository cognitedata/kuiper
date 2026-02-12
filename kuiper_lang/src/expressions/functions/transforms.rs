use serde_json::{Map, Value};

use crate::{
    expressions::{Expression, ResolveResult},
    types::{Array, Object, ObjectField, Type},
    TransformError,
};

function_def!(PairsFunction, "pairs", 1);

impl<'a: 'c, 'c> Expression<'a, 'c> for PairsFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let inp = self.args[0].resolve(state)?;
        let obj = match inp.into_owned() {
            Value::Object(o) => o,
            x => {
                return Err(TransformError::new_incorrect_type(
                    "invalid argument to pairs function",
                    "obj",
                    TransformError::value_desc(&x),
                    &self.span,
                ));
            }
        };
        let mut res = vec![];
        for (key, val) in obj {
            let mut map = Map::new();
            map.insert("key".to_string(), Value::String(key));
            map.insert("value".to_string(), val);
            res.push(Value::Object(map));
        }
        Ok(ResolveResult::Owned(Value::Array(res)))
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let item = self.args[0].resolve_types(state)?;
        let item_obj = item.try_as_object(&self.span)?;
        if item_obj
            .fields
            .contains_key(&crate::types::ObjectField::Generic)
        {
            // We can't know anything about the ordering of the fields if any field is generic...
            let field_type = item_obj.element_union();
            Ok(Type::array_of_type(Type::Object(Object {
                fields: [
                    (ObjectField::Constant("key".to_owned()), Type::String),
                    (ObjectField::Constant("value".to_owned()), field_type),
                ]
                .into_iter()
                .collect(),
            })))
        } else {
            // Since we use a BTreeMap in both cases, the order of the fields will be the same.
            let mut entries = Vec::new();
            for (field, elem) in item_obj.fields {
                let ObjectField::Constant(key) = field else {
                    // Should be unreachable.
                    continue;
                };
                entries.push(Type::Object(Object {
                    fields: [
                        (
                            ObjectField::Constant("key".to_owned()),
                            Type::from_const(key),
                        ),
                        (ObjectField::Constant("value".to_owned()), elem),
                    ]
                    .into_iter()
                    .collect(),
                }));
            }
            Ok(Type::Array(Array {
                elements: entries,
                end_dynamic: None,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        compile_expression,
        types::{Object, ObjectField, Type},
    };

    #[test]
    pub fn test_pairs() {
        let expr = compile_expression("pairs(input)", &["input"]).unwrap();

        let inp = json!({
            "k1": "v1",
            "k2": "v2",
            "k3": 123
        });

        let res_raw = expr.run([&inp]).unwrap();

        let res = res_raw.as_array().unwrap();
        assert_eq!(res.len(), 3);

        let val = res.first().unwrap();
        assert_eq!("k1", val.get("key").unwrap().as_str().unwrap());
        assert_eq!("v1", val.get("value").unwrap().as_str().unwrap());
        let val = res.get(1).unwrap();
        assert_eq!("k2", val.get("key").unwrap().as_str().unwrap());
        assert_eq!("v2", val.get("value").unwrap().as_str().unwrap());
        let val = res.get(2).unwrap();
        assert_eq!("k3", val.get("key").unwrap().as_str().unwrap());
        assert_eq!(123, val.get("value").unwrap().as_u64().unwrap());
    }

    #[test]
    fn test_pairs_types() {
        let expr = compile_expression("pairs(input)", &["input"]).unwrap();
        let ty = expr
            .run_types([Type::Object(Object {
                fields: [
                    (ObjectField::Constant("k1".to_owned()), Type::String),
                    (
                        ObjectField::Constant("k2".to_owned()),
                        Type::from_const("v2"),
                    ),
                    (ObjectField::Constant("k3".to_owned()), Type::from_const(3)),
                ]
                .into_iter()
                .collect(),
            })])
            .unwrap();

        fn elem_obj(key: &str, val: Type) -> Type {
            Type::Object(Object {
                fields: [
                    (
                        ObjectField::Constant("key".to_owned()),
                        Type::from_const(key),
                    ),
                    (ObjectField::Constant("value".to_owned()), val),
                ]
                .into_iter()
                .collect(),
            })
        }

        assert_eq!(
            ty,
            Type::Array(crate::types::Array {
                elements: vec![
                    elem_obj("k1", Type::String),
                    elem_obj("k2", Type::from_const("v2")),
                    elem_obj("k3", Type::from_const(3)),
                ],
                end_dynamic: None,
            })
        );

        let ty = expr
            .run_types([Type::Object(Object {
                fields: [
                    (ObjectField::Generic, Type::String),
                    (ObjectField::Constant("k1".to_owned()), Type::Integer),
                ]
                .into_iter()
                .collect(),
            })])
            .unwrap();
        assert_eq!(
            ty,
            Type::array_of_type(Type::Object(Object {
                fields: [
                    (ObjectField::Constant("key".to_owned()), Type::String),
                    (
                        ObjectField::Constant("value".to_owned()),
                        Type::Integer.union_with(Type::String)
                    ),
                ]
                .into_iter()
                .collect(),
            }))
        );
    }
}
