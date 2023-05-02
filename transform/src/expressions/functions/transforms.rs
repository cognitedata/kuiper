use serde_json::{Map, Value};

use crate::{
    expressions::{Expression, ResolveResult},
    TransformError,
};

function_def!(PairsFunction, "pairs", 1);

impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for PairsFunction {
    fn resolve(
        &'a self,
        state: &'b crate::expressions::ExpressionExecutionState<'c, 'b>,
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
                    state.id,
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
        return Ok(ResolveResult::Owned(Value::Array(res)));
    }
}

// Takes an outer object, and inner array. Flattens the inner array, then joins each element of it
// with the outer object, and returns a combined array.
// If no outer object is specified, just flattens.
function_def!(FlattenFunction, "flatten", 1, Some(2));

impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for FlattenFunction {
    fn resolve(
        &'a self,
        state: &'b crate::expressions::ExpressionExecutionState<'c, 'b>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let inner = self.args.get(0).unwrap().resolve(state)?;
        let inner_flat = Self::flatten_rec(inner.into_owned());
        if let Some(a2) = self.args.get(1) {
            let outer = a2.resolve(state)?;
            let Value::Object(outer) = outer.as_ref() else {
                return Err(TransformError::new_incorrect_type("invalid argument 'outer' to pairs function", "obj", TransformError::value_desc(&outer), &self.span, state.id))
            };
            let mut res = vec![];
            for it in inner_flat.into_iter() {
                let mut inner_map = match it {
                    Value::Object(o) => o,
                    x => {
                        let mut m = Map::new();
                        m.insert("inner".to_string(), x);
                        m
                    }
                };
                for (key, value) in outer.iter() {
                    inner_map.insert(key.clone(), value.clone());
                }
                res.push(Value::Object(inner_map));
            }
            Ok(ResolveResult::Owned(Value::Array(res)))
        } else {
            Ok(ResolveResult::Owned(Value::Array(inner_flat)))
        }
    }
}

impl FlattenFunction {
    fn flatten_rec(val: Value) -> Vec<Value> {
        let mut result = vec![];
        match val {
            Value::Array(a) => {
                for v in a.into_iter() {
                    result.extend(Self::flatten_rec(v).into_iter());
                }
            }
            x => result.push(x),
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::Program;

    #[test]
    pub fn test_pairs() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "pairs",
                "inputs": ["input"],
                "transform": "pairs(input)",
                "expandOutput": true
            }]))
            .unwrap(),
        )
        .unwrap();

        let inp = json!({
            "k1": "v1",
            "k2": "v2",
            "k3": 123
        });

        let res = program.execute(&inp).unwrap();

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
    pub fn test_flatten_simple() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "flat",
                "inputs": ["input"],
                "transform": "flatten(input.data)",
                "expandOutput": true
            }]))
            .unwrap(),
        )
        .unwrap();

        let inp = json!({
            "data": [1, 2, 3, 4, 5]
        });

        let res = program.execute(&inp).unwrap();

        assert_eq!(res.len(), 5);

        for i in 1..6 {
            assert_eq!(res[i - 1].as_u64().unwrap(), i as u64);
        }
    }

    #[test]
    pub fn test_flatten_expand() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "flat",
                "inputs": ["input"],
                "transform": "flatten(input.data, input)",
                "expandOutput": true
            }]))
            .unwrap(),
        )
        .unwrap();

        let inp = json!({
            "outer_value": "test",
            "data": [1, 2, 3, 4, 5]
        });

        let res = program.execute(&inp).unwrap();

        assert_eq!(res.len(), 5);

        for i in 1..6 {
            assert_eq!(res[i - 1].get("inner").unwrap().as_u64().unwrap(), i as u64);
            assert_eq!(
                res[i - 1].get("outer_value").unwrap().as_str().unwrap(),
                "test"
            );
        }
    }

    #[test]
    pub fn test_flatten_merge() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "flat",
                "inputs": ["input"],
                "transform": "flatten(input.data, input)",
                "expandOutput": true
            }]))
            .unwrap(),
        )
        .unwrap();

        let inp = json!({
            "outer_value": "test",
            "data": [
                {
                    "val": 1,
                    "t": 1
                }, {
                    "val": 2,
                    "t": 2
                }, {
                    "val": 3,
                    "t": 3
                }
            ]
        });

        let res = program.execute(&inp).unwrap();

        assert_eq!(res.len(), 3);

        for i in 1..4 {
            assert_eq!(res[i - 1].get("val").unwrap().as_u64().unwrap(), i as u64);
            assert_eq!(res[i - 1].get("t").unwrap().as_u64().unwrap(), i as u64);
            assert_eq!(
                res[i - 1].get("outer_value").unwrap().as_str().unwrap(),
                "test"
            );
        }
    }
}
