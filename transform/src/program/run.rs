use std::collections::HashMap;

use serde_json::Value;

use crate::expressions::{Expression, ExpressionExecutionState, ResolveResult, TransformError};

use super::{
    input::{Transform, TransformOrInput},
    Program,
};

impl Program {
    /// Execute the program on a JSON value. The output is a list of values or a compile error.
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
    /// Compute the product of each input with each other, so if the input is
    /// [1, 2, 3] and [1, 2], the result will be [1, 1], [1, 2], [2, 1], [2, 2], [3, 1] and [3, 2].
    fn compute_input_product<'a>(
        &self,
        it: &'a HashMap<TransformOrInput, Vec<ResolveResult<'a>>>,
    ) -> Vec<HashMap<TransformOrInput, ResolveResult<'a>>> {
        let mut res_len = 1usize;
        let mut len = 0usize;
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

    fn compute_input_merge<'a>(
        &self,
        it: &'a HashMap<TransformOrInput, Vec<ResolveResult<'a>>>,
    ) -> Vec<HashMap<TransformOrInput, ResolveResult<'a>>> {
        let mut res: Vec<HashMap<TransformOrInput, ResolveResult<'a>>> = Vec::new();

        for (key, value) in it.iter() {
            if !self.inputs().used_inputs.contains(key) {
                continue;
            }
            for v in value {
                let mut map = HashMap::with_capacity(1);
                map.insert(TransformOrInput::Merge, v.as_self_ref());
                res.push(map);
            }
        }
        res
    }

    fn compute_input_zip<'a>(
        &self,
        it: &'a HashMap<TransformOrInput, Vec<ResolveResult<'a>>>,
    ) -> Vec<HashMap<TransformOrInput, ResolveResult<'a>>> {
        let mut res_len = 0usize;
        for (k, v) in it.iter() {
            if self.inputs().used_inputs.contains(k) && v.len() > res_len {
                res_len = v.len();
            }
        }
        let mut res: Vec<HashMap<TransformOrInput, ResolveResult<'a>>> =
            Vec::with_capacity(res_len);
        for _ in 0..res_len {
            res.push(HashMap::new());
        }

        for (key, value) in it.iter() {
            for i in 0..res_len {
                let el = res.get_mut(i).unwrap();
                if i >= value.len() {
                    el.insert(key.clone(), ResolveResult::Value(Value::Null));
                } else {
                    el.insert(key.clone(), value.get(i).unwrap().as_self_ref());
                }
            }
        }
        res
    }

    /// Execute a transform, internal method.
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
            Self::Filter(m) => {
                let value = m.map.resolve(&state)?;
                let filter_output = match value.as_ref() {
                    Value::Null => false,
                    Value::Bool(x) => *x,
                    _ => true,
                };
                if filter_output {
                    if data.contains_key(&TransformOrInput::Merge) {
                        vec![data
                            .get(&TransformOrInput::Merge)
                            .unwrap()
                            .clone()
                            .into_value()]
                    } else {
                        vec![data
                            .iter()
                            .next()
                            .ok_or_else(|| {
                                TransformError::InvalidProgramError(
                                    "Filter was expected to have at least one input, this is a bug"
                                        .to_string(),
                                )
                            })?
                            .1
                            .clone()
                            .into_value()]
                    }
                } else {
                    vec![]
                }
            }
        })
    }

    /// Execute the transform.
    pub fn execute(
        &self,
        raw_data: &HashMap<TransformOrInput, Vec<ResolveResult>>,
    ) -> Result<Vec<Value>, TransformError> {
        let inputs = self.inputs();

        let items = match inputs.mode {
            super::input::TransformInputType::Product => self.compute_input_product(raw_data),
            super::input::TransformInputType::Merge => self.compute_input_merge(raw_data),
            super::input::TransformInputType::Zip => self.compute_input_zip(raw_data),
        };

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
