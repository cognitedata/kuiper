use crate::expressions::{Expression, ExpressionExecutionState, ResolveResult};
use crate::TransformError;
use serde_json::Value;

function_def!(TryFloatFunction, "try_float", 2);

impl<'a: 'c, 'c> Expression<'a, 'c> for TryFloatFunction {
    fn resolve(
        &'a self,
        state: &ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        match self.args[0]
            .resolve(state)?
            .to_string()
            .trim_matches('"')
            .replace(' ', "")
            .replace(',', ".")
            .parse::<f64>()
        {
            Ok(value) => Ok(ResolveResult::Owned(Value::from(value))),
            Err(_) => Ok(self.args[1].resolve(state)?),
        }
    }
}

function_def!(TryIntFunction, "try_int", 2);

impl<'a: 'c, 'c> Expression<'a, 'c> for TryIntFunction {
    fn resolve(
        &'a self,
        state: &ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        match self.args[0]
            .resolve(state)?
            .to_string()
            .trim_matches('"')
            .trim()
            .to_string()
            .parse::<i64>()
        {
            Ok(value) => Ok(ResolveResult::Owned(Value::from(value))),
            Err(_) => Ok(self.args[1].resolve(state)?),
        }
    }
}

function_def!(TryBoolFunction, "try_bool", 2);

impl<'a: 'c, 'c> Expression<'a, 'c> for TryBoolFunction {
    fn resolve(
        &'a self,
        state: &ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        match self.args[0]
            .resolve(state)?
            .to_string()
            .trim_matches('"')
            .trim()
            .to_lowercase()
            .parse::<bool>()
        {
            Ok(value) => Ok(ResolveResult::Owned(Value::from(value))),
            Err(_) => Ok(self.args[1].resolve(state)?),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_expression;
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
            "test8": try_float("not a float", "also not a float"),
            "test9": try_float("5,2", "9"),
            "test10": try_float("1 234,56", "9"),
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
}
