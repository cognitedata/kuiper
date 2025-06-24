use logos::{Logos, Span};

use crate::lex::{LexerError, Token};
use crate::ParseError;

fn iter_line_spans(input: &str) -> impl Iterator<Item = Span> + '_ {
    let mut position = 0;
    input.split_inclusive('\n').map(move |line| {
        let span = Span {
            start: position,
            end: position + line.len(),
        };
        position += line.len();
        span
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndentNodeKind {
    Parenthesis,
    Bracket,
    Brace,
    Initial,
}

struct IndentNode {
    kind: IndentNodeKind,
    line: usize,
    caused_indent: bool,
    in_postfix_chain: Option<bool>,
}

fn raw_token(input: &str, span: Span) -> &str {
    &input[span.start..span.end]
}

fn to_indent_token(tok: &Token) -> Option<IndentNodeKind> {
    match tok {
        Token::OpenParenthesis => Some(IndentNodeKind::Parenthesis),
        Token::OpenBracket => Some(IndentNodeKind::Bracket),
        Token::OpenBrace => Some(IndentNodeKind::Brace),
        _ => None,
    }
}

fn check_closing_token(
    stack: &mut Vec<IndentNode>,
    tok: &Token,
    span: &Span,
) -> Result<Option<IndentNode>, PrettyError> {
    if !matches!(
        tok,
        Token::CloseParenthesis | Token::CloseBracket | Token::CloseBrace
    ) {
        return Ok(None);
    }

    if let Some(node) = stack.pop() {
        match (node.kind, tok) {
            (IndentNodeKind::Parenthesis, Token::CloseParenthesis)
            | (IndentNodeKind::Bracket, Token::CloseBracket)
            | (IndentNodeKind::Brace, Token::CloseBrace) => {
                return Ok(Some(node));
            }
            _ => {
                return Err(PrettyError::Pretty(
                    format!("Expected closing token for {:?}", node.kind),
                    span.clone(),
                ));
            }
        }
    } else {
        return Err(PrettyError::Pretty(
            "Unexpected closing token".to_string(),
            span.clone(),
        ));
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PrettyError {
    #[error("Failed to parse input: {0}")]
    Parser(#[from] ParseError),
    #[error("Pretty printing failed: {0}")]
    Pretty(String, Span),
}

impl From<LexerError> for PrettyError {
    fn from(err: LexerError) -> Self {
        PrettyError::Parser(ParseError::User { error: err })
    }
}

fn trim_inter_token_whitespace(
    ws: &str,
    last_token: Option<&Token>,
    current_token: Option<&Token>,
) -> String {
    // If there are newlines, just strip any other whitespace, keeping just the newlines.
    // A newline is always a valid separator. We do not fold, so we never remove any newlines.
    if ws.contains('\n') {
        return ws.replace(|c: char| c.is_whitespace() && c != '\n', "");
    }

    // In this case there are no newlines, so we return the _expected_ whitespace.
    let expected_spaces = match (last_token, current_token) {
        (None, _) | (_, None) => 0, // No previous token, no whitespace.
        (Some(Token::Operator(_)), _) | (Some(_), Some(Token::Operator(_))) => 1, // Operators are surrounded by spaces.
        (Some(Token::Comma), Some(Token::CloseBracket) | Some(Token::CloseParenthesis)) => 0, // No space before closing tokens, except for braces.
        (Some(Token::Comma), _) => 1, // Otherwise, commas are followed by a space.
        // Some special tokens are always followed by a space.
        (Some(Token::DefineEqual), _) | (_, Some(Token::DefineEqual)) => 1, // Define equal is always followed by and preceeded by a space.
        (Some(Token::Arrow), _) | (_, Some(Token::Arrow)) => 1, // Arrow is always followed by and preceeded by a space.
        (Some(Token::If), _) => 1,                              // If is always followed by a space.
        (Some(Token::Else), _) | (_, Some(Token::Else)) => 1, // Else is always followed by and preceeded by a space.
        (_, Some(Token::Comment)) | (Some(Token::Comment), _) => 1, // Comments are always preceded by a space.
        (Some(Token::Colon), _) => 1, // Colon is always followed by a space.
        (Some(Token::Not), _) => 1, // Not is always followed by a space. Since the only valid token before this is `is`, it will
        // also be preceeded by a space.
        // A bunch of tokens followed by a brace may be an if condition, so we expect a space.
        (
            Some(
                Token::CloseParenthesis
                | Token::CloseBracket
                | Token::Float(_)
                | Token::Integer(_)
                | Token::Boolean(_)
                | Token::Identifier(_)
                | Token::String(_)
                | Token::TypeLiteral(_),
            ),
            Some(Token::OpenBrace),
        ) => 1,
        (Some(Token::OpenBrace), Some(Token::CloseBrace)) => 0, // Empty objects do not have spaces inside them.
        (Some(Token::OpenBrace), _) | (_, Some(Token::CloseBrace)) => 1, // Spaces inside objects.
        _ => 0,                                                 // Otherwise, no space is expected.
    };

    return " ".repeat(expected_spaces);
}

/// Make sure a comment has a single space before // or /* and after */
/// Also, if the comment is multiline, remove any trailing whitespace.
fn prettify_comment(comment: &str) -> String {
    if comment.starts_with("//") {
        format!("// {}", &comment[2..].trim_start())
            .trim_end()
            .to_owned()
    } else {
        let mut output = String::new();
        for line in comment.lines() {
            let has_newline = !line.ends_with("*/");
            let mut line = line.trim_end().to_owned();
            if line.starts_with("/*") {
                line = format!("/* {}", &line[2..].trim_start())
                    .trim_end()
                    .to_owned();
            }
            if line.ends_with("*/") {
                line = format!("{} */", line[..(line.len() - 2)].trim_end())
                    .trim_start()
                    .to_owned();
            }

            output.push_str(&line);
            if has_newline {
                output.push('\n');
            }
        }
        output
    }
}

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

    // We use the raw tokenizer, since we want comment tokens. This does mean we lex the input twice,
    // but it probably isn't a big deal, considering the normal size of kuiper expressions.
    let tokens = Token::lexer(input).spanned();

    let mut output = String::new();
    let mut stack: Vec<IndentNode> = Vec::new();
    stack.push(IndentNode {
        kind: IndentNodeKind::Initial,
        line: 0,
        caused_indent: false,
        in_postfix_chain: None,
    });

    let mut lines = iter_line_spans(input).enumerate().peekable();

    let mut tokens_on_line = 0;
    let mut indent = 0;
    let mut indent_on_line = 0;
    let mut last_end = 0;
    let mut last_token: Option<Token> = None;

    for (tok, tok_span) in tokens {
        let tok = tok?;
        let line;
        // First, find the line number for the current token.
        loop {
            // This should be impossible.
            let Some((line_num, line_span)) = lines.peek() else {
                return Err(PrettyError::Pretty(
                    "Token outside of input".to_string(),
                    tok_span,
                ));
            };

            // Check if the start of the token is on the current line.
            if line_span.start <= tok_span.start && line_span.end > tok_span.start {
                line = *line_num;
                tokens_on_line += 1;
                if last_end >= line_span.start {
                    tokens_on_line += 1;
                }
                break;
            }

            lines.next();
            tokens_on_line = 0;
            if indent_on_line > 0 {
                indent += 4;
                indent_on_line = 0;
            }
        }

        // Is the token an opening indent token?
        if let Some(kind) = to_indent_token(&tok) {
            // Only the last indent token on each line is responsible for the indent level.
            if let Some(n) = stack.last_mut() {
                if n.line == line {
                    n.caused_indent = false;
                }
            }
            stack.push(IndentNode {
                kind: kind,
                line,
                caused_indent: true,
                in_postfix_chain: None,
            });
            indent_on_line += 1;
        }
        // Is the token a closing indent token?
        if let Some(node) = check_closing_token(&mut stack, &tok, &tok_span)? {
            if node.line == line {
                // If the closing token is on the same line, we just reduce the count of indent tokens on the current line.
                indent_on_line -= 1;
            } else {
                // Else, we need to reduce the indent level, if the original node caused an indent.
                if node.caused_indent {
                    indent -= 4;
                }
            }
        }
        // First, push any whitespace between the last token and the current one.
        output.push_str(&trim_inter_token_whitespace(
            &input[last_end..tok_span.start],
            last_token.as_ref(),
            Some(&tok),
        ));

        // If the token is a period, we give it extra indentation.
        // This is a bit of a hack, and can have some slightly weird effects if the code is already badly
        // formatted, but it works decently well.
        let mut current_indent = indent;

        // Certain tokens can cause us to enter or exit a postfix chain, check those.
        match tok {
            Token::Period => {
                if let Some(n) = stack.last_mut() {
                    if n.in_postfix_chain.is_none() {
                        if n.line != line {
                            n.in_postfix_chain = Some(true);
                        } else {
                            n.in_postfix_chain = Some(false);
                        }
                    }
                    if n.in_postfix_chain.unwrap_or_default() {
                        // If we are in a postfix chain, we indent the next token.
                        current_indent += 4;
                    }
                }
            }
            Token::Operator(_) | Token::Colon | Token::SemiColon | Token::Comma => {
                if let Some(n) = stack.last_mut() {
                    n.in_postfix_chain = None;
                }
            }
            _ => (),
        };

        // If the token is the first on the line, push indent.
        if tokens_on_line == 1 {
            output.push_str(&" ".repeat(current_indent));
        }

        // Now, push the raw token to the output.
        last_end = tok_span.end;
        if matches!(tok, Token::Comment) {
            output.push_str(&prettify_comment(raw_token(input, tok_span)));
        } else {
            output.push_str(raw_token(input, tok_span));
        }
        last_token = Some(tok);
    }

    output.push_str(&trim_inter_token_whitespace(
        &input[last_end..],
        last_token.as_ref(),
        None,
    ));

    Ok(output)
}

#[cfg(test)]
mod tests {
    fn test_pretty_print(input: &str, expected: &str) {
        let result = super::format_expression(input).unwrap();
        println!("{result}");
        println!("{expected}");
        assert_eq!(result, expected);
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
