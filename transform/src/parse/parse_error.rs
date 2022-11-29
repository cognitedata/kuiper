use logos::Span;

use crate::lexer::Token;

#[derive(Debug)]
pub enum ParserError {
    EmptyExpression(ParserErrorData),
    IncorrectSymbol(ParserErrorData),
    ExpectedSymbol(ParserErrorData),
    InvalidExpression(ParserErrorData),
    InvalidToken(ParserErrorData),
    NFunctionArgs(ParserErrorData),
}

#[derive(Debug)]
pub struct ParserErrorData {
    pub position: Span,
    pub detail: Option<String>,
}

impl ParserError {
    pub fn empty_expression(position: Span) -> Self {
        Self::EmptyExpression(ParserErrorData {
            position,
            detail: None,
        })
    }
    pub fn incorrect_symbol(position: Span, symbol: Token) -> Self {
        match symbol {
            Token::Error => Self::invalid_token(position),
            _ => Self::IncorrectSymbol(ParserErrorData {
                position,
                detail: Some(format!("Unexpected symbol {}", symbol)),
            }),
        }
    }
    pub fn unrecognized_function(position: Span, symbol: &str) -> Self {
        Self::IncorrectSymbol(ParserErrorData {
            position,
            detail: Some(format!("Unrecognized function: {}", symbol)),
        })
    }

    pub fn expected_symbol(position: Span, symbol: &str) -> Self {
        Self::ExpectedSymbol(ParserErrorData {
            position,
            detail: Some(format!("Expected {}", symbol)),
        })
    }
    pub fn invalid_expr(position: Span, detail: &str) -> Self {
        Self::InvalidExpression(ParserErrorData {
            position,
            detail: Some(detail.to_string()),
        })
    }
    pub fn n_function_args(position: Span, detail: &str) -> Self {
        Self::NFunctionArgs(ParserErrorData {
            position,
            detail: Some(format!("Incorrect number of function args: {}", detail)),
        })
    }
    pub fn invalid_token(position: Span) -> Self {
        Self::InvalidToken(ParserErrorData {
            position,
            detail: None,
        })
    }
}
