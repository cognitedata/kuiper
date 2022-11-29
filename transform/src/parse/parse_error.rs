use logos::Span;

#[derive(Debug)]
pub enum ParserError {
    EmptyExpression(ParserErrorData),
    IncorrectSymbol(ParserErrorData),
    ExpectedSymbol(ParserErrorData),
    InvalidExpression(ParserErrorData),
    NFunctionArgs(ParserErrorData),
}

#[derive(Debug)]
pub struct ParserErrorData {
    position: Span,
    detail: Option<String>,
}

impl ParserError {
    pub fn empty_expression(position: Span) -> Self {
        Self::EmptyExpression(ParserErrorData {
            position,
            detail: None,
        })
    }
    pub fn incorrect_symbol(position: Span, symbol: String) -> Self {
        Self::IncorrectSymbol(ParserErrorData {
            position,
            detail: Some(format!("Unexpected symbol {}", symbol)),
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
            detail: Some(format!("Incorrect number of function args {}", detail)),
        })
    }
}
