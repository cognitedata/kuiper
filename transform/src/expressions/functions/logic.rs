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
