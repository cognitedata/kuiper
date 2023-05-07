mod token;

use std::num::{ParseFloatError, ParseIntError};

pub use self::token::Token;

use logos::{Logos, Span, SpannedIter};

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

pub type ParseError = lalrpop_util::ParseError<usize, Token, LexerError>;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum LexerError {
    #[default]
    UnknownToken,
    InvalidToken(Span),
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
}

impl From<ParseIntError> for LexerError {
    fn from(value: ParseIntError) -> Self {
        LexerError::ParseInt(value)
    }
}

impl From<ParseFloatError> for LexerError {
    fn from(value: ParseFloatError) -> Self {
        LexerError::ParseFloat(value)
    }
}

pub struct Lexer<'input> {
    token_stream: SpannedIter<'input, Token>,
    last: Option<Spanned<Token, usize, LexerError>>,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        let mut stream = Token::lexer(input).spanned();
        let last = stream.next().map(|(token, span)| match token {
            Ok(t) => Ok((span.start, t, span.end)),
            Err(_) => Err(LexerError::InvalidToken(span)),
        });
        Self {
            token_stream: stream,
            last,
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Token, usize, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        let tok = self.token_stream.next().map(|(token, span)| match token {
            Ok(t) => Ok((span.start, t, span.end)),
            Err(_) => Err(LexerError::InvalidToken(span)),
        });
        // Unpleasant hack to get around LR(1) and a bug in Logos.
        // Keep a token stored, and if we encounter ) =>, combine the two tokens.

        match &tok {
            Some(t) => match t {
                Ok((_, Token::Arrow, e)) => match self.last {
                    Some(Ok((s, Token::CloseParenthesis, _))) => {
                        self.last = self.token_stream.next().map(|(token, span)| match token {
                            Ok(t) => Ok((span.start, t, span.end)),
                            Err(_) => Err(LexerError::InvalidToken(span)),
                        });
                        Some(Ok((s, Token::CombinedArrow, *e)))
                    }
                    _ => {
                        let lst = self.last.take();
                        self.last = tok;
                        lst
                    }
                },
                _ => {
                    let lst = self.last.take();
                    self.last = tok;
                    lst
                }
            },
            None => self.last.take(),
        }
    }
}
