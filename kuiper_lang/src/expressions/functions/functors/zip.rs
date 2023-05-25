use std::borrow::Cow;

use serde_json::Value;

use crate::{
    compiler::BuildError,
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    program::NULL_CONST,
};

function_def!(ZipFunction, "zip", 3, None, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for ZipFunction {
    fn resolve(
        &'a self,
        state: &crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let mut sources = Vec::with_capacity(self.args.len() - 1);
        let mut output_len = 0;
        for source in self.args.iter().take(self.args.len() - 1) {
            let r = source.resolve(state)?;
            let r = match r {
                std::borrow::Cow::Borrowed(r) => r.as_array().map(Cow::Borrowed),
                std::borrow::Cow::Owned(r) => match r {
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
            res.push(func.call(&state, &chunk)?.into_owned());
        }

        Ok(ResolveResult::Owned(Value::Array(res)))
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
    use serde_json::{json, Value};

    use crate::Program;

    #[test]
    pub fn test_zip_two() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "map",
                "inputs": [],
                "transform": r#"zip([1, 2, 3], [4, 5, 6, 7], (v1, v2) => { "v1": v1, "v2": v2 })"#
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program.execute(&Value::Null).unwrap();
        assert_eq!(res.len(), 1);
        let val = res.first().unwrap();
        let val_arr = val.as_array().unwrap();
        assert_eq!(4, val_arr.len());
        let obj = val_arr.get(0).unwrap().as_object().unwrap();
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
}
