use std::collections::HashMap;

use serde_json::Value;

use crate::expressions::{Expression, ExpressionExecutionState, ResolveResult, TransformError};

use super::{
    input::{Transform, TransformOrInput},
    Program,
};

impl Program {
    pub fn execute(&self, input: Value) -> Result<Vec<Value>, TransformError> {
        let mut result = HashMap::<TransformOrInput, Vec<ResolveResult>>::new();
        result.insert(
            TransformOrInput::Input,
            vec![ResolveResult::Reference(&input)],
        );

        let len = self.transforms.len();
        for (idx, tf) in self.transforms.iter().enumerate() {
            let value = tf.execute(&result)?;
            if idx == len - 1 {
                return Ok(value);
            }
            // cached_results.insert(idx, value);
            result.insert(
                TransformOrInput::Transform(idx),
                value.into_iter().map(ResolveResult::Value).collect(),
            );
        }
        Err(TransformError::SourceMissingError(
            "No transforms in program".to_string(),
        ))
    }
}

fn compute_input_product<'a>(
    it: &'a HashMap<TransformOrInput, Vec<ResolveResult<'a>>>,
) -> Vec<HashMap<TransformOrInput, ResolveResult<'a>>> {
    let len = it.iter().fold(1usize, |acc, v| acc * v.1.len());
    let mut res: Vec<HashMap<TransformOrInput, ResolveResult<'a>>> = Vec::with_capacity(len);

    for (idx, (key, value)) in it.iter().enumerate() {
        if idx == 0 {
            for el in value {
                let mut chunk = HashMap::with_capacity(it.len());
                chunk.insert(key.clone(), el.as_self_ref());
                res.push(chunk);
            }
        } else {
            let mut next_res = Vec::new();
            for el in value {
                for nested_vec in res.iter() {
                    let mut new_vec = nested_vec.clone();
                    new_vec.insert(key.clone(), el.as_self_ref());
                    next_res.push(new_vec);
                }
            }
            res = next_res;
        }
    }
    res
}

impl Transform {
    pub fn execute(
        &self,
        raw_data: &HashMap<TransformOrInput, Vec<ResolveResult>>,
    ) -> Result<Vec<Value>, TransformError> {
        let items = compute_input_product(&raw_data);

        let mut res = Vec::new();
        for data in items {
            let state = ExpressionExecutionState::new(&data, self.inputs());
            let next = match self {
                Self::Map(m) => {
                    let mut map = serde_json::Map::new();
                    for (key, tf) in m.map.iter() {
                        let res = tf.resolve(&state)?;
                        let value = match res {
                            ResolveResult::Reference(r) => r.clone(),
                            ResolveResult::Value(r) => r,
                        };
                        map.insert(key.clone(), value);
                    }
                    vec![Value::Object(map)]
                }
                Self::Flatten(m) => {
                    let res = m.map.resolve(&state)?;
                    let value = match res {
                        ResolveResult::Reference(r) => r.clone(),
                        ResolveResult::Value(r) => r,
                    };
                    match value {
                        Value::Array(a) => a,
                        _ => vec![value],
                    }
                }
            };
            for it in next {
                res.push(it);
            }
        }
        Ok(res)
    }
}
