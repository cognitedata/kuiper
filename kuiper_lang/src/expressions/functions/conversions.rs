use crate::expressions::numbers::JsonNumber;
use crate::expressions::{Expression, ExpressionExecutionState, ResolveResult};
use crate::types::Type;
use crate::TransformError;
use serde_json::Value;

function_def!(TryFloatFunction, "try_float", 2);

/// Replaces ',' with '.' and trims any ' ' and '_' in the string.
/// Returns None if the string is not ASCII.
fn replace_for_parse(mut inp: String) -> Option<String> {
    // SAFETY: We terminate early if we encounter a non-ascii byte,
    // meaning that we never create invalid UTF-8
    let inner = unsafe { inp.as_mut_vec() };
    let mut offset = 0;
    // Efficiently replace characters in string for replace
    // Normal replace allocates a new vector, which is generally necessary when
    // replacing in strings, since the string might grow.
    // We know that the string cannot grow, it can only shrink,
    // so we can replace-in-place, by just shifting characters backwards for
    // each character we should skip that we encounter.
    for i in 0..inner.len() {
        let c = inner[i];
        // This is necessary for safety! Removing this is a source of UB,
        // as then we might accidentally create invalid UTF-8.
        if !c.is_ascii() {
            return None;
        }
        match c {
            b',' => inner[i - offset] = b'.',
            b' ' | b'_' => offset += 1,
            _ => inner[i - offset] = c,
        }
    }
    inp.truncate(inp.len() - offset);
    Some(inp)
}

impl Expression for TryFloatFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, TransformError> {
        self.args[0].resolve(state)?.map_clone_string(
            state,
            |s, state| match replace_for_parse(s).map(|r| r.parse::<f64>()) {
                Some(Ok(value)) => Ok(ResolveResult::Owned(value.into())),
                _ => Ok(self.args[1].resolve(state)?),
            },
            |v, state| match v {
                Value::Number(n) => Ok(ResolveResult::Owned(n.as_f64().unwrap().into())),
                _ => Ok(self.args[1].resolve(state)?),
            },
        )
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let a1 = self.args[0].resolve_types(state)?;
        let a2 = self.args[1].resolve_types(state)?;
        if a1.is_float() {
            return Ok(a1);
        }
        if !a1.is_assignable_to(&Type::number().union_with(Type::String)) {
            return Ok(a2);
        }
        Ok(Type::Float.union_with(a2))
    }
}

function_def!(TryIntFunction, "try_int", 2);

impl Expression for TryIntFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, TransformError> {
        self.args[0].resolve(state)?.map_clone_string(
            state,
            |s, state| match replace_for_parse(s).map(|r| r.parse::<i64>()) {
                Some(Ok(value)) => Ok(ResolveResult::Owned(value.into())),
                _ => Ok(self.args[1].resolve(state)?),
            },
            |v, state| match v {
                Value::Number(n) => Ok(ResolveResult::Owned(
                    JsonNumber::from(n)
                        .try_cast_integer(&self.span)?
                        .try_into_json()
                        .unwrap(),
                )),
                _ => Ok(self.args[1].resolve(state)?),
            },
        )
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let a1 = self.args[0].resolve_types(state)?;
        let a2 = self.args[1].resolve_types(state)?;
        if a1.is_integer() {
            return Ok(a1);
        }
        if !a1.is_assignable_to(&Type::number().union_with(Type::String)) {
            return Ok(a2);
        }
        Ok(Type::Integer.union_with(a2))
    }
}

function_def!(TryBoolFunction, "try_bool", 2);

impl Expression for TryBoolFunction {
    fn resolve<'a>(
        &'a self,
        state: &mut ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let r = match self.args[0].resolve(state)?.as_ref() {
            Value::Bool(b) => *b,
            Value::String(s) => match s.trim_matches('"').trim().to_lowercase().parse::<bool>() {
                Ok(value) => value,
                Err(_) => return self.args[1].resolve(state),
            },
            _ => return self.args[1].resolve(state),
        };
        Ok(ResolveResult::Owned(Value::from(r)))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let a1 = self.args[0].resolve_types(state)?;
        let a2 = self.args[1].resolve_types(state)?;
        if a1.is_boolean() {
            return Ok(a1);
        }
        if !a1.is_assignable_to(&Type::Boolean.union_with(Type::String)) {
            return Ok(a2);
        }
        Ok(Type::Boolean.union_with(a2))
    }
}

#[cfg(test)]
mod tests {
    use crate::{compile_expression, types::Type};
    use serde_json::json;

    #[test]
    pub fn test_try_float() {
        let exp = compile_expression(
            r#"{
            "test1": try_float("4.2", 7),
            "test2": try_float(input.floatValue, 7),
            "test3": try_float(input.stringValue, 2.5),
            "test4": try_float("1.3e5", 4),
            "test5": try_float(6.2, 4),
            "test6": try_float("not a float", 6.3),
            "test7": try_float("   4.2  ", 6.3),
            "test8": try_float("not a float æøåæøå", "also not a float"),
            "test9": try_float("5,2", "9"),
            "test10": try_float("1 234,56", "9"),
            "test11": try_float("1_000", "6"),
        }"#,
            &["input"],
        )
        .unwrap();

        let input = json!({"floatValue": "5.3", "stringValue": "heyoo"});

