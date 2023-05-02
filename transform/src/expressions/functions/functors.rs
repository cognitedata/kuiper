use serde_json::{Map, Value};

use crate::{
    expressions::{Expression, ExpressionType, ResolveResult},
    with_temp_values, ParserError, TransformError,
};

use super::LambdaAcceptFunction;

function_def!(MapFunction, "map", 2, lambda);

impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for MapFunction {
    fn resolve(
        &'a self,
        state: &'b crate::expressions::ExpressionExecutionState<'c, 'b>,
    ) -> Result<crate::expressions::ResolveResult<'c>, TransformError> {
        let source = self.args[0].resolve(state)?;
        let nargs = match self.args[1].as_ref() {
            ExpressionType::Lambda(l) => l.input_names.len(),
            _ => 1,
        };

        match source.as_ref() {
            Value::Array(x) => {
                if nargs > 1 {
                    return Err(TransformError::new_invalid_operation(
                        "Mapping over an array requires a function with a single argument"
                            .to_string(),
                        &self.span,
                        state.id,
                    ));
                }

                let mut res = Vec::with_capacity(x.len());
                let mut inner = state.get_temporary_clone(1);
                for val in x {
                    let r = with_temp_values!(inner, inner_state, &[val], {
                        self.args[1].resolve(&inner_state)
                    })?;
                    res.push(r);
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            Value::Object(x) => {
                let mut res = Map::with_capacity(x.len());
                let pairs: Vec<_> = x
                    .iter()
                    .map(|(key, val)| (Value::String(key.clone()), val))
                    .collect();

                if nargs == 1 {
                    let mut inner = state.get_temporary_clone(1);
                    for (key, val) in pairs.iter() {
                        let r = with_temp_values!(inner, inner_state, &[*val], {
                            self.args[1].resolve(&inner_state)
                        })?;
                        res.insert(key.as_str().unwrap().to_string(), r);
                    }
                } else {
                    let mut inner = state.get_temporary_clone(2);
                    for (key, val) in pairs.iter() {
                        let r = with_temp_values!(inner, inner_state, &[key, *val], {
                            self.args[1].resolve(&inner_state)
                        })?;
                        res.insert(key.as_str().unwrap().to_string(), r);
                    }
                }
                Ok(ResolveResult::Owned(Value::Object(res)))
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to map",
                "array or object",
                TransformError::value_desc(x),
                &self.span,
                state.id,
            )),
        }
    }
}

impl LambdaAcceptFunction for MapFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
    ) -> Result<(), crate::ParserError> {
        if idx != 1 {
            return Err(crate::ParserError::unexpected_lambda(&lambda.span));
        }
        let nargs = lambda.input_names.len();
        if nargs != 1 && nargs != 2 {
            return Err(ParserError::n_function_args(
                lambda.span.clone(),
                "map takes a function with one or two arguments",
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
    pub fn test_simple_map() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "map",
                "inputs": [],
                "transform": r#"map([1, 2, 3, 4], (i) => pow($i, 2))"#
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program.execute(&Value::Null).unwrap();

        assert_eq!(res.len(), 1);
        let val = res.first().unwrap();
        let val_arr = val.as_array().unwrap();
        assert_eq!(4, val_arr.len());
        assert_eq!(val_arr.get(0).unwrap().as_f64().unwrap(), 1.0);
        assert_eq!(val_arr.get(1).unwrap().as_f64().unwrap(), 4.0);
        assert_eq!(val_arr.get(2).unwrap().as_f64().unwrap(), 9.0);
        assert_eq!(val_arr.get(3).unwrap().as_f64().unwrap(), 16.0);
    }

    #[test]
    pub fn test_map_object() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "map",
                "inputs": [],
                "transform": r#"{
                    "r1": map({ "a1": 1, "a2": 2, "a3": 3, "a4": 4 }, (i) => pow($i, 2)),
                    "r2": map({ "a1": 1, "a2": 2, "a3": 3, "a4": 4 }, (key, val) => concat($key, ":", pow($val, 2)))
                }"#
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program.execute(&Value::Null).unwrap();

        assert_eq!(res.len(), 1);
        let val = res.first().unwrap();
        let val_obj = val.as_object().unwrap();
        assert_eq!(2, val_obj.len());
        let obj = val_obj.get("r1").unwrap().as_object().unwrap();
        assert_eq!(obj.get("a1").unwrap().as_f64().unwrap(), 1.0);
        assert_eq!(obj.get("a2").unwrap().as_f64().unwrap(), 4.0);
        assert_eq!(obj.get("a3").unwrap().as_f64().unwrap(), 9.0);
        assert_eq!(obj.get("a4").unwrap().as_f64().unwrap(), 16.0);
        let obj = val_obj.get("r2").unwrap().as_object().unwrap();
        assert_eq!(obj.get("a1").unwrap().as_str().unwrap(), "a1:1.0");
        assert_eq!(obj.get("a2").unwrap().as_str().unwrap(), "a2:4.0");
        assert_eq!(obj.get("a3").unwrap().as_str().unwrap(), "a3:9.0");
        assert_eq!(obj.get("a4").unwrap().as_str().unwrap(), "a4:16.0");
    }
}
