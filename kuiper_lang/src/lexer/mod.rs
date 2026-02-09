mod token;

use std::{
    fmt::Display,
    num::{ParseFloatError, ParseIntError},
};

pub use self::token::Token;

use logos::{Logos, Span, SpannedIter};

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

/// Error returned by the parser. Contains the location of the error and the token at the error,
/// as well as rich information about valid tokens at the given location.
pub type ParseError = lalrpop_util::ParseError<usize, Token, LexerError>;

/// An error from the lexer.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum LexerError {
    /// An unknown token was encountered.
    #[default]
    UnknownToken,
    /// An invalid token was encountered.
    InvalidToken(Span),
    /// Failed to parse an integer.
    ParseInt((ParseIntError, Span)),
    /// Failed to parse a float.
    ParseFloat((ParseFloatError, Span)),
    /// An invalid escape character was encountered.
    InvalidEscapeChar((char, Span)),
}

impl Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexerError::UnknownToken => write!(f, "Unknown token"),
            LexerError::InvalidToken(s) => write!(f, "Unknown token at {}..{}", s.start, s.end),
            LexerError::ParseInt(e) => write!(f, "Failed to parse string as integer: {}", e.0),
            LexerError::ParseFloat(e) => write!(f, "Failed to parse string as float: {}", e.0),
            LexerError::InvalidEscapeChar(c) => write!(f, "Invalid escape character: {}", c.0),
        }
    }
}

pub struct Lexer<T> {
    token_stream: T,
    last: Option<Spanned<Token, usize, LexerError>>,
}

impl<'input> Lexer<SpannedIter<'input, Token>> {
    pub fn new(input: &'input str) -> Self {
        let mut stream = Token::lexer(input).spanned();

        let last = loop {
            match stream.next() {
                Some((Ok(Token::Comment), _)) => (),
                Some((Ok(t), span)) => break Some(Ok((span.start, t, span.end))),
                Some((Err(_), span)) => break Some(Err(LexerError::InvalidToken(span))),
                None => break None,
            }
        };

        Self {
            token_stream: stream,
            last,
        }
    }
}

impl<T: Iterator<Item = (Result<Token, LexerError>, Span)>> Lexer<T> {
    pub fn new_raw_tokens(mut stream: T) -> Self {
        let last = loop {
            match stream.next() {
                Some((Ok(Token::Comment), _)) => (),
                Some((Ok(t), span)) => break Some(Ok((span.start, t, span.end))),
                Some((Err(_), span)) => break Some(Err(LexerError::InvalidToken(span))),
                None => break None,
            }
        };

        Self {
            token_stream: stream,
            last,
        }
    }
}

impl<T: Iterator<Item = (Result<Token, LexerError>, Span)>> Iterator for Lexer<T> {
    type Item = Spanned<Token, usize, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        let tok = self.token_stream.next().map(|(token, span)| match token {
            Ok(t) => Ok((span.start, t, span.end)),
            Err(LexerError::UnknownToken) => Err(LexerError::InvalidToken(span)),
            Err(e) => Err(e),
        });
        // Unpleasant hack to get around LR(1) and a bug in Logos.
        // Keep a token stored, and if we encounter ) =>, combine the two tokens.

        match &tok {
            Some(t) => match t {
                // Skip comments. We can add other ignored tokens here.
                Ok((_, Token::Comment, _)) => self.next(),
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
