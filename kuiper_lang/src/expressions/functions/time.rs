use crate::{
    expressions::{
        base::{get_number_from_value, get_string_from_value},
        functions::FunctionExpression,
        Expression, ResolveResult,
    },
    TransformError,
};

use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use serde_json::{Number, Value};

function_def!(ToUnixTimeFunction, "to_unix_timestamp", 2, Some(3));

impl<'a: 'c, 'c> Expression<'a, 'c> for ToUnixTimeFunction {
    fn resolve(
        &'a self,
        state: &crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let dat = self.args.get(0).unwrap().resolve(state)?;
        let val = get_string_from_value(Self::INFO.name, &dat, &self.span, state.id)?;
        let val_ref = val.as_ref();
        let fmt = self.args.get(1).unwrap().resolve(state)?;
        let fmt_val = get_string_from_value(Self::INFO.name, &fmt, &self.span, state.id)?;
        let fmt_ref = fmt_val.as_ref();
        // If the format string contains timezone, create a timestamp with timezone directly
        if fmt_ref.contains("%z") {
            let time = DateTime::parse_from_str(val_ref, fmt_ref).map_err(|e| {
                TransformError::new_conversion_failed(
                    format!("Failed to convert '{val_ref}' to datetime using '{fmt_ref}': {e}"),
                    &self.span,
                    state.id,
                )
            })?;
            Ok(ResolveResult::Owned(Value::Number(Number::from(
                time.timestamp_millis(),
            ))))
        // If not, first create a naive datetime from the input...
        } else {
            let time = NaiveDateTime::parse_from_str(val_ref, fmt_ref).map_err(|e| {
                TransformError::new_conversion_failed(
                    format!("Failed to convert '{val_ref}' to datetime using '{fmt_ref}': {e}"),
                    &self.span,
                    state.id,
                )
            })?;
            // Then, if there is a third "offset" argument, use that to construct an offset datetime.
            if self.args.len() == 3 {
                let off_val = get_number_from_value(
                    Self::INFO.name,
                    self.args.get(2).unwrap().resolve(state)?.as_ref(),
                    &self.span,
                    state.id,
                )?
                .try_as_i64(&self.span, state.id)?;
                if off_val < i32::MIN as i64 || off_val > i32::MAX as i64 {
                    return Err(TransformError::new_invalid_operation(
                        format!("Offset {off_val} out of bounds for to_unix_timestamp"),
                        &self.span,
                        state.id,
                    ));
                }

                let offset = FixedOffset::east_opt(off_val as i32).ok_or_else(|| {
                    TransformError::new_invalid_operation(
                        format!("Offset {off_val} out of bounds for to_unix_timestamp"),
                        &self.span,
                        state.id,
                    )
                })?;
                match offset.from_local_datetime(&time) {
                    chrono::LocalResult::Single(x) => Ok(ResolveResult::Owned(Value::Number(
                        Number::from(x.timestamp_millis()),
                    ))),
                    _ => Err(TransformError::new_conversion_failed(
                        "Failed to apply timezone offset to timestamp".to_string(),
                        &self.span,
                        state.id,
                    )),
                }
            } else {
                Ok(ResolveResult::Owned(Value::Number(Number::from(
                    time.timestamp_millis(),
                ))))
            }
        }
    }
}

function_def!(NowFunction, "now", 0);

impl<'a: 'c, 'c> Expression<'a, 'c> for NowFunction {
    const IS_DETERMINISTIC: bool = false;
    fn resolve(
        &'a self,
        _state: &crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let res = Utc::now().timestamp_millis();
        Ok(ResolveResult::Owned(Value::Number(res.into())))
    }
}
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::{json, Value};

    use crate::{compile_expression, Program};

    #[test]
    pub fn test_time_conversion() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "tostring",
                "inputs": ["input"],
                "transform": r#"{
                    "t1": to_unix_timestamp(input.v1, '%Y-%m-%d %H:%M:%S'),
                    "t2": to_unix_timestamp(input.v2, '%Y-%m-%d %H:%M:%S %z'),
                    "t12": to_unix_timestamp(input.v1, '%Y-%m-%d %H:%M:%S', 3600),
                    "t13": to_unix_timestamp(input.v1, '%Y-%m-%d %H:%M:%S%.f', -3600),
                    "t3": to_unix_timestamp(input.v3, '%Y %b %d %H:%M'),
                    "t4": to_unix_timestamp(input.v4, '%Y %b %d %H:%M %z')
                }"#,
                "type": "map"
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program
            .execute(&json!({
                "v1": "1970-01-02 00:00:00",
                "v2": "1970-01-02 01:00:00 +0100",
                "v3": "1970 Jan 02 00:00",
                "v4": "1970 Jan 02 01:00 +0100"
            }))
            .unwrap();

        assert_eq!(res.len(), 1);
        let val = res.first().unwrap();
        assert_eq!(86400000, val.get("t1").unwrap().as_i64().unwrap());
        assert_eq!(86400000, val.get("t2").unwrap().as_i64().unwrap());
        assert_eq!(82800000, val.get("t12").unwrap().as_i64().unwrap());
        assert_eq!(90000000, val.get("t13").unwrap().as_i64().unwrap());
        assert_eq!(86400000, val.get("t3").unwrap().as_i64().unwrap());
        assert_eq!(86400000, val.get("t4").unwrap().as_i64().unwrap());
    }

    #[test]
    pub fn test_now() {
        let program = Program::compile(
            serde_json::from_value(json!([{
                "id": "tostring",
                "inputs": ["input"],
                "transform": r#"now()"#,
                "type": "map"
            }]))
            .unwrap(),
        )
        .unwrap();

        let res = program.execute(&Value::Null).unwrap();
        assert!(res.first().unwrap().as_i64().unwrap() > 0);
    }

    #[test]
    pub fn test_now_const() {
        let r = compile_expression("now()", &mut HashMap::new(), "test").unwrap();
        assert_eq!("now()", r.to_string());
    }
}
