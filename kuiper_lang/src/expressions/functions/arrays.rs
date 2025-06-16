use itertools::Itertools;
use serde_json::{Number, Value};

use crate::{
    expressions::{
        numbers::JsonNumber,
        types::{Sequence, Type, TypeError},
        Expression, ResolveResult,
    },
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

    fn resolve_types(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<Type, TypeError> {
        let source = self.args[0].resolve_types(state)?;
        source.assert_assignable_to(
            &Type::Union(vec![Type::String, Type::any_array(), Type::any_object()]),
            &self.span,
        )?;
        Ok(Type::Integer)
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

        let chunk_size = self.args[1]
            .resolve(state)?
            .try_as_number("chunk", &self.span)?
            .try_as_u64(&self.span)? as usize;

        if chunk_size == 0 {
            return Err(TransformError::new_invalid_operation(
                "Chunk size must be greater than 0".to_string(),
                &self.span,
            ));
        }

        if arr.len() <= chunk_size {
            return Ok(ResolveResult::Owned(Value::Array(vec![Value::Array(arr)])));
        }

        let mut res = vec![];
        for chunk in arr.into_iter().chunks(chunk_size).into_iter() {
            res.push(Value::Array(chunk.collect()));
        }
        Ok(ResolveResult::Owned(Value::Array(res)))
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<Type, TypeError> {
        let source = self.args[0].resolve_types(state)?;
        let chunk_size = self.args[1].resolve_types(state)?;
        chunk_size.assert_assignable_to(&Type::Integer, &self.span)?;
        let source_arr = source.try_as_array(&self.span)?;
        Ok(Type::Sequence(Sequence {
            end_dynamic: Some(Box::new(Type::Sequence(Sequence {
                elements: Vec::new(),
                end_dynamic: Some(Box::new(source_arr.element_union())),
            }))),
            elements: Vec::new(),
        }))
    }
}

function_def!(TailFunction, "tail", 1, Some(2));

impl<'a: 'c, 'c> Expression<'a, 'c> for TailFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let source = self.args[0].resolve(state)?;

        let arr = match source.as_ref() {
            Value::Array(a) => a,
            x => {
                return Err(TransformError::new_incorrect_type(
                    "Incorrect input to tail",
                    "array",
                    TransformError::value_desc(x),
                    &self.span,
                ))
            }
        };

        let number = match self.args.get(1) {
            None => 1,
            Some(exp) => exp
                .resolve(state)?
                .try_as_number("tail", &self.span)?
                .try_as_u64(&self.span)?,
        };

        match number {
            1 => {
                if arr.is_empty() {
                    Ok(ResolveResult::Owned(Value::Null))
                } else {
                    Ok(ResolveResult::Owned(arr[arr.len() - 1].to_owned()))
                }
            }
            range => {
                let start = arr.len().saturating_sub(range as usize);
                let end = arr.len();
                Ok(ResolveResult::Owned(Value::Array(
                    arr[start..end].to_owned(),
                )))
            }
        }
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::expressions::types::TypeExecutionState<'c, '_>,
    ) -> Result<crate::expressions::types::Type, crate::expressions::types::TypeError> {
        let source = self.args[0].resolve_types(state)?;
        let source_arr = source.try_as_array(&self.span)?;

        match self.args.get(1) {
            None => Ok(source_arr.index_from_end(0).unwrap_or_else(Type::null)),
            Some(exp) => {
                let number = exp.resolve_types(state)?;
                let key = number.clone();
                number.assert_assignable_to(&Type::Integer, &self.span)?;
                if let Type::Constant(Value::Number(n)) = number {
                    let Some(n) = n.as_u64() else {
                        return Err(TypeError::ExpectedType(
                            Type::Integer,
                            key,
                            self.span.clone(),
                        ));
                    };
                    if n == 1 {
                        Ok(source_arr.index_from_end(0).unwrap_or_else(Type::null))
                    } else {
                        if let Some(end_dynamic) = source_arr.end_dynamic {
                            Ok(Type::Sequence(Sequence {
                                end_dynamic: Some(end_dynamic),
                                elements: vec![],
                            }))
                        } else {
                            let start = source_arr.elements.len().saturating_sub(n as usize);
                            let res = source_arr.elements[start..].to_owned();

                            Ok(Type::Sequence(Sequence {
                                elements: res,
                                end_dynamic: None,
                            }))
                        }
                    }
                } else {
                    let mut res = Type::Union(Vec::new());
                    // If the value is 0
                    res = res.union_with(Type::null());
                    // If the value is 1
                    res = res.union_with(source_arr.index_from_end(0).unwrap_or_else(Type::null));
                    // If the value is greater than 1. We don't really want to return a combinatorial explosion
                    // of possible sequences.
                    res = res.union_with(Type::Sequence(Sequence {
                        elements: vec![],
                        end_dynamic: Some(Box::new(source_arr.element_union())),
                    }));
                    Ok(res)
                }
            }
        }
    }
}