        let result = exp.run([&input]).unwrap();

        assert_eq!(4.2, result.get("test1").unwrap().as_f64().unwrap());
        assert_eq!(5.3, result.get("test2").unwrap().as_f64().unwrap());
        assert_eq!(2.5, result.get("test3").unwrap().as_f64().unwrap());
        assert_eq!(1.3e5, result.get("test4").unwrap().as_f64().unwrap());
        assert_eq!(6.2, result.get("test5").unwrap().as_f64().unwrap());
        assert_eq!(6.3, result.get("test6").unwrap().as_f64().unwrap());
        assert_eq!(4.2, result.get("test7").unwrap().as_f64().unwrap());
        assert_eq!(
            "also not a float",
            result.get("test8").unwrap().as_str().unwrap()
        );
        assert_eq!(5.2, result.get("test9").unwrap().as_f64().unwrap());
        assert_eq!(1234.56, result.get("test10").unwrap().as_f64().unwrap());
        assert_eq!(1000.0, result.get("test11").unwrap().as_f64().unwrap());
    }

    #[test]
    pub fn test_try_int() {
        let exp = compile_expression(
            r#"{
            "test1": try_int("4.2", 7),
            "test2": try_int(input.intValue, 7),
            "test3": try_int(input.stringValue, 2.5),
            "test4": try_int("1.3e5", 4),
            "test5": try_int(6, 4),
            "test6": try_int("not a int", 8),
            "test7": try_int("   4  ", 6.3),
            "test8": try_int("not a int", "also not a int"),
        }"#,
            &["input"],
        )
        .unwrap();

        let input = json!({"intValue": "5", "stringValue": "heyoo"});

        let result = exp.run([&input]).unwrap();

        assert_eq!(7, result.get("test1").unwrap().as_i64().unwrap());
        assert_eq!(5, result.get("test2").unwrap().as_i64().unwrap());
        assert_eq!(2.5, result.get("test3").unwrap().as_f64().unwrap());
        assert_eq!(4, result.get("test4").unwrap().as_i64().unwrap());
        assert_eq!(6, result.get("test5").unwrap().as_i64().unwrap());
        assert_eq!(8, result.get("test6").unwrap().as_i64().unwrap());
        assert_eq!(4, result.get("test7").unwrap().as_i64().unwrap());
        assert_eq!(
            "also not a int",
            result.get("test8").unwrap().as_str().unwrap()
        );
    }

    #[test]
    pub fn test_try_bool() {
        let exp = compile_expression(
            r#"{
            "test1": try_bool("true", false),
            "test2": try_bool("True", false),
            "test3": try_bool(input.boolValue, false),
            "test4": try_bool(input.stringValue, false),
            "test5": try_bool(true, false),
            "test6": try_bool("not a bool", false),
            "test7": try_bool("   TRUE  ", 6),
            "test8": try_bool("not a bool", "also not a bool"),
        }"#,
            &["input"],
        )
        .unwrap();

        let input = json!({"boolValue": "true", "stringValue": "heyoo"});

        let result = exp.run([&input]).unwrap();

        assert!(result.get("test1").unwrap().as_bool().unwrap());
        assert!(result.get("test2").unwrap().as_bool().unwrap());
        assert!(result.get("test3").unwrap().as_bool().unwrap());
        assert!(!result.get("test4").unwrap().as_bool().unwrap());
        assert!(result.get("test5").unwrap().as_bool().unwrap());
        assert!(!result.get("test6").unwrap().as_bool().unwrap());
        assert!(result.get("test7").unwrap().as_bool().unwrap());
        assert_eq!(
            "also not a bool",
            result.get("test8").unwrap().as_str().unwrap()
        );
    }

    #[test]
    fn test_try_float_types() {
        let exp = compile_expression(r#"try_float(input, "default")"#, &["input"]).unwrap();
        let t = exp.run_types([Type::String]).unwrap();
        assert_eq!(t, Type::Float.union_with(Type::from_const("default")));

        let t = exp.run_types([Type::from_const(5.5)]).unwrap();
        assert_eq!(t, Type::from_const(5.5));

        let t = exp.run_types([Type::null()]).unwrap();
        assert_eq!(t, Type::from_const("default"));
    }

    #[test]
    fn test_try_int_types() {
        let exp = compile_expression(r#"try_int(input, "default")"#, &["input"]).unwrap();
        let t = exp.run_types([Type::String]).unwrap();
        assert_eq!(t, Type::Integer.union_with(Type::from_const("default")));

        let t = exp.run_types([Type::from_const(5)]).unwrap();
        assert_eq!(t, Type::from_const(5));

        let t = exp.run_types([Type::null()]).unwrap();
        assert_eq!(t, Type::from_const("default"));
    }

    #[test]
    fn test_try_bool_types() {
        let exp = compile_expression(r#"try_bool(input, "default")"#, &["input"]).unwrap();
        let t = exp.run_types([Type::String]).unwrap();
        assert_eq!(t, Type::Boolean.union_with(Type::from_const("default")));

        let t = exp.run_types([Type::from_const(true)]).unwrap();
        assert_eq!(t, Type::from_const(true));

        let t = exp.run_types([Type::null()]).unwrap();
        assert_eq!(t, Type::from_const("default"));
    }
}
