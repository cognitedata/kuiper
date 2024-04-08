use serde_json::Value;

use crate::{
    expressions::{Expression, ResolveResult},
    TransformError,
};

function_def!(IfFunction, "if", 2, Some(3));

impl<'a: 'c, 'c> Expression<'a, 'c> for IfFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let cond = self.args.first().unwrap().resolve(state)?.as_bool();

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

impl<'a: 'c, 'c> Expression<'a, 'c> for CaseFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let lhs = &self.args[0];
        let lhs = lhs.resolve(state)?;
        // If length is odd, no else arg, so 5 / 2 - (1 - 1) = 2 groups
        // If length is even, else arg, so 6 / 2 - (1 - 0) = 2 groups
        let pairs = (self.args.len() / 2) - (1 - self.args.len() % 2);
        let result = if lhs.is_number() {
            self.resolve_number(state, lhs, pairs)?
        } else if lhs.is_string() {
            self.resolve_string(state, lhs, pairs)?
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

function_def!(AnyFunction, "any", 1);

impl<'a: 'c, 'c> Expression<'a, 'c> for AnyFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        match &self.args[0].resolve(state)?.as_ref() {
            Value::Array(list) => {
                for i in list {
                    if i.as_bool().is_some_and(|x| x) {
                        return Ok(ResolveResult::Owned(Value::Bool(true)));
                    }
                }

                Ok(ResolveResult::Owned(Value::Bool(false)))
            }

            Value::Object(object) => {
                for i in object.values() {
                    if i.as_bool().is_some_and(|x| x) {
                        return Ok(ResolveResult::Owned(Value::Bool(true)));
                    }
                }
                Ok(ResolveResult::Owned(Value::Bool(false)))
            }

            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to any",
                "array or object",
                TransformError::value_desc(&x),
                &self.span,
            )),
        }
    }
}

function_def!(AllFunction, "all", 1);

impl<'a: 'c, 'c> Expression<'a, 'c> for AllFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        match &self.args[0].resolve(state)?.as_ref() {
            Value::Array(list) => {
                for i in list {
                    if i.as_bool().is_some_and(|x| !x) {
                        return Ok(ResolveResult::Owned(Value::Bool(false)));
                    }
                }

                Ok(ResolveResult::Owned(Value::Bool(true)))
            }

            Value::Object(object) => {
                for i in object.values() {
                    if i.as_bool().is_some_and(|x| !x) {
                        return Ok(ResolveResult::Owned(Value::Bool(false)));
                    }
                }
                Ok(ResolveResult::Owned(Value::Bool(true)))
            }

            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to all",
                "array or object",
                TransformError::value_desc(&x),
                &self.span,
            )),
        }
    }
}

impl CaseFunction {
    fn resolve_generic<'a: 'b, 'b>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'b, '_>,
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

    fn resolve_number<'a: 'b, 'b>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'b, '_>,
        lhs: ResolveResult<'a>,
        pairs: usize,
    ) -> Result<Option<usize>, TransformError> {
        let lhs_val = lhs.try_as_number("case", &self.span)?;
        for idx in 0..pairs {
            let rhs = self.args[idx * 2 + 1]
                .resolve(state)?
                .try_as_number("case", &self.span)?;
            if lhs_val.eq(rhs, &self.span) {
                return Ok(Some(idx * 2 + 2));
            }
        }
        Ok(None)
    }

    fn resolve_string<'a: 'b, 'b>(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'b, '_>,
        lhs: ResolveResult<'a>,
        pairs: usize,
    ) -> Result<Option<usize>, TransformError> {
        let lhs_val = lhs.try_into_string("case", &self.span)?;
        for idx in 0..pairs {
            let cmp = self.args[idx * 2 + 1].resolve(state)?;
            let rhs = cmp;
            let rhs_val = rhs.try_into_string("case", &self.span)?;
            if lhs_val == rhs_val {
                return Ok(Some(idx * 2 + 2));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;

    #[test]
    pub fn test_simple_if() {
        let expr = compile_expression(
            r#"{
            "t1": if(true, 'test'),
            "t2": if(1 == 2, 'test2'),
            "t3": if(1 > 2, 'test3', 'test4')
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        assert_eq!("test", res.get("t1").unwrap().as_str().unwrap());
        assert!(res.get("t2").unwrap().is_null());
        assert_eq!("test4", res.get("t3").unwrap().as_str().unwrap());
    }

    #[test]
    pub fn test_case() {
        let expr = compile_expression(
            r#"{
            "t1": case('foo', 'bar', 1, 'baz', 2, 'foo', 3),
            "t2": case('nope', 'bar', 1, 'baz', 2, 'foo', 3),
            "t3": case('foo', 'bar', 1, 'baz', 2, 4)
        }"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        assert_eq!(3, res.get("t1").unwrap().as_u64().unwrap());
        assert!(res.get("t2").unwrap().is_null());
        assert_eq!(4, res.get("t3").unwrap().as_u64().unwrap());
    }

    #[test]
    pub fn test_any() {
        let expr = compile_expression(
            r#"{
                "t1": any([false, false, true]),
                "t2": any([false, false, false]),
                "t3": any([true, true, true]),
                "t4": any({"k1": true, "k2": false}),
                "t5": any({"k1": false, "k2": false}),
                "t6": any({"k1": true, "k2": true}),
            }"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();
        assert_eq!(res.get("t1").unwrap().as_bool().unwrap(), true);
        assert_eq!(res.get("t2").unwrap().as_bool().unwrap(), false);
        assert_eq!(res.get("t3").unwrap().as_bool().unwrap(), true);
        assert_eq!(res.get("t4").unwrap().as_bool().unwrap(), true);
        assert_eq!(res.get("t5").unwrap().as_bool().unwrap(), false);
        assert_eq!(res.get("t6").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    pub fn test_all() {
        let expr = compile_expression(
            r#"{
                "t1": all([false, false, true]),
                "t2": all([false, false, false]),
                "t3": all([true, true, true]),
                "t4": all({"k1": true, "k2": false}),
                "t5": all({"k1": false, "k2": false}),
                "t6": all({"k1": true, "k2": true}),
            }"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();
        assert_eq!(res.get("t1").unwrap().as_bool().unwrap(), false);
        assert_eq!(res.get("t2").unwrap().as_bool().unwrap(), false);
        assert_eq!(res.get("t3").unwrap().as_bool().unwrap(), true);
        assert_eq!(res.get("t4").unwrap().as_bool().unwrap(), false);
        assert_eq!(res.get("t5").unwrap().as_bool().unwrap(), false);
        assert_eq!(res.get("t6").unwrap().as_bool().unwrap(), true);
    }
}
