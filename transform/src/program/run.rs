use once_cell::unsync::OnceCell;

use serde_json::Value;

use crate::expressions::{Expression, ExpressionExecutionState, ResolveResult, TransformError};

use super::{
    input::{Transform, TransformInputType, TransformOrInput, TransformType},
    Program,
};

// State for the transform as a whole.
// Number of entries is known from the start
// Must be able to:
//   Add new entries
//   Reference old entries
//   Reference new entries as they are added
// Do not need to
//   Reference new entries before they are added
//   Modify old entries
pub struct TransformState<'inp> {
    data: Vec<OnceCell<Vec<ResolveResult<'inp>>>>,
    null_const: Value,
    num_inputs: usize,
}

impl<'inp> TransformState<'inp> {
    pub fn new(num_transforms: usize, num_inputs: usize) -> Self {
        let mut dat = Vec::with_capacity(num_transforms + num_inputs);
        for _ in 0..(num_transforms + num_inputs) {
            dat.push(OnceCell::new());
        }
        Self {
            data: dat,
            null_const: Value::Null,
            num_inputs,
        }
    }

    pub fn get_elem<'a>(&'a self, key: &TransformOrInput) -> Option<&Vec<ResolveResult<'a>>>
    where
        'inp: 'a,
    {
        let idx = key.get_index(self.num_inputs);
        self.data.get(idx)?.get()
    }

    pub fn insert_elem<'a>(&'a self, key: TransformOrInput, value: Vec<ResolveResult<'inp>>) {
        let idx = key.get_index(self.num_inputs);

        self.data
            .get(idx)
            .unwrap()
            .set(value)
            .unwrap_or_else(|_| panic!("OnceCell already set!"));
    }

    pub fn null_const(&self) -> &Value {
        &self.null_const
    }
}

impl Program {
    pub fn execute_multiple<'a>(
        &self,
        input: &'a [&'a Value],
    ) -> Result<Vec<Value>, TransformError> {
        let data = TransformState::new(self.transforms.len(), input.len());
        for (idx, inp) in input.iter().enumerate() {
            data.insert_elem(
                TransformOrInput::Input(idx),
                vec![ResolveResult::Borrowed(inp)],
            );
        }

        let len = self.transforms.len();
        for (idx, tf) in self.transforms.iter().enumerate() {
            let value = tf.execute(&data)?;
            if idx == len - 1 {
                return Ok(value.into_iter().map(|r| r.into_owned()).collect());
            }
            // cached_results.insert(idx, value);
            data.insert_elem(TransformOrInput::Transform(idx), value);
        }
        Err(TransformError::InvalidProgramError(
            "No transforms in program".to_string(),
        ))
    }

    /// Execute the program on a JSON value. The output is a list of values or a compile error.
    pub fn execute(&self, input: &Value) -> Result<Vec<Value>, TransformError> {
        let data = TransformState::new(self.transforms.len(), 1);
        data.insert_elem(
            TransformOrInput::Input(0),
            vec![ResolveResult::Borrowed(input)],
        );

        let len = self.transforms.len();
        for (idx, tf) in self.transforms.iter().enumerate() {
            let value = tf.execute(&data)?;
            if idx == len - 1 {
                return Ok(value.into_iter().map(|r| r.into_owned()).collect());
            }
            data.insert_elem(TransformOrInput::Transform(idx), value);
        }
        Err(TransformError::InvalidProgramError(
            "No transforms in program".to_string(),
        ))
    }
}

