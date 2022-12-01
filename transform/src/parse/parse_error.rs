use logos::Span;

use crate::lexer::Token;

#[derive(Debug)]
pub enum ParserError {
    EmptyExpression(ParserErrorData),
    UnexpectedSymbol(ParserErrorData),
    ExpectedSymbol(ParserErrorData),
    InvalidExpression(ParserErrorData),
    InvalidToken(ParserErrorData),
    NFunctionArgs(ParserErrorData),
    ExpectExpression(ParserErrorData),
}

#[derive(Debug)]
pub struct ParserErrorData {
    pub position: Span,
    pub detail: Option<String>,
}

impl ParserError {
    pub(crate) fn empty_expression(position: Span) -> Self {
        Self::EmptyExpression(ParserErrorData {
            position,
            detail: None,
        })
    }
    pub(crate) fn unexpected_symbol(position: Span, symbol: Token) -> Self {
        match symbol {
            Token::Error => Self::invalid_token(position),
            _ => Self::UnexpectedSymbol(ParserErrorData {
                position,
                detail: Some(format!("Unexpected symbol {}", symbol)),
            }),
        }
    }
    pub(crate) fn unrecognized_function(position: Span, symbol: &str) -> Self {
        Self::UnexpectedSymbol(ParserErrorData {
            position,
            detail: Some(format!("Unrecognized function: {}", symbol)),
        })
    }

    pub(crate) fn expected_symbol(position: Span, symbol: &str) -> Self {
        Self::ExpectedSymbol(ParserErrorData {
            position,
            detail: Some(format!("Expected {}", symbol)),
        })
    }
    pub(crate) fn invalid_expr(position: Span, detail: &str) -> Self {
        Self::InvalidExpression(ParserErrorData {
            position,
            detail: Some(detail.to_string()),
        })
    }
    pub(crate) fn n_function_args(position: Span, detail: &str) -> Self {
        Self::NFunctionArgs(ParserErrorData {
            position,
            detail: Some(format!("Incorrect number of function args: {}", detail)),
        })
    }
    pub(crate) fn invalid_token(position: Span) -> Self {
        Self::InvalidToken(ParserErrorData {
            position,
            detail: None,
        })
    }
    pub(crate) fn expect_expression(position: Span) -> Self {
        Self::ExpectExpression(ParserErrorData {
            position,
            detail: None,
        })
    }
}