function_def!(SliceFunction, "slice", 2, Some(3));

impl<'a: 'c, 'c> Expression<'a, 'c> for SliceFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let inp_value = self.args[0].resolve(state)?;
        let inp_array = inp_value.as_array().ok_or_else(|| {
            TransformError::new_incorrect_type(
                "slice",
                "array",
                TransformError::value_desc(&inp_value),
                &self.span,
            )
        })?;

        let start = self.args[1]
            .resolve(state)?
            .try_as_number("slice", &self.span)?
            .try_as_i64(&self.span)?;

        let end_value: Option<Result<i64, crate::TransformError>> = self.args.get(2).map(|c| {
            c.resolve(state)?
                .try_as_number("slice", &self.span)?
                .try_as_i64(&self.span)
        });
        let end = end_value.transpose()?;
        if end.is_some_and(|v| v == start) {
            return Ok(ResolveResult::Owned(Value::Array(Vec::new())));
        }

        let start = get_array_index(inp_array, start);

        if let Some(end) = end {
            let end = get_array_index(inp_array, end);
            if end <= start {
                return Ok(ResolveResult::Owned(Value::Array(vec![])));
            }
            Ok(ResolveResult::Owned(Value::Array(
                inp_array[start..end].iter().cloned().collect_vec(),
            )))
        } else {
            Ok(ResolveResult::Owned(Value::Array(
                inp_array[start..].iter().cloned().collect_vec(),
            )))
        }
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::expressions::types::TypeExecutionState<'c, '_>,
    ) -> Result<Type, TypeError> {
        let inp_value = self.args[0].resolve_types(state)?;
        let inp_array = inp_value.try_as_array(&self.span)?;

        let start = self.args[1].resolve_types(state)?;
        start.assert_assignable_to(&Type::Integer, &self.span)?;

        let end = self
            .args
            .get(2)
            .map(|c| c.resolve_types(state))
            .transpose()?;
        if let Some(end) = end {
            end.assert_assignable_to(&Type::Integer, &self.span)?;
        }

        // Technically we could check for constant slicing here, TODO. There's reasonably high value to that.
        Ok(Type::Sequence(Sequence {
            elements: vec![],
            end_dynamic: Some(Box::new(inp_array.element_union())),
        }))
    }
}

fn get_array_index(arr: &[Value], idx: i64) -> usize {
    let len = arr.len() as i64;
    if idx >= len {
        len as usize
    } else if idx < 0 && ((-idx) > len) {
        0
    } else if idx < 0 {
        (len + idx) as usize
    } else {
        idx as usize
    }
}

function_def!(SumFunction, "sum", 1);

impl<'a: 'c, 'c> Expression<'a, 'c> for SumFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let arr = self.args[0].resolve(state)?;

        let inp_array = arr.as_array().ok_or_else(|| {
            TransformError::new_incorrect_type(
                "sum",
                "array",
                TransformError::value_desc(&arr),
                &self.span,
            )
        })?;

        let mut sum = JsonNumber::PosInteger(0);

        for it in inp_array {
            let number: JsonNumber = it
                .as_number()
                .ok_or_else(|| {
                    TransformError::new_incorrect_type(
                        "sum",
                        "number in array",
                        TransformError::value_desc(it),
                        &self.span,
                    )
                })?
                .into();

            sum = sum.try_add(number, &self.span)?;
        }

        Ok(ResolveResult::Owned(sum.try_into_json().ok_or_else(
            || {
                TransformError::new_conversion_failed(
                    "Failed to create json number from result of sum",
                    &self.span,
                )
            },
        )?))
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::expressions::types::TypeExecutionState<'c, '_>,
    ) -> Result<Type, TypeError> {
        let arr = self.args[0].resolve_types(state)?;
        let arr = arr.try_as_array(&self.span)?;

        let mut return_type = Type::Integer;
        for it in arr.all_elements() {
            let ty = it;
            if ty.is_float() && return_type.is_integer() {
                return_type = Type::Float;
            } else if !ty.is_integer() {
                ty.assert_assignable_to(&Type::number(), &self.span)?;
                return_type = Type::number();
            }
        }
        Ok(return_type)
    }
}

function_def!(ContainsFunction, "contains", 2);

