mod token;

use std::{
    collections::VecDeque,
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
    inner: Option<TemplateExpansionState>,
    last: Option<Spanned<Token, usize, LexerError>>,
}

enum TemplateIterState {
    Begin,
    InTemplate,
    Done,
}

struct TemplateExpansionState {
    last_end: usize,
    segments: VecDeque<TemplateComponent>,
    state: TemplateIterState,
}

impl TemplateExpansionState {
    fn parse(raw: &str, start_offset: usize) -> Result<Self, LexerError> {
        enum State {
            Raw,
            Expression(usize),
        }

        let mut state = State::Raw;
        let mut segments = VecDeque::new();
        let mut current = String::new();
        let mut last_offset = start_offset;
        let mut next_offset = start_offset;
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            next_offset += c.len_utf8();
            match state {
                State::Raw => {
                    if c == '{' {
                        if chars.peek() == Some(&'{') {
                            chars.next();
                            current.push('{');
                        } else {
                            state = State::Expression(0);
                            let span = last_offset..next_offset;
                            last_offset = next_offset;
                            segments.push_back(TemplateComponent::Raw(current, span));
                            current = String::new();
                        }
                    } else if c == '}' {
                        if chars.peek() == Some(&'}') {
                            chars.next();
                            current.push('}');
                        } else {
                            return Err(LexerError::InvalidToken(last_offset..next_offset));
                        }
                    } else {
                        current.push(c);
                    }
                }
                State::Expression(depth) => {
                    if c == '{' {
                        state = State::Expression(depth + 1);
                    } else if c == '}' {
                        if depth == 0 {
                            state = State::Raw;
                            segments.push_back(TemplateComponent::Expression(
                                Lexer::new(&current)
                                    .map(|v| v.map(|r| (r.0 + last_offset, r.1, r.2 + last_offset)))
                                    .collect(),
                            ));
                            last_offset = next_offset;
                            current = String::new();
                            continue;
                        } else {
                            state = State::Expression(depth - 1);
                        }
                    }
                    current.push(c);
                }
            }
        }
        if !current.is_empty() {
            segments.push_back(TemplateComponent::Raw(current, last_offset..next_offset));
        }
        Ok(Self {
            last_end: start_offset + raw.len(),
            segments,
            state: TemplateIterState::Begin,
        })
    }

    fn next(&mut self) -> Option<Spanned<Token, usize, LexerError>> {
        match self.state {
            TemplateIterState::Begin => {
                self.state = TemplateIterState::InTemplate;
                Ok((self.last_end, Token::TemplateStringStart, self.last_end)).into()
            }
            TemplateIterState::InTemplate => match self.segments.pop_front() {
                Some(TemplateComponent::Raw(s, span)) => {
                    Some(Ok((span.start, Token::TemplateStringSegment(s), span.end)))
                }
                Some(TemplateComponent::Expression(mut expr)) => {
                    let r = expr.pop_front();
                    if !expr.is_empty() {
                        self.segments
                            .push_front(TemplateComponent::Expression(expr));
                    }
                    r
                }
                None => {
                    self.state = TemplateIterState::Done;
                    Ok((self.last_end, Token::TemplateStringEnd, self.last_end)).into()
                }
            },
            TemplateIterState::Done => None,
        }
    }
}

enum TemplateComponent {
    Raw(String, Span),
    Expression(VecDeque<Spanned<Token, usize, LexerError>>),
}

impl<'input> Lexer<SpannedIter<'input, Token>> {
    pub fn new(input: &'input str) -> Self {
        let stream = Token::lexer(input).spanned();

        Self::new_raw_tokens(stream)
    }
}

impl<T: Iterator<Item = (Result<Token, LexerError>, Span)>> Lexer<T> {
    pub fn new_raw_tokens(mut stream: T) -> Self {
        let mut inner = None;

        let last = loop {
            match stream.next() {
                Some((Ok(Token::Comment), _)) => (),
                Some((Ok(Token::RawTemplateString(s)), span)) => {
                    match TemplateExpansionState::parse(&s, span.start + 2) {
                        Ok(state) => {
                            inner = Some(state);
                            break inner.as_mut().unwrap().next();
                        }
                        Err(e) => break Some(Err(e)),
                    }
                }
                Some((Ok(t), span)) => break Some(Ok((span.start, t, span.end))),
                Some((Err(_), span)) => break Some(Err(LexerError::InvalidToken(span))),
                None => break None,
            }
        };

        Self {
            token_stream: stream,
            inner,
            last,
        }
    }

    fn get_next_token(&mut self) -> Option<Spanned<Token, usize, LexerError>> {
        if let Some(inner) = &mut self.inner {
            if let Some(token) = inner.next() {
                return Some(token);
            } else {
                self.inner = None;
            }
        }

        self.token_stream.next().map(|(token, span)| match token {
            Ok(t) => Ok((span.start, t, span.end)),
            Err(LexerError::UnknownToken) => Err(LexerError::InvalidToken(span)),
            Err(e) => Err(e),
        })
    }
}

impl<T: Iterator<Item = (Result<Token, LexerError>, Span)>> Iterator for Lexer<T> {
    type Item = Spanned<Token, usize, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        let tok = self.get_next_token();
        // Unpleasant hack to get around LR(1) and a bug in Logos.
        // Keep a token stored, and if we encounter ) =>, combine the two tokens.

        match &tok {
            Some(t) => match t {
                // Skip comments. We can add other ignored tokens here.
                Ok((_, Token::Comment, _)) => self.next(),
                Ok((_, Token::Arrow, e)) => match self.last {
                    Some(Ok((s, Token::CloseParenthesis, _))) => {
                        self.last = self.get_next_token();
                        Some(Ok((s, Token::CombinedArrow, *e)))
                    }
                    _ => {
                        let lst = self.last.take();
                        self.last = tok;
                        lst
                    }
                },
                Ok((_, Token::RawTemplateString(s), v)) => {
                    match TemplateExpansionState::parse(s, *v + 2) {
                        Ok(state) => {
                            self.inner = Some(state);
                            self.next()
                        }
                        Err(e) => Some(Err(e)),
                    }
                }
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
