use itertools::Itertools;
use serde_json::{Number, Value};

use crate::{
    expressions::{base::get_number_from_value, Expression, ResolveResult},
    TransformError,
};

function_def!(LengthFunction, "length", 1);

impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for LengthFunction {
    fn resolve(
        &'a self,
        state: &'b crate::expressions::ExpressionExecutionState<'c, 'b>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        let len = match source.as_ref() {
            serde_json::Value::String(s) => s.len(),
            serde_json::Value::Array(a) => a.len(),
            serde_json::Value::Object(o) => o.len(),
            x => {
                return Err(TransformError::new_incorrect_type(
                    "Incorrect input to length",
                    "array, string, or object",
                    TransformError::value_desc(&x),
                    &self.span,
                    state.id,
                ))
            }
        };

        Ok(ResolveResult::Owned(Value::Number(Number::from(len))))
    }
}

function_def!(ChunkFunction, "chunk", 2);

impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for ChunkFunction {
    fn resolve(
        &'a self,
        state: &'b crate::expressions::ExpressionExecutionState<'c, 'b>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let source = self.args[0].resolve(state)?;

        let arr = match source {
            ResolveResult::Borrowed(Value::Array(a)) => a.clone(),
            ResolveResult::Owned(Value::Array(a)) => a,
            x => {
                return Err(TransformError::new_incorrect_type(
                    "Incorrect input to chunk",
                    "array",
                    TransformError::value_desc(x.as_ref()),
                    &self.span,
                    state.id,
                ))
            }
        };

        let chunk_raw = self.args[1].resolve(state)?;
        let chunk_size = get_number_from_value("chunk", &chunk_raw, &self.span, state.id)?;
        let chunk_u = match chunk_size {
            crate::expressions::numbers::JsonNumber::NegInteger(_) => {
                return Err(TransformError::new_incorrect_type(
                    "Incorrect type for chunk size",
                    "positive integer",
                    "negative integer",
                    &self.span,
                    state.id,
                ))
            }
            crate::expressions::numbers::JsonNumber::PosInteger(x) => x as usize,
            crate::expressions::numbers::JsonNumber::Float(_) => {
                return Err(TransformError::new_incorrect_type(
                    "Incorrect type for chunk size",
                    "positive integer",
                    "floating point",
                    &self.span,
                    state.id,
                ))
            }
        };
        if arr.len() <= chunk_u {
            return Ok(ResolveResult::Owned(Value::Array(vec![Value::Array(arr)])));
        }

        if chunk_u == 0 {
            return Err(TransformError::new_invalid_operation(
                "Chunk size must be greater than 0".to_string(),
                &self.span,
                state.id,
            ));
        }

        let mut res = vec![];
        for chunk in arr.into_iter().chunks(chunk_u).into_iter() {
            res.push(Value::Array(chunk.collect()));
        }
        Ok(ResolveResult::Owned(Value::Array(res)))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::Program;

    #[test]
    pub fn test_length() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "map",
                "inputs": [],
                "transform": r#"{
                    "v1": [1, 2, 3, 4].length(),
                    "v2": "test test".length(),
                    "v3": { "t": "t2", "t1": "t3" }.length()   
                }"#
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program.execute(&Value::Null).unwrap();

        assert_eq!(res.len(), 1);
        let val = res.first().unwrap();
        let obj = val.as_object().unwrap();
        assert_eq!(3, obj.len());
        assert_eq!(4, obj.get("v1").unwrap().as_u64().unwrap());
        assert_eq!(9, obj.get("v2").unwrap().as_u64().unwrap());
        assert_eq!(2, obj.get("v3").unwrap().as_u64().unwrap());
    }

    #[test]
    pub fn test_chunks() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "map",
                "inputs": [],
                "transform": r#"{
                    "v1": [1, 2, 3, 4, 5, 6].chunk(4),
                    "v2": ["test", 1, 2].chunk(1),
                    "v3": [1, 2, 3, 4, 5, 6, 7].chunk(8)
                }"#
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program.execute(&Value::Null).unwrap();

        assert_eq!(res.len(), 1);
        let val = res.first().unwrap();
        let obj = val.as_object().unwrap();
        assert_eq!(3, obj.len());
        assert_eq!(2, obj.get("v1").unwrap().as_array().unwrap().len());
        assert_eq!(3, obj.get("v2").unwrap().as_array().unwrap().len());
        assert_eq!(1, obj.get("v3").unwrap().as_array().unwrap().len());
    }
}
