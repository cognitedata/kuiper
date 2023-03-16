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

    pub fn total_len(&self) -> usize {
        self.data.len()
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
    fn compute_input_product<'a>(
        &self,
        it: &'a TransformState,
        total_len: usize,
    ) -> (Vec<Vec<Option<&'a Value>>>, usize) {
        let mut res_len = 1usize;
        let mut num_inputs = 0usize;
        for key in self.inputs.used_inputs.iter() {
            let v = it.get_elem(key).unwrap();
            if !v.is_empty() {
                res_len *= v.len();
                if matches!(key, TransformOrInput::Input(_)) {
                    num_inputs += 1;
                }
            }
        }

        let mut res: Vec<Vec<Option<&'a Value>>> = Vec::with_capacity(res_len);

        let mut first = true;
        for key in self.inputs.used_inputs.iter() {
            let value = it.get_elem(key).unwrap();
            if value.is_empty() {
                continue;
            }
            if first {
                for el in value {
                    let mut chunk = vec![None; total_len];
                    chunk[key.get_index(num_inputs)] = Some(el.as_ref());
                    res.push(chunk);
                }
                first = false;
            } else {
                let mut next_res = Vec::new();
                for el in value {
                    for nested_vec in res.iter() {
                        let mut new_vec = nested_vec.clone();
                        new_vec[key.get_index(num_inputs)] = Some(el.as_ref());
                        next_res.push(new_vec);
                    }
                }
                res = next_res;
            }
        }

        (res, num_inputs)
    }

    fn compute_input_merge<'a>(
        &self,
        it: &'a TransformState,
    ) -> (Vec<Vec<Option<&'a Value>>>, usize) {
        let mut res: Vec<Vec<Option<&'a Value>>> = Vec::new();

        for key in self.inputs.used_inputs.iter() {
            if key == &TransformOrInput::Merge {
                continue;
            }
            let dat = it.get_elem(key).unwrap();
            for v in dat {
                let item = vec![Some(v.as_ref())];
                res.push(item);
            }
        }
        (res, 0)
    }

    fn compute_input_zip<'a>(
        &self,
        it: &'a TransformState,
        total_len: usize,
    ) -> (Vec<Vec<Option<&'a Value>>>, usize) {
        let mut res_len = 0usize;
        let mut num_inputs = 0usize;
        for key in self.inputs.used_inputs.iter() {
            let v = it.get_elem(key).unwrap();
            if v.len() > res_len {
                res_len = v.len();
            }
            if matches!(key, TransformOrInput::Input(_)) {
                num_inputs += 1;
            }
        }

        let mut res: Vec<Vec<Option<&'a Value>>> = Vec::with_capacity(res_len);
        for _ in 0..res_len {
            res.push(vec![None; total_len]);
        }

        for key in self.inputs.used_inputs.iter() {
            let dat = it.get_elem(key).unwrap();
            for i in 0..res_len {
                let el = res.get_mut(i).unwrap();

                if i >= dat.len() {
                    el[key.get_index(num_inputs)] = Some(it.null_const());
                } else {
                    el[key.get_index(num_inputs)] = Some(dat.get(i).unwrap().as_ref());
                }
            }
        }

        (res, num_inputs)
    }

    /// Execute a transform, internal method.
    fn execute_chunk<'a, 'd>(
        &'d self,
        data: &'a Vec<Option<&'d Value>>,
        num_inputs: usize,
        is_merge: bool,
    ) -> Result<Vec<ResolveResult<'d>>, TransformError> {
        let state = ExpressionExecutionState::<'d, 'a>::new(
            data,
            &self.inputs.inputs,
            &self.id,
            num_inputs,
        );
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
                        vec![ResolveResult::Borrowed(data.get(0).unwrap().unwrap())]
                    } else {
                        vec![ResolveResult::Borrowed(
                            data.iter()
                                .next()
                                .ok_or_else(|| {
                                    TransformError::InvalidProgramError(
                                    "Filter was expected to have at least one input, this is a bug"
                                        .to_string(),
                                )
                                })?
                                .unwrap(),
                        )]
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

        let (items, num_inputs) = match inputs.mode {
            super::input::TransformInputType::Product => {
                self.compute_input_product(raw_data, raw_data.total_len())
            }
            super::input::TransformInputType::Merge => self.compute_input_merge(raw_data),
            super::input::TransformInputType::Zip => {
                self.compute_input_zip(raw_data, raw_data.total_len())
            }
        };

        let is_merge = matches!(inputs.mode, TransformInputType::Merge);

        let res = if items.is_empty() {
            let data = Vec::new();
            self.execute_chunk(&data, num_inputs, is_merge)?
        } else {
            let mut res = Vec::new();
            for data in items {
                let next = self.execute_chunk(&data, num_inputs, is_merge)?;
                for it in next {
                    res.push(it);
                }
            }
            res
        };

        Ok(res)
    }
}
