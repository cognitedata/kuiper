use std::borrow::Cow;

use serde_json::Value;

use crate::{
    compiler::BuildError,
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    types::{Array, Type},
    NULL_CONST,
};

function_def!(ZipFunction, "zip", 3, None, lambda);

impl Expression for ZipFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'a, '_>,
    ) -> Result<crate::expressions::ResolveResult<'a>, crate::TransformError> {
        let mut sources = Vec::with_capacity(self.args.len() - 1);
        let mut output_len = 0;
        for source in self.args.iter().take(self.args.len() - 1) {
            let r = source.resolve(state)?;
            let r = match r {
                ResolveResult::Borrowed(r) => r.as_array().map(Cow::Borrowed),
                ResolveResult::Owned(r) => match r {
                    Value::Array(a) => Some(Cow::Owned(a)),
                    _ => None,
                },
            };
            if let Some(r) = &r {
                if r.len() > output_len {
                    output_len = r.len();
                }
            }

            sources.push(r);
        }

        let func = self.args.last().unwrap();

        let mut res = Vec::with_capacity(output_len);
        for idx in 0..output_len {
            let mut chunk = Vec::with_capacity(self.args.len() - 1);
            for s in &sources {
                let v = s.as_ref().and_then(|v| v.get(idx)).unwrap_or(&NULL_CONST);
                chunk.push(v);
            }
            res.push(func.call(state, &chunk)?.into_owned());
        }

        Ok(ResolveResult::Owned(Value::Array(res)))
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let mut sources = Vec::with_capacity(self.args.len() - 1);
        let mut known_output_len = 0;
        let mut num_end_dynamic = 0;
        for source in self.args.iter().take(self.args.len() - 1) {
            let r = source.resolve_types(state)?;
            if let Ok(r) = r.try_as_array(&self.span) {
                if known_output_len < r.elements.len() {
                    known_output_len = r.elements.len();
                }
                if r.end_dynamic.is_some() {
                    num_end_dynamic += 1;
                }
                sources.push(Some(r));
            } else {
                sources.push(None);
            }
        }

        let func = self.args.last().unwrap();

        let mut res_types = Vec::with_capacity(known_output_len);
        for idx in 0..known_output_len {
            let mut chunk = Vec::with_capacity(self.args.len() - 1);
            for s in &sources {
                if let Some(arr) = s {
                    chunk.push(
                        arr.index_into(idx)
                            .or_else(|| arr.end_dynamic.clone().map(|v| v.nullable()))
                            .unwrap_or_else(Type::null),
                    );
                } else {
                    chunk.push(Type::null());
                }
            }
            let chunk_ref = chunk.iter().collect::<Vec<&Type>>();
            let elem = func.call_types(state, &chunk_ref)?;
            res_types.push(elem);
        }

        if num_end_dynamic > 0 {
            let mut chunk = Vec::with_capacity(self.args.len() - 1);
            for s in &sources {
                if let Some(arr) = s {
                    let ty = arr
                        .end_dynamic
                        .clone()
                        .map(|v| *v)
                        .unwrap_or_else(Type::null);
                    // If there's just one end_dynamic, we can use it as is,
                    // but if there's more, any array might end before the others.
                    if num_end_dynamic > 1 {
                        chunk.push(ty.nullable());
                    } else {
                        chunk.push(ty);
                    }
                } else {
                    chunk.push(Type::null());
                }
            }
            let chunk_ref = chunk.iter().collect::<Vec<&Type>>();
            let end_dynamic = func.call_types(state, &chunk_ref)?;
            Ok(Type::Array(Array {
                elements: res_types,
                end_dynamic: Some(Box::new(end_dynamic)),
            }))
        } else {
            Ok(Type::Array(Array {
                elements: res_types,
                end_dynamic: None,
            }))
        }
    }
}

impl LambdaAcceptFunction for ZipFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
        num_args: usize,
    ) -> Result<(), BuildError> {
        if idx != num_args - 1 {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }
        if lambda.input_names.len() != num_args - 1 {
            return Err(BuildError::n_function_args(
                lambda.span.clone(),
                "zip takes a function with as many arguments as the zip function itself",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::{
        compile_expression,
        types::{Array, Object, Type},
    };

    #[test]
    pub fn test_zip_two() {
        let expr = compile_expression(
            r#"zip([1, 2, 3], [4, 5, 6, 7], (v1, v2) => { "v1": v1, "v2": v2 })"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();
        let val_arr = res.as_array().unwrap();
        assert_eq!(4, val_arr.len());
        let obj = val_arr.first().unwrap().as_object().unwrap();
        assert_eq!(1, obj.get("v1").unwrap().as_u64().unwrap());
        assert_eq!(4, obj.get("v2").unwrap().as_u64().unwrap());
        let obj = val_arr.get(1).unwrap().as_object().unwrap();
        assert_eq!(2, obj.get("v1").unwrap().as_u64().unwrap());
        assert_eq!(5, obj.get("v2").unwrap().as_u64().unwrap());
        let obj = val_arr.get(2).unwrap().as_object().unwrap();
        assert_eq!(3, obj.get("v1").unwrap().as_u64().unwrap());
        assert_eq!(6, obj.get("v2").unwrap().as_u64().unwrap());
        let obj = val_arr.get(3).unwrap().as_object().unwrap();
        assert_eq!(&Value::Null, obj.get("v1").unwrap());
        assert_eq!(7, obj.get("v2").unwrap().as_u64().unwrap());
    }

    #[test]
    fn test_zip_types() {
        let expr = compile_expression(
            "zip(input1, input2, (v1, v2) => { \"v1\": v1, \"v2\": v2 })",
            &["input1", "input2"],
        )
        .unwrap();
        let res = expr
            .run_types([
                Type::array_of_type(Type::Integer),
                Type::array_of_type(Type::String),
            ])
            .unwrap();
        assert_eq!(
            res,
            Type::array_of_type(Type::Object(
                Object::default()
                    .with_field("v1", Type::Integer.nullable())
                    .with_field("v2", Type::String.nullable())
            ))
        );

        let res = expr
            .run_types([
                Type::Array(Array {
                    elements: vec![Type::from_const(1), Type::from_const(2)],
                    end_dynamic: Some(Box::new(Type::from_const(3))),
                }),
                Type::Array(Array {
                    elements: vec![Type::from_const(4)],
                    end_dynamic: None,
                }),
            ])
            .unwrap();
        assert_eq!(
            res,
            Type::Array(Array {
                elements: vec![
                    Type::Object(
                        Object::default()
                            .with_field("v1", Type::from_const(1))
                            .with_field("v2", Type::from_const(4))
                    ),
                    Type::Object(
                        Object::default()
                            .with_field("v1", Type::from_const(2))
                            .with_field("v2", Type::null())
                    ),
                ],
                end_dynamic: Some(Box::new(Type::Object(
                    Object::default()
                        .with_field("v1", Type::from_const(3))
                        .with_field("v2", Type::null())
                )))
            })
        )
    }
}