impl Transform {
    /// Compute the product of each input with each other, so if the input is
    /// [1, 2, 3] and [1, 2], the result will be [1, 1], [1, 2], [2, 1], [2, 2], [3, 1] and [3, 2].
    fn compute_input_product<'a>(&self, it: &'a TransformState) -> Vec<Vec<&'a Value>> {
        let mut res_len = 1usize;
        for key in self.inputs.used_inputs.iter() {
            let v = it.get_elem(key).unwrap();
            if !v.is_empty() {
                res_len *= v.len();
            }
        }

        let mut res: Vec<Vec<&'a Value>> = Vec::with_capacity(res_len);

        let mut first = true;
        for key in &self.inputs.used_inputs {
            let value = it.get_elem(key).unwrap();
            if value.is_empty() {
                continue;
            }
            if first {
                for el in value {
                    let mut chunk = Vec::with_capacity(self.inputs.used_inputs.len());
                    chunk.push(el.as_ref());
                    res.push(chunk);
                }
                first = false;
            } else {
                let mut next_res = Vec::new();
                for el in value {
                    for nested_vec in res.iter() {
                        let mut new_vec = nested_vec.clone();
                        new_vec.push(el.as_ref());
                        next_res.push(new_vec);
                    }
                }
                res = next_res;
            }
        }

        res
    }

    fn compute_input_merge<'a>(&self, it: &'a TransformState) -> Vec<Vec<&'a Value>> {
        let mut res: Vec<Vec<&'a Value>> = Vec::new();

        for key in &self.inputs.used_inputs {
            if key == &TransformOrInput::Merge {
                continue;
            }
            let dat = it.get_elem(key).unwrap();
            for v in dat {
                let item = vec![v.as_ref()];
                res.push(item);
            }
        }
        res
    }

    fn compute_input_zip<'a>(&self, it: &'a TransformState) -> Vec<Vec<&'a Value>> {
        let mut res_len = 0usize;
        println!("{:?}", self.inputs.used_inputs);
        for key in &self.inputs.used_inputs {
            let v = it.get_elem(key).unwrap();
            println!("{:?}", v);
            if v.len() > res_len {
                res_len = v.len();
            }
        }

        let mut res: Vec<Vec<&'a Value>> = Vec::with_capacity(res_len);
        for _ in 0..res_len {
            res.push(vec![it.null_const(); self.inputs.used_inputs.len()]);
        }

        for (idx, key) in self.inputs.used_inputs.iter().enumerate() {
            let dat = it.get_elem(key).unwrap();
            for (idx2, v) in dat.iter().enumerate() {
                let el = res.get_mut(idx2).unwrap();
                el[idx] = v.as_ref();
            }
        }
        println!("{:?}", res);

        res
    }

    /// Execute a transform, internal method.
    fn execute_chunk<'a, 'd>(
        &'d self,
        data: &'a Vec<&'d Value>,
        is_merge: bool,
    ) -> Result<Vec<ResolveResult<'d>>, TransformError> {
        let state = ExpressionExecutionState::<'d, 'a>::new(data, &self.id);
        let res = self.map.resolve(&state)?;

        Ok(match self.transform_type {
            TransformType::Map => {
                if self.flatten {
                    match res {
                        ResolveResult::Borrowed(r) => match r {
                            Value::Array(a) => a.iter().map(ResolveResult::Borrowed).collect(),
                            x => vec![ResolveResult::Borrowed(x)],
                        },
                        ResolveResult::Owned(r) => match r {
                            Value::Array(a) => a.into_iter().map(ResolveResult::Owned).collect(),
                            x => vec![ResolveResult::Owned(x)],
                        },
                    }
                } else {
                    vec![res]
                }
            }
            TransformType::Filter => {
                let filter_output = match res.as_ref() {
                    Value::Null => false,
                    Value::Bool(x) => *x,
                    _ => true,
                };
                if filter_output {
                    if is_merge {
                        vec![ResolveResult::Borrowed(data.get(0).unwrap())]
                    } else {
                        vec![ResolveResult::Borrowed(data.iter().next().ok_or_else(
                            || {
                                TransformError::InvalidProgramError(
                                    "Filter was expected to have at least one input, this is a bug"
                                        .to_string(),
                                )
                            },
                        )?)]
                    }
                } else {
                    vec![]
                }
            }
        })
    }

    /// Execute the transform.
    pub fn execute<'a, 'b: 'a>(
        &'b self,
        raw_data: &'b TransformState<'b>,
    ) -> Result<Vec<ResolveResult<'b>>, TransformError> {
        let inputs = &self.inputs;

        let items = match inputs.mode {
            super::input::TransformInputType::Product => self.compute_input_product(raw_data),
            super::input::TransformInputType::Merge => self.compute_input_merge(raw_data),
            super::input::TransformInputType::Zip => self.compute_input_zip(raw_data),
        };

        let is_merge = matches!(inputs.mode, TransformInputType::Merge);

        let res = if items.is_empty() {
            let data = Vec::new();
            self.execute_chunk(&data, is_merge)?
        } else {
            let mut res = Vec::new();
            for data in items {
                let next = self.execute_chunk(&data, is_merge)?;
                for it in next {
                    res.push(it);
                }
            }
            res
        };

        Ok(res)
    }
}
