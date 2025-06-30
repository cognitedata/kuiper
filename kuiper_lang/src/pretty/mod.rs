use formatter::Formatter;
use utils::iter_line_spans;

pub use utils::PrettyError;

mod formatter;
mod utils;

/// Format a kuiper expression into a pretty printed string.
/// Returns an error if the input is not a valid kuiper expression.
///
/// This does not fold newlines, but it will remove spaces between tokens.
///
/// Note that this is just a best-effort formatter, designed to be quite conservative, but
/// apply proper indentation and spacing to the expression.
pub fn format_expression(input: &str) -> Result<String, PrettyError> {
    // This is probably not the perfect way to do this, but it works well enough.
    // We do a few separate things in a single pass:
    // 1. Re-indent based on certain tokens. Parentheses, brackets, braces, and postfix selector chains.
    // 2. Remove unnecessary whitespace between tokens, while preserving newlines.
    // 3. Add a single space between tokens where appropriate, e.g. around operators, commas, and colons.

    // Validate that the program parses, otherwise this will almost certainly fail in some other weird way,
    // or produce terrible results.
    crate::parse::ProgramParser::new().parse(crate::lexer::Lexer::new(input))?;

    Formatter::new(input, iter_line_spans(input).enumerate().peekable()).run()
}

#[cfg(test)]
mod tests {
    fn test_pretty_print(input: &str, expected: &str) {
        let result = super::format_expression(input).unwrap();
        println!("{result}");
        println!("{expected}");
        assert_eq!(result, expected);

        // Check that the result can be parsed back into a valid program.
        crate::parse::ProgramParser::new()
            .parse(crate::lexer::Lexer::new(&result))
            .expect("Formatted expression should be valid");
    }

    #[test]
    fn test_pretty_printing() {
        test_pretty_print(
            r#"
input.map(x=>
x + 1
)
        "#,
            r#"
input.map(x =>
    x + 1
)
"#,
        );

        test_pretty_print(
            r#"
input . map(x =>
/* There's a comment here */
      x+1
)"#,
            r#"
input.map(x =>
    /* There's a comment here */
    x + 1
)"#,
        );

        test_pretty_print(
            r#"
foo(1 +1).bar(2 + 2).baz(3* 3)
"#,
            r#"
foo(1 + 1).bar(2 + 2).baz(3 * 3)
"#,
        );

        // Yes this one ends up looking weird. It's a consequence of the fact that we don't fold newlines.
        // Essentially, the last unclosed indent token on each line is the one that is used to establish indentation.
        test_pretty_print(
            r#"
// Multiple indenting tokens on one line.
test().foo(bar(baz(
1),
    2),
3
) // Only indents until the last opening token is closed.
"#,
            r#"
// Multiple indenting tokens on one line.
test().foo(bar(baz(
    1),
2),
3
) // Only indents until the last opening token is closed.
"#,
        );

        test_pretty_print(
            r#"
// Big fancy object
{
"key": "val",
"key2":   [
1,
2,
3
],
"key3":{
"key4":"val4",
"key5": "val5",
"key6": [1,2, 3 ],
}
}"#,
            r#"
// Big fancy object
{
    "key": "val",
    "key2": [
        1,
        2,
        3
    ],
    "key3": {
        "key4": "val4",
        "key5": "val5",
        "key6": [1, 2, 3],
    }
}"#,
        );

        test_pretty_print(
            r#"
"     Multiline strings are preserved entirely, even if they end with whitespace.

      ".concat("foo")
"#,
            r#"
"     Multiline strings are preserved entirely, even if they end with whitespace.

      ".concat("foo")
"#,
        );

        test_pretty_print(
            r#"input
    .foo()
    .bar()
    .baz()
"#,
            r#"input
    .foo()
    .bar()
    .baz()
"#,
        );

        // Yes this does look a little funky, but I think it is correct.
        // It would look less weird if you indented the `x`, which is probably what a user would
        // want to do.
        test_pretty_print(
            r#"
input.map(x => x
    .foo()
    .bar()
    .baz()
    + 5
)"#,
            r#"
input.map(x => x
        .foo()
        .bar()
        .baz()
    + 5
)"#,
        );

        test_pretty_print(
            r#"
input.foo()
.bar(
    1 + 1
)
.baz(
1 + 1,
input
.bar(),
// Note that there are two layers of indentation here, due to postfix chains.
input
.baz(
1 + 1,
)
)
"#,
            r#"
input.foo()
    .bar(
        1 + 1
    )
    .baz(
        1 + 1,
        input
            .bar(),
        // Note that there are two layers of indentation here, due to postfix chains.
        input
            .baz(
                1 + 1,
            )
    )
"#,
        );

        test_pretty_print(
            r#"
// Very nested
[
{
(
"key"
):
(
(
(
(
"val"
)
)
)
)
}
]
        "#,
            r#"
// Very nested
[
    {
        (
            "key"
        ):
        (
            (
                (
                    (
                        "val"
                    )
                )
            )
        )
    }
]
"#,
        );

        test_pretty_print(
            r#"
// Comments everywhere
(/*Hello*/1/* there*/+2/* there are spaces   */*/* between */3/*these */)
"#,
            r#"
// Comments everywhere
( /* Hello */ 1 /* there */ + 2 /* there are spaces */ * /* between */ 3 /* these */ )
"#,
        );

        test_pretty_print(
            r#"
/*This is a multiline


comment
    Leading whitespace is preserved, but trailing whitespace is removed.
*/
1//There must be a single token for the expression to be valid.
"#,
            r#"
/* This is a multiline


comment
    Leading whitespace is preserved, but trailing whitespace is removed.
*/
1 // There must be a single token for the expression to be valid.
"#,
        );
    }

