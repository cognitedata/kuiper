use serde_json::Value;

use crate::expressions::{base::get_boolean_from_value, Expression, ResolveResult};

function_def!(IfFunction, "if", 2, Some(3));

impl<'a> Expression<'a> for IfFunction {
    fn resolve(
        &'a self,
        state: &'a crate::expressions::ExpressionExecutionState,
    ) -> Result<crate::expressions::ResolveResult<'a>, crate::TransformError> {
        let cond_raw = self.args.first().unwrap().resolve(state)?;
        let cond = get_boolean_from_value(cond_raw.as_ref());

        if cond {
            Ok(self.args.get(1).unwrap().resolve(state)?)
        } else {
            if self.args.len() == 2 {
                Ok(ResolveResult::Value(Value::Null))
            } else {
                Ok(self.args.get(2).unwrap().resolve(state)?)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::Program;

    #[test]
    pub fn test_simple_if() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "tostring",
                "inputs": [],
                "transform": {
                    "t1": "if(true, 'test')",
                    "t2": "if(1 == 2, 'test2')",
                    "t3": "if(1 > 2, 'test3', 'test4')"
                },
                "type": "map"
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program.execute(&Value::Null).unwrap();

        assert_eq!(res.len(), 1);
        let val = res.first().unwrap();
        assert_eq!("test", val.get("t1").unwrap().as_str().unwrap());
        assert!(val.get("t2").unwrap().is_null());
        assert_eq!("test4", val.get("t3").unwrap().as_str().unwrap());
    }
}
