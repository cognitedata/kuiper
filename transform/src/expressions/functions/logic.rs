use serde_json::Value;

use crate::{
    expressions::{
        base::{get_boolean_from_value, get_number_from_value, get_string_from_value},
        Expression, ResolveResult,
    },
    TransformError,
};

function_def!(IfFunction, "if", 2, Some(3));

impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for IfFunction {
    fn resolve(
        &'a self,
        state: &'b crate::expressions::ExpressionExecutionState<'c, 'b>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let cond_raw = self.args.first().unwrap().resolve(state)?;
        let cond = get_boolean_from_value(cond_raw.as_ref());

        if cond {
            Ok(self.args.get(1).unwrap().resolve(state)?)
        } else if self.args.len() == 2 {
            Ok(ResolveResult::Owned(Value::Null))
        } else {
            Ok(self.args.get(2).unwrap().resolve(state)?)
        }
    }
}

function_def!(CaseFunction, "case", 3, None);

impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for CaseFunction {
    fn resolve(
        &'a self,
        state: &'b crate::expressions::ExpressionExecutionState<'c, 'b>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let lhs = &self.args[0];
        let lhs = lhs.resolve(state)?;
        // If length is odd, no else arg, so 5 / 2 - (1 - 1) = 2 groups
        // If length is even, else arg, so 6 / 2 - (1 - 0) = 2 groups
        let pairs = (self.args.len() / 2) - (1 - self.args.len() % 2);
        let result = if lhs.is_number() {
            self.resolve_number(state, &lhs, pairs)?
        } else if lhs.is_string() {
            self.resolve_string(state, &lhs, pairs)?
        } else {
            self.resolve_generic(state, &lhs, pairs)?
        };

        if let Some(idx) = result {
            Ok(self.args[idx].resolve(state)?)
        } else if self.args.len() % 2 == 0 {
            Ok(self.args[self.args.len() - 1].resolve(state)?)
        } else {
            Ok(ResolveResult::Owned(Value::Null))
        }
    }
}

impl CaseFunction {
    fn resolve_generic<'a>(
        &'a self,
        state: &'a crate::expressions::ExpressionExecutionState,
        lhs: &Value,
        pairs: usize,
    ) -> Result<Option<usize>, TransformError> {
        for idx in 0..pairs {
            let cmp = self.args[idx * 2 + 1].resolve(state)?;
            if lhs == cmp.as_ref() {
                return Ok(Some(idx * 2 + 2));
            }
        }
        Ok(None)
    }

    fn resolve_number<'a>(
        &'a self,
        state: &'a crate::expressions::ExpressionExecutionState,
        lhs: &Value,
        pairs: usize,
    ) -> Result<Option<usize>, TransformError> {
        let lhs_val = get_number_from_value("case", lhs, &self.span, state.id)?;
        for idx in 0..pairs {
            let cmp = self.args[idx * 2 + 1].resolve(state)?;
            let rhs = cmp.as_ref();
            let rhs_val = get_number_from_value("case", rhs, &self.span, state.id)?;
            if lhs_val.eq(rhs_val, &self.span, state.id) {
                return Ok(Some(idx * 2 + 2));
            }
        }
        Ok(None)
    }

    fn resolve_string<'a>(
        &'a self,
        state: &'a crate::expressions::ExpressionExecutionState,
        lhs: &Value,
        pairs: usize,
    ) -> Result<Option<usize>, TransformError> {
        let lhs_val = get_string_from_value("case", lhs, &self.span, state.id)?;
        for idx in 0..pairs {
            let cmp = self.args[idx * 2 + 1].resolve(state)?;
            let rhs = cmp;
            let rhs_val = get_string_from_value("case", &rhs, &self.span, state.id)?;
            if lhs_val == rhs_val {
                return Ok(Some(idx * 2 + 2));
            }
        }
        Ok(None)
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
                "transform": r#"{
                    "t1": if(true, 'test'),
                    "t2": if(1 == 2, 'test2'),
                    "t3": if(1 > 2, 'test3', 'test4')
                }"#,
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

    #[test]
    pub fn test_case() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "tostring",
                "inputs": [],
                "transform": r#"{
                    "t1": case('foo', 'bar', 1, 'baz', 2, 'foo', 3),
                    "t2": case('nope', 'bar', 1, 'baz', 2, 'foo', 3),
                    "t3": case('foo', 'bar', 1, 'baz', 2, 4)
                }"#
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program.execute(&Value::Null).unwrap();

        assert_eq!(res.len(), 1);
        let val = res.first().unwrap();
        assert_eq!(3, val.get("t1").unwrap().as_u64().unwrap());
        assert!(val.get("t2").unwrap().is_null());
        assert_eq!(4, val.get("t3").unwrap().as_u64().unwrap());
    }
}
