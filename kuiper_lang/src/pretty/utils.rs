#![expect(unused)] // Temp, until this is actually finished.
use logos::Span;

use crate::lex::{LexerError, Token};
use crate::ParseError;

pub(super) fn iter_line_spans(input: &str) -> impl Iterator<Item = Span> + '_ {
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
pub(super) enum IndentNodeKind {
    Parenthesis,
    Bracket,
    Brace,
    Initial,
}

pub(super) struct IndentNode {
    kind: IndentNodeKind,
    line: usize,
    caused_indent: bool,
    has_postfix_chain: bool,
}

pub(super) fn raw_token(input: &str, span: Span) -> &str {
    &input[span.start..span.end]
}

pub(super) fn to_indent_token(tok: &Token) -> Option<IndentNodeKind> {
    match tok {
        Token::OpenParenthesis => Some(IndentNodeKind::Parenthesis),
        Token::OpenBracket => Some(IndentNodeKind::Bracket),
        Token::OpenBrace => Some(IndentNodeKind::Brace),
        _ => None,
    }
}

pub(super) fn check_closing_token(
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
            | (IndentNodeKind::Brace, Token::CloseBrace) => Ok(Some(node)),
            _ => Err(PrettyError::Pretty(
                format!("Expected closing token for {:?}", node.kind),
                span.clone(),
            )),
        }
    } else {
        Err(PrettyError::Pretty(
            "Unexpected closing token".to_string(),
            span.clone(),
        ))
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

pub(super) fn trim_inter_token_whitespace(
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

    " ".repeat(expected_spaces)
}

/// Make sure a comment has a single space before // or /* and after */
/// Also, if the comment is multiline, remove any trailing whitespace.
pub(super) fn prettify_comment(comment: &str) -> String {
    if let Some(stripped) = comment.strip_prefix("//") {
        format!("// {}", stripped.trim_start())
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
                let inner = line[..(line.len() - 2)].trim_end();
                // Trim leading whitespace only if there is no inner content.
                if inner.is_empty() {
                    line = "*/".to_owned();
                } else {
                    line = format!("{inner} */").to_owned();
                }
            }

            output.push_str(&line);
            if has_newline {
                output.push('\n');
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use logos::{Logos, SpannedIter};

    use super::*;

    fn tokens(input: &str) -> SpannedIter<'_, Token> {
        Token::lexer(input).spanned()
    }

    #[test]
    fn test_iter_line_spans() {
        let input = "line1\nline2  \n  line3";
        let spans: Vec<Span> = iter_line_spans(input).collect();
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0], Span { start: 0, end: 6 });
        assert_eq!(spans[1], Span { start: 6, end: 14 });
        assert_eq!(spans[2], Span { start: 14, end: 21 });
    }

    #[test]
    fn test_get_raw_tokens() {
        let input = "x + 5 - input.test";
        let raw_tokens: Vec<_> = tokens(input)
            .map(|(_tok, span)| raw_token(input, span))
            .collect();
        assert_eq!(raw_tokens, vec!["x", "+", "5", "-", "input", ".", "test"]);
    }

    #[test]
    fn test_inter_token_whitespace() {
        fn token_test(input: &str, expected: &str) {
            let mut output = String::new();
            let mut last_token: Option<Token> = None;
            let mut last_end = 0;
            for (token, span) in tokens(input) {
                let token = token.unwrap();
                output.push_str(&trim_inter_token_whitespace(
                    &input[last_end..span.start],
                    last_token.as_ref(),
                    Some(&token),
                ));
                last_end = span.end;
                let raw = raw_token(input, span);
                output.push_str(&raw);
                last_token = Some(token);
            }
            output.push_str(&trim_inter_token_whitespace(
                &input[last_end..],
                last_token.as_ref(),
                None,
            ));

            assert_eq!(output, expected);
        }

        token_test("x+5*6/7", "x + 5 * 6 / 7");
        token_test("input  .foo(a,  b) *  5", "input.foo(a, b) * 5");
        token_test("  input.foo(a=>b + 1)", "input.foo(a => b + 1)");
        // This method removes all non-newline whitespace, re-indentation is done later.
        token_test(
            r#"input.foo(
   a + 1,
   b - 1
    )"#,
            r#"input.foo(
a + 1,
b - 1
)"#,
        );

        token_test(
            "if a ==2{b + 1}else {b -   1 }",
            "if a == 2 { b + 1 } else { b - 1 }",
        );
        token_test("{  }", "{}");
        token_test("[   ]", "[]");
        token_test(r#"{ "a" :5 }"#, r#"{ "a": 5 }"#);
    }

    #[test]
    fn test_indent_tokens() {
        let mut tokens = tokens(r#"input.foo(a, { "b": [1, 2, 3] })"#);
        let mut stack = Vec::new();
        let mut removed = Vec::new();
        for (tok, span) in tokens {
            let tok = tok.unwrap();
            if let Some(kind) = to_indent_token(&tok) {
                stack.push(IndentNode {
                    kind,
                    line: 0,
                    caused_indent: false,
                    has_postfix_chain: false,
                });
            } else if let Some(node) = check_closing_token(&mut stack, &tok, &span).unwrap() {
                removed.push(node);
            }
        }

        assert_eq!(removed[0].kind, IndentNodeKind::Bracket);
        assert_eq!(removed[1].kind, IndentNodeKind::Brace);
        assert_eq!(removed[2].kind, IndentNodeKind::Parenthesis);
        assert_eq!(stack.len(), 0);
        assert_eq!(removed.len(), 3);
    }

    #[test]
    fn test_prettify_comment() {
        fn test_comment(input: &str, expected: &str) {
            let output = prettify_comment(input);
            assert_eq!(output, expected);
        }
        // Note that these should only ever contain the comment token, prettify_comment is designed to consume the
        // entire comment token, and nothing else.
        test_comment("//test", "// test");
        test_comment("// test   ", "// test");
        test_comment("/*test*/", "/* test */");
        // Leading whitespace is preserved, but trailing whitespace is removed.
        test_comment(
            r#"/*
    test
  */"#,
            "/*
    test
*/",
        );
        test_comment(
            r#"/*
    test
  foo */"#,
            "/*
    test
  foo */",
        );
    }
}