    #[test]
    fn test_pretty_printing_fancy() {
        // Test a big, real expression.
        test_pretty_print(
            r#"if input.Header.MessageType == "DATA_REPORT" || input.Header.MessageType == "ONDEMAND_DATA_REPORT" {
input.MessagePayload.pairs().flatmap(deviceOrEdgeApp =>
deviceOrEdgeApp.value.flatmap(device =>
device.DataTags.flatmap(dataTags =>
concat("ts:iot-agora:", input.Header.Company, ":", input.Header.Project, ":", input.Header.EdgeDeviceId, ":",
if (deviceOrEdgeApp.key == "Devices", "device", "app"), ":", device.Name, ":", dataTags.TagName).if_value(external_id =>
[{
"type": "time_series",
"dataSetId": 6124285521957219,
"externalId": external_id,
"name": concat(device.Name, ":", dataTags.TagName),
"isString": dataTags.DataType == "BYTES_VALUE" || dataTags.DataType == "BOOLEAN_VALUE",
"metadata": join(coalesce(dataTags.Metadata, {}), { "deviceName": device.Name }, {"tagName": dataTags.TagName})
},
{
"type": "datapoint",
"timestamp": dataTags.Timestamp,
"value": try_float(dataTags.Value, dataTags.Value),
"externalId": external_id,
"status": if (dataTags.Quality == 0, "Good", "Bad")
}])
)
)
)
} else if input.Header.MessageType == "ALARM" {
if input.MessagePayload.State == "SET" {
{
"type": "event",
"dataSetId": 6124285521957219,
"externalId": concat("ts:iot-agora:", input.Header.Company, ":", input.Header.Project, ":", input.Header.EdgeDeviceId, ":",
input.MessagePayload.SourceType, ":", input.MessagePayload.SourceName, ":", input.MessagePayload.TagName, ":", input.MessagePayload.EventId),
"startTime": input.Header.Timestamp,
"eventType": input.MessagePayload.CrossedThresholdName,
"subtype": input.MessagePayload.State,
"metadata": {
"SetTagValue": input.MessagePayload.TagValue,
"SetCrossedThresholdValue": input.MessagePayload.CrossedThresholdValue
}
}
} else if input.MessagePayload.State == "CLEAR" {
{
"type": "event",
"dataSetId": 6124285521957219,
"externalId": concat("ts:iot-agora:", input.Header.Company, ":", input.Header.Project, ":", input.Header.EdgeDeviceId, ":",
input.MessagePayload.SourceType, ":", input.MessagePayload.SourceName, ":", input.MessagePayload.TagName, ":", input.MessagePayload.EventId),
"endTime": input.Header.Timestamp,
"eventType": input.MessagePayload.CrossedThresholdName,
"subtype": input.MessagePayload.State,
"metadata": {
"ClearTagValue": input.MessagePayload.TagValue,
"ClearCrossedThresholdValue": input.MessagePayload.CrossedThresholdValue
}
}
}
}
"#,
            r#"if input.Header.MessageType == "DATA_REPORT" || input.Header.MessageType == "ONDEMAND_DATA_REPORT" {
    input.MessagePayload.pairs().flatmap(deviceOrEdgeApp =>
        deviceOrEdgeApp.value.flatmap(device =>
            device.DataTags.flatmap(dataTags =>
                concat("ts:iot-agora:", input.Header.Company, ":", input.Header.Project, ":", input.Header.EdgeDeviceId, ":",
                    if (deviceOrEdgeApp.key == "Devices", "device", "app"), ":", device.Name, ":", dataTags.TagName).if_value(external_id =>
                    [{
                        "type": "time_series",
                        "dataSetId": 6124285521957219,
                        "externalId": external_id,
                        "name": concat(device.Name, ":", dataTags.TagName),
                        "isString": dataTags.DataType == "BYTES_VALUE" || dataTags.DataType == "BOOLEAN_VALUE",
                        "metadata": join(coalesce(dataTags.Metadata, {}), { "deviceName": device.Name }, { "tagName": dataTags.TagName })
                    },
                    {
                        "type": "datapoint",
                        "timestamp": dataTags.Timestamp,
                        "value": try_float(dataTags.Value, dataTags.Value),
                        "externalId": external_id,
                        "status": if (dataTags.Quality == 0, "Good", "Bad")
                    }])
            )
        )
    )
} else if input.Header.MessageType == "ALARM" {
    if input.MessagePayload.State == "SET" {
        {
            "type": "event",
            "dataSetId": 6124285521957219,
            "externalId": concat("ts:iot-agora:", input.Header.Company, ":", input.Header.Project, ":", input.Header.EdgeDeviceId, ":",
                input.MessagePayload.SourceType, ":", input.MessagePayload.SourceName, ":", input.MessagePayload.TagName, ":", input.MessagePayload.EventId),
            "startTime": input.Header.Timestamp,
            "eventType": input.MessagePayload.CrossedThresholdName,
            "subtype": input.MessagePayload.State,
            "metadata": {
                "SetTagValue": input.MessagePayload.TagValue,
                "SetCrossedThresholdValue": input.MessagePayload.CrossedThresholdValue
            }
        }
    } else if input.MessagePayload.State == "CLEAR" {
        {
            "type": "event",
            "dataSetId": 6124285521957219,
            "externalId": concat("ts:iot-agora:", input.Header.Company, ":", input.Header.Project, ":", input.Header.EdgeDeviceId, ":",
                input.MessagePayload.SourceType, ":", input.MessagePayload.SourceName, ":", input.MessagePayload.TagName, ":", input.MessagePayload.EventId),
            "endTime": input.Header.Timestamp,
            "eventType": input.MessagePayload.CrossedThresholdName,
            "subtype": input.MessagePayload.State,
            "metadata": {
                "ClearTagValue": input.MessagePayload.TagValue,
                "ClearCrossedThresholdValue": input.MessagePayload.CrossedThresholdValue
            }
        }
    }
}
"#,
        );
    }
}
