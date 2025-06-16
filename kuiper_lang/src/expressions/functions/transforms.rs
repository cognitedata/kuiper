use serde_json::{Map, Value};

use crate::{
    expressions::{Expression, ResolveResult},
    types::{Object, ObjectField, Sequence, Type},
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
        let inp = self.args[0].resolve_types(state)?;
        let inp_obj = inp.try_as_object(&self.span)?;
        Ok(Type::Sequence(Sequence {
            elements: vec![],
            end_dynamic: Some(Box::new(Type::Object(Object {
                fields: [
                    (ObjectField::Constant("key".to_string()), Type::String),
                    (
                        ObjectField::Constant("value".to_string()),
                        inp_obj.element_union(),
                    ),
                ]
                .into_iter()
                .collect(),
            }))),
        }))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::compile_expression;

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
}
