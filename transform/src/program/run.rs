use std::collections::HashMap;

use serde_json::Value;

use crate::expressions::{Expression, ExpressionExecutionState, ResolveResult, TransformError};

use super::{
    input::{Transform, TransformOrInput},
    Program,
};

impl Program {
    pub fn execute(&self, input: &Value) -> Result<Vec<Value>, TransformError> {
        let mut result = HashMap::<TransformOrInput, Vec<ResolveResult>>::new();
        result.insert(
            TransformOrInput::Input,
            vec![ResolveResult::Reference(input)],
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
        Err(TransformError::InvalidProgramError(
            "No transforms in program".to_string(),
        ))
    }
}

impl Transform {
    fn compute_input_product<'a>(
        &self,
        it: &'a HashMap<TransformOrInput, Vec<ResolveResult<'a>>>,
    ) -> Vec<HashMap<TransformOrInput, ResolveResult<'a>>> {
        let mut res_len = 1usize;
        let mut len = 1usize;
        for (k, v) in it.iter() {
            if self.inputs().used_inputs.contains(k) && !v.is_empty() {
                len += 1;
                res_len *= v.len();
            }
        }

        let mut res: Vec<HashMap<TransformOrInput, ResolveResult<'a>>> =
            Vec::with_capacity(res_len);

        let mut first = true;
        for (key, value) in it.iter() {
            if !self.inputs().used_inputs.contains(key) || value.is_empty() {
                continue;
            }
            if first {
                for el in value {
                    let mut chunk = HashMap::with_capacity(len);
                    chunk.insert(key.clone(), el.as_self_ref());
                    res.push(chunk);
                }
                first = false;
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

    fn execute_chunk(
        &self,
        data: &HashMap<TransformOrInput, ResolveResult>,
    ) -> Result<Vec<Value>, TransformError> {
        let state = ExpressionExecutionState::new(data, &self.inputs().inputs, self.id());
        Ok(match self {
            Self::Map(m) => {
                let mut map = serde_json::Map::new();
                for (key, tf) in m.map.iter() {
                    let value = tf.resolve(&state)?.into_value();
                    map.insert(key.clone(), value);
                }
                vec![Value::Object(map)]
            }
            Self::Flatten(m) => {
                let value = m.map.resolve(&state)?.into_value();
                match value {
                    Value::Array(a) => a,
                    _ => vec![value],
                }
            }
        })
    }

    pub fn execute(
        &self,
        raw_data: &HashMap<TransformOrInput, Vec<ResolveResult>>,
    ) -> Result<Vec<Value>, TransformError> {
        let items = self.compute_input_product(raw_data);

        let res = if items.is_empty() {
            let data = HashMap::new();
            self.execute_chunk(&data)?
        } else {
            let mut res = Vec::new();
            for data in items {
                let next = self.execute_chunk(&data)?;
                for it in next {
                    res.push(it);
                }
            }
            res
        };

        Ok(res)
    }
}
