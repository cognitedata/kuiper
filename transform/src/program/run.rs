use std::{cell::RefCell, collections::HashMap, ops::AddAssign};

use once_cell::unsync::OnceCell;

use serde_json::Value;

use crate::expressions::{Expression, ExpressionExecutionState, ResolveResult, TransformError};

use super::{
    input::{Transform, TransformOrInput},
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
    map: RefCell<HashMap<TransformOrInput, usize>>,
    idx_c: RefCell<usize>,
    null_const: Value,
}

impl<'inp> TransformState<'inp> {
    pub fn new(num_transforms: usize, num_inputs: usize) -> Self {
        let mut dat = Vec::with_capacity(num_transforms + num_inputs);
        for _ in 0..(num_transforms + num_inputs) {
            dat.push(OnceCell::new());
        }
        Self {
            data: dat,
            map: RefCell::new(HashMap::new()),
            idx_c: RefCell::new(0),
            null_const: Value::Null,
        }
    }

    pub fn get_elem<'a>(&'a self, key: &TransformOrInput) -> Option<&Vec<ResolveResult<'a>>>
    where
        'inp: 'a,
    {
        let idx = *self.map.borrow().get(key)?;
        assert!(idx < *self.idx_c.borrow());

        self.data.get(idx)?.get()
    }

    pub fn insert_elem<'a>(&'a self, key: TransformOrInput, value: Vec<ResolveResult<'inp>>) {
        let idx = *self.idx_c.borrow();

        self.data
            .get(idx)
            .unwrap()
            .set(value)
            .unwrap_or_else(|_| panic!("OnceCell already set!"));
        self.map.borrow_mut().insert(key, idx);
        self.idx_c.borrow_mut().add_assign(1);
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
            // cached_results.insert(idx, value);
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
    ) -> Vec<HashMap<TransformOrInput, &'a Value>> {
        let mut res_len = 1usize;
        let mut len = 0usize;
        for key in self.inputs().used_inputs.iter() {
            let v = it.get_elem(key).unwrap();
            if self.inputs().used_inputs.contains(key) && !v.is_empty() {
                len += 1;
                res_len *= v.len();
            }
        }

        let mut res: Vec<HashMap<TransformOrInput, &'a Value>> = Vec::with_capacity(res_len);

        let mut first = true;
        for key in self.inputs().used_inputs.iter() {
            let value = it.get_elem(key).unwrap();
            if first {
                for el in value {
                    let mut chunk = HashMap::with_capacity(len);
                    chunk.insert(key.clone(), el.as_ref());
                    res.push(chunk);
                }
                first = false;
            } else {
                let mut next_res = Vec::new();
                for el in value {
                    for nested_vec in res.iter() {
                        let mut new_vec = nested_vec.clone();
                        new_vec.insert(key.clone(), el.as_ref());
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
        it: &'a TransformState,
    ) -> Vec<HashMap<TransformOrInput, &'a Value>> {
        let mut res: Vec<HashMap<TransformOrInput, &'a Value>> = Vec::new();

        for key in self.inputs().used_inputs.iter() {
            if key == &TransformOrInput::Merge {
                continue;
            }
            let dat = it.get_elem(key).unwrap();
            for v in dat {
                let mut map = HashMap::with_capacity(1);
                map.insert(TransformOrInput::Merge, v.as_ref());
                res.push(map);
            }
        }
        res
    }

    fn compute_input_zip<'a>(
        &self,
        it: &'a TransformState,
    ) -> Vec<HashMap<TransformOrInput, &'a Value>> {
        let mut res_len = 0usize;
        for key in self.inputs().used_inputs.iter() {
            let v = it.get_elem(key).unwrap();
            if v.len() > res_len {
                res_len = v.len();
            }
        }

        let mut res: Vec<HashMap<TransformOrInput, &'a Value>> = Vec::with_capacity(res_len);
        for _ in 0..res_len {
            res.push(HashMap::new());
        }

        for key in self.inputs().used_inputs.iter() {
            let dat = it.get_elem(key).unwrap();
            for i in 0..res_len {
                let el = res.get_mut(i).unwrap();
                if i >= dat.len() {
                    el.insert(key.clone(), it.null_const());
                } else {
                    el.insert(key.clone(), dat.get(i).unwrap().as_ref());
                }
            }
        }

        res
    }

    /// Execute a transform, internal method.
    fn execute_chunk<'a, 'd>(
        &'d self,
        data: &'a HashMap<TransformOrInput, &'d Value>,
    ) -> Result<Vec<ResolveResult<'d>>, TransformError> {
        let state = ExpressionExecutionState::<'d, 'a>::new(data, &self.inputs().inputs, self.id());
        Ok(match self {
            Self::Map(m) => {
                let mut map = serde_json::Map::new();
                for (key, tf) in m.map.iter() {
                    let value = tf.resolve(&state)?.into_owned();
                    map.insert(key.clone(), value);
                }
                vec![ResolveResult::Owned(Value::Object(map))]
            }

            Self::Flatten(m) => {
                let res: ResolveResult<'d> = m.map.resolve(&state)?;
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
                        vec![ResolveResult::Borrowed(
                            data.get(&TransformOrInput::Merge).unwrap(),
                        )]
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
                                .1,
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