impl<'a: 'c, 'c> Expression<'a, 'c> for ContainsFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let raw_list = self.args[0].resolve(state)?;
        let look_for = self.args[1].resolve(state)?;
        match raw_list.as_ref() {
            Value::Array(list) => {
                for i in list {
                    if i == look_for.as_ref() {
                        return Ok(ResolveResult::Owned(Value::Bool(true)));
                    }
                }

                Ok(ResolveResult::Owned(Value::Bool(false)))
            }
            Value::String(s) => {
                let look_for = look_for.try_as_string("contains", &self.span)?;

                Ok(ResolveResult::Owned(Value::Bool(
                    s.contains(look_for.as_ref()),
                )))
            }
            _ => Err(TransformError::new_incorrect_type(
                "contains",
                "array",
                TransformError::value_desc(&raw_list),
                &self.span,
            )),
        }
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::expressions::types::TypeExecutionState<'c, '_>,
    ) -> Result<Type, TypeError> {
        self.args[0].resolve_types(state)?;
        self.args[1].resolve_types(state)?;
        // Technically we could check if it is impossible here, but it's not that useful.
        Ok(Type::Boolean)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

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

    #[test]
    pub fn test_slice() {
        let expr = compile_expression(
            r#"{
            "s1": [1, 2, 3, 4].slice(1, 3),
            "s2": [].slice(15, 16),
            "s3": [1, 2, 3, 4].slice(-3),
            "s4": [1, 2, 3, 4].slice(0, -15),
            "s5": [1, 2, 3, 4].slice(0, 15),
            "s6": [1, 2, 3, 4].slice(0),
            "s7": [1, 2, 3, 4].slice(2, 1),
            "s8": [1, 2, 3, 4].slice(15),
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        assert_eq!(
            &Value::Array(vec![2.into(), 3.into()]),
            res.get("s1").unwrap()
        );
        assert_eq!(&Value::Array(vec![]), res.get("s2").unwrap());
        assert_eq!(
            &Value::Array(vec![2.into(), 3.into(), 4.into()]),
            res.get("s3").unwrap()
        );
        assert_eq!(&Value::Array(vec![]), res.get("s4").unwrap());
        assert_eq!(
            &Value::Array(vec![1.into(), 2.into(), 3.into(), 4.into()]),
            res.get("s5").unwrap()
        );
        assert_eq!(
            &Value::Array(vec![1.into(), 2.into(), 3.into(), 4.into()]),
            res.get("s6").unwrap()
        );
        assert_eq!(&Value::Array(vec![]), res.get("s7").unwrap());
        assert_eq!(&Value::Array(vec![]), res.get("s8").unwrap());
    }

    #[test]
    pub fn test_tail() {
        let expr = compile_expression(
            r#"{
            "v1": [1, 2, 3, 4, 5, 6].tail(),
            "v2": [1, 2, 3, 4].tail(2),
            "v3": [1, 2, 3, 4, 5, 6, 7].tail(1)
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let obj = res.as_object().unwrap();
        assert_eq!(3, obj.len());
        assert_eq!(6, obj.get("v1").unwrap().as_u64().unwrap());
        assert_eq!(
            &Value::Array(vec![3.into(), 4.into()]),
            obj.get("v2").unwrap()
        );
        assert_eq!(7, obj.get("v3").unwrap().as_u64().unwrap());
    }

    #[test]
    pub fn test_sum() {
        let expr = compile_expression("[1, 1, 1, 2, 2, 2].sum()", &[]).unwrap();

        let res = expr.run([]).unwrap();

        assert_eq!(9, res.as_u64().unwrap());
    }

    #[test]
    pub fn test_contains() {
        let expr = compile_expression(
            r#"{
                "t1": [1, 2, 3, 4].contains(4),
                "t2": [1, 2, 3, 4].contains(6),
                "t3": ["hey", "now"].contains("hey"),
                "t4": ["hey", "now"].contains("hey1"),
                "t5": [{"hello": "there"}, "now"].contains("hello"),
                "t6": [{"hello": "there"}, "now"].contains({"hello": "there"}),
                "t7": "hello there".contains("hello"),
                "t8": "goodbye".contains("hello"),
                "t9": "hell".contains("el"),
            }"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        assert_eq!(res.get("t1").unwrap().as_bool().unwrap(), true);
        assert_eq!(res.get("t2").unwrap().as_bool().unwrap(), false);
        assert_eq!(res.get("t3").unwrap().as_bool().unwrap(), true);
        assert_eq!(res.get("t4").unwrap().as_bool().unwrap(), false);
        assert_eq!(res.get("t5").unwrap().as_bool().unwrap(), false);
        assert_eq!(res.get("t6").unwrap().as_bool().unwrap(), true);
        assert_eq!(res.get("t7").unwrap().as_bool().unwrap(), true);
        assert_eq!(res.get("t8").unwrap().as_bool().unwrap(), false);
        assert_eq!(res.get("t9").unwrap().as_bool().unwrap(), true);
    }
}
