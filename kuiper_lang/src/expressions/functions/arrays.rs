use itertools::Itertools;
use serde_json::{Number, Value};

use crate::{
    expressions::{base::get_number_from_value, Expression, ResolveResult},
    TransformError,
};

function_def!(LengthFunction, "length", 1);

impl<'a: 'c, 'c> Expression<'a, 'c> for LengthFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        let len = match source.as_ref() {
            serde_json::Value::String(s) => s.chars().count(),
            serde_json::Value::Array(a) => a.len(),
            serde_json::Value::Object(o) => o.len(),
            x => {
                return Err(TransformError::new_incorrect_type(
                    "Incorrect input to length",
                    "array, string, or object",
                    TransformError::value_desc(x),
                    &self.span,
                ))
            }
        };

        Ok(ResolveResult::Owned(Value::Number(Number::from(len))))
    }
}

function_def!(ChunkFunction, "chunk", 2);

impl<'a: 'c, 'c> Expression<'a, 'c> for ChunkFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
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
                ))
            }
        };

        let chunk_raw = self.args[1].resolve(state)?;
        let chunk_size = get_number_from_value("chunk", &chunk_raw, &self.span)?;
        let chunk_u = match chunk_size {
            crate::expressions::numbers::JsonNumber::NegInteger(_) => {
                return Err(TransformError::new_incorrect_type(
                    "Incorrect type for chunk size",
                    "positive integer",
                    "negative integer",
                    &self.span,
                ))
            }
            crate::expressions::numbers::JsonNumber::PosInteger(x) => x as usize,
            crate::expressions::numbers::JsonNumber::Float(_) => {
                return Err(TransformError::new_incorrect_type(
                    "Incorrect type for chunk size",
                    "positive integer",
                    "floating point",
                    &self.span,
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
            ));
        }

        let mut res = vec![];
        for chunk in arr.into_iter().chunks(chunk_u).into_iter() {
            res.push(Value::Array(chunk.collect()));
        }
        Ok(ResolveResult::Owned(Value::Array(res)))
    }
}

function_def!(TailFunction, "tail", 1, Some(2));

impl<'a: 'c, 'c> Expression<'a, 'c> for TailFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
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
                ))
            }
        };

        let number = match self.args.get(1) {
            None => 1,
            Some(exp) => {
                let res = exp.resolve(state)?;
                get_number_from_value("length", &res, &self.span)?.try_as_u64(&self.span)?
            }
        };

        match number {
            1 => Ok(ResolveResult::Owned(arr[arr.len() - 1].to_owned())),
            range => {
                let start = arr.len() - range as usize;
                let end = arr.len();
                Ok(ResolveResult::Owned(Value::Array(
                    arr[start..end].to_owned(),
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    pub fn test_length() {
        let expr = compile_expression(
            r#"{
            "v1": [1, 2, 3, 4].length(),
            "v2": "test test".length(),
            "v3": { "t": "t2", "t1": "t3" }.length()
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let obj = res.as_object().unwrap();
        assert_eq!(3, obj.len());
        assert_eq!(4, obj.get("v1").unwrap().as_u64().unwrap());
        assert_eq!(9, obj.get("v2").unwrap().as_u64().unwrap());
        assert_eq!(2, obj.get("v3").unwrap().as_u64().unwrap());
    }

    #[test]
    pub fn test_chunks() {
        let expr = compile_expression(
            r#"{
            "v1": [1, 2, 3, 4, 5, 6].chunk(4),
            "v2": ["test", 1, 2].chunk(1),
            "v3": [1, 2, 3, 4, 5, 6, 7].chunk(8)
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let obj = res.as_object().unwrap();
        assert_eq!(3, obj.len());
        assert_eq!(2, obj.get("v1").unwrap().as_array().unwrap().len());
        assert_eq!(3, obj.get("v2").unwrap().as_array().unwrap().len());
        assert_eq!(1, obj.get("v3").unwrap().as_array().unwrap().len());
    }
}
