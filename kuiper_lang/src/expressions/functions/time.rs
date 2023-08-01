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
        let val = get_string_from_value(Self::INFO.name, &dat, &self.span)?;
        let val_ref = val.as_ref();
        let fmt = self.args.get(1).unwrap().resolve(state)?;
        let fmt_val = get_string_from_value(Self::INFO.name, &fmt, &self.span)?;
        let fmt_ref = fmt_val.as_ref();
        // If the format string contains timezone, create a timestamp with timezone directly
        if fmt_ref.contains("%z") {
            let time = DateTime::parse_from_str(val_ref, fmt_ref).map_err(|e| {
                TransformError::new_conversion_failed(
                    format!("Failed to convert '{val_ref}' to datetime using '{fmt_ref}': {e}"),
                    &self.span,
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
                )
            })?;
            // Then, if there is a third "offset" argument, use that to construct an offset datetime.
            if self.args.len() == 3 {
                let off_val = get_number_from_value(
                    Self::INFO.name,
                    self.args.get(2).unwrap().resolve(state)?.as_ref(),
                    &self.span,
                )?
                .try_as_i64(&self.span)?;
                if off_val < i32::MIN as i64 || off_val > i32::MAX as i64 {
                    return Err(TransformError::new_invalid_operation(
                        format!("Offset {off_val} out of bounds for to_unix_timestamp"),
                        &self.span,
                    ));
                }

                let offset = FixedOffset::east_opt(off_val as i32).ok_or_else(|| {
                    TransformError::new_invalid_operation(
                        format!("Offset {off_val} out of bounds for to_unix_timestamp"),
                        &self.span,
                    )
                })?;
                match offset.from_local_datetime(&time) {
                    chrono::LocalResult::Single(x) => Ok(ResolveResult::Owned(Value::Number(
                        Number::from(x.timestamp_millis()),
                    ))),
                    _ => Err(TransformError::new_conversion_failed(
                        "Failed to apply timezone offset to timestamp".to_string(),
                        &self.span,
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

function_def!(FormatTimestampFunction, "format_timestamp", 2);

impl<'a: 'c, 'c> Expression<'a, 'c> for FormatTimestampFunction {
    fn resolve(
        &'a self,
        state: &crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let timestamp = self.args[0].resolve(state)?;
        let format = self.args[1].resolve(state)?;

        let timestamp_num =
            get_number_from_value("format_timestamp", timestamp.as_ref(), &self.span)?
                .try_as_i64(&self.span)?;
        let format_str = get_string_from_value("format_timestamp", format.as_ref(), &self.span)?;

        let datetime = Utc.timestamp_millis_opt(timestamp_num).single().ok_or(
            TransformError::new_conversion_failed(
                format!("Failed to convert {timestamp_num} to datetime"),
                &self.span,
            ),
        )?;

        Ok(ResolveResult::Owned(Value::String(
            datetime.format(&format_str).to_string(),
        )))
    }
}

#[cfg(test)]
mod tests {

    use serde_json::json;

    use crate::compile_expression;

    #[test]
    pub fn test_time_conversion() {
        let expr = compile_expression(
            r#"{
            "t1": to_unix_timestamp(input.v1, '%Y-%m-%d %H:%M:%S'),
            "t2": to_unix_timestamp(input.v2, '%Y-%m-%d %H:%M:%S %z'),
            "t12": to_unix_timestamp(input.v1, '%Y-%m-%d %H:%M:%S', 3600),
            "t13": to_unix_timestamp(input.v1, '%Y-%m-%d %H:%M:%S%.f', -3600),
            "t3": to_unix_timestamp(input.v3, '%Y %b %d %H:%M'),
            "t4": to_unix_timestamp(input.v4, '%Y %b %d %H:%M %z')
        }"#,
            &["input"],
        )
        .unwrap();

        let inp = json!({
            "v1": "1970-01-02 00:00:00",
            "v2": "1970-01-02 01:00:00 +0100",
            "v3": "1970 Jan 02 00:00",
            "v4": "1970 Jan 02 01:00 +0100"
        });
        let res = expr.run([&inp]).unwrap();

        assert_eq!(86400000, res.get("t1").unwrap().as_i64().unwrap());
        assert_eq!(86400000, res.get("t2").unwrap().as_i64().unwrap());
        assert_eq!(82800000, res.get("t12").unwrap().as_i64().unwrap());
        assert_eq!(90000000, res.get("t13").unwrap().as_i64().unwrap());
        assert_eq!(86400000, res.get("t3").unwrap().as_i64().unwrap());
        assert_eq!(86400000, res.get("t4").unwrap().as_i64().unwrap());
    }

    #[test]
    pub fn test_now() {
        let expr = compile_expression("now()", &[]).unwrap();

        let res = expr.run([].iter()).unwrap();
        assert!(res.as_i64().unwrap() > 0);
    }

    #[test]
    pub fn test_now_const() {
        let r = compile_expression("now()", &[]).unwrap();
        assert_eq!("now()", r.to_string());
    }

    #[test]
    pub fn test_time_format() {
        let expr = compile_expression(
            r#"{
                "s1": format_timestamp(1690873155301, "%Y-%m-%d %H:%M:%S"),
                "s2": format_timestamp(to_unix_timestamp("2023-08-01 13:42:13", "%Y-%m-%d %H:%M:%S"), "%Y-%m-%d %H:%M:%S"),
                "s3": format_timestamp(0, "%H:%M:%S %Y/%m - %d"),
                "s4": format_timestamp(1417176009000, "%a %b %e %T %Y"),
            }"#,
            &[],
        )
        .unwrap();
        let result = expr.run([].iter()).unwrap();

        assert_eq!(
            "2023-08-01 06:59:15",
            result.get("s1").unwrap().as_str().unwrap()
        );
        assert_eq!(
            "2023-08-01 13:42:13",
            result.get("s2").unwrap().as_str().unwrap()
        );
        assert_eq!(
            "00:00:00 01/01 - 1970",
            result.get("s3").unwrap().as_str().unwrap()
        );
        assert_eq!(
            "Fri Nov 28 12:00:09 2014",
            result.get("s4").unwrap().as_str().unwrap()
        );
    }
}
