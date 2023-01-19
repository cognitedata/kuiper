use serde_json::{Map, Value};

use crate::{
    expressions::{base::ReferenceOrValue, Expression, ResolveResult},
    TransformError,
};

function_def!(PairsFunction, "pairs", 1);

impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for PairsFunction {
    fn resolve(
        &'a self,
        state: &'b crate::expressions::ExpressionExecutionState<'c, 'b>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let inp = self.args[0].resolve(state)?;
        let obj = match inp.into_value() {
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
        return Ok(ReferenceOrValue::Value(Value::Array(res)));
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
                "transform": "pairs($input)",
                "type": "flatten"
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
}
