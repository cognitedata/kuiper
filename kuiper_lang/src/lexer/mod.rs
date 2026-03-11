mod token;

use std::{
    collections::VecDeque,
    fmt::Display,
    iter::Peekable,
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
    /// The template depth exceeded the maximum allowed depth.
    TemplateDepthExceeded(Span),
}

impl Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexerError::UnknownToken => write!(f, "Unknown token"),
            LexerError::InvalidToken(s) => write!(f, "Unknown token at {}..{}", s.start, s.end),
            LexerError::ParseInt(e) => write!(f, "Failed to parse string as integer: {}", e.0),
            LexerError::ParseFloat(e) => write!(f, "Failed to parse string as float: {}", e.0),
            LexerError::InvalidEscapeChar(c) => write!(f, "Invalid escape character: {}", c.0),
            LexerError::TemplateDepthExceeded(s) => write!(
                f,
                "Template depth exceeded maximum of {} at {}..{}",
                MAX_TEMPLATE_DEPTH, s.start, s.end
            ),
        }
    }
}

pub struct Lexer<T: Iterator<Item = (Result<Token, LexerError>, Span)>> {
    token_stream: Peekable<T>,
    inner: Vec<TemplateExpansionState>,
}

enum TemplateIterState {
    Begin,
    InTemplate,
    Done,
}

pub(crate) struct TemplateExpansionState {
    span: Span,
    last_token_end: usize,
    segments: VecDeque<TemplateComponent>,
    state: TemplateIterState,
}

impl TemplateExpansionState {
    pub(crate) fn parse(raw: &str, start_offset: usize) -> Result<Self, LexerError> {
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
                            return Err(LexerError::InvalidToken(
                                (next_offset - c.len_utf8())..next_offset,
                            ));
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
                                Token::lexer(&current)
                                    .spanned()
                                    .map(|v| {
                                        v.0.map(|r| {
                                            (v.1.start + last_offset, r, v.1.end + last_offset)
                                        })
                                    })
                                    .collect(),
                            ));
                            last_offset = next_offset - 1;
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
            span: (start_offset - 2)..(start_offset + raw.len() + 1),
            last_token_end: start_offset - 2,
            segments,
            state: TemplateIterState::Begin,
        })
    }

    fn peek(&self) -> Option<Spanned<Token, usize, LexerError>> {
        match self.state {
            TemplateIterState::Begin => Some(Ok((
                self.span.start,
                Token::TemplateStringStart,
                self.span.start + 2,
            ))),
            TemplateIterState::InTemplate => self.segments.front().and_then(|seg| match seg {
                TemplateComponent::Raw(s, span) => Some(Ok((
                    span.start,
                    Token::TemplateStringSegment(s.clone()),
                    span.end,
                ))),
                TemplateComponent::Expression(expr) => expr.front().cloned(),
            }),
            TemplateIterState::Done => Some(Ok((
                self.last_token_end,
                Token::TemplateStringEnd,
                self.span.end,
            ))),
        }
    }
}

impl Iterator for TemplateExpansionState {
    type Item = Spanned<Token, usize, LexerError>;

    fn next(&mut self) -> Option<Spanned<Token, usize, LexerError>> {
        match self.state {
            TemplateIterState::Begin => {
                self.state = TemplateIterState::InTemplate;
                self.last_token_end += 2;
                Ok((
                    self.last_token_end - 2,
                    Token::TemplateStringStart,
                    self.last_token_end,
                ))
                .into()
            }
            TemplateIterState::InTemplate => match self.segments.pop_front() {
                Some(TemplateComponent::Raw(s, span)) => {
                    self.last_token_end = span.end;
                    Some(Ok((span.start, Token::TemplateStringSegment(s), span.end)))
                }
                Some(TemplateComponent::Expression(mut expr)) => {
                    self.last_token_end = expr
                        .front()
                        .and_then(|v| v.as_ref().ok().map(|(_, _, end)| *end))
                        .unwrap_or(self.last_token_end);
                    let r = expr.pop_front();
                    if !expr.is_empty() {
                        self.segments
                            .push_front(TemplateComponent::Expression(expr));
                    }
                    r
                }
                None => {
                    self.state = TemplateIterState::Done;
                    Ok((self.last_token_end, Token::TemplateStringEnd, self.span.end)).into()
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

// Even just three nested templates is likely a mistake. This is to prevent infinite recursion in the lexer.
const MAX_TEMPLATE_DEPTH: usize = 5;

impl<'input> Lexer<SpannedIter<'input, Token>> {
    pub fn new(input: &'input str) -> Self {
        let stream = Token::lexer(input).spanned();

        Self::new_raw_tokens(stream)
    }
}

impl<T: Iterator<Item = (Result<Token, LexerError>, Span)>> Lexer<T> {
    pub fn new_raw_tokens(stream: T) -> Self {
        Self {
            token_stream: stream.peekable(),
            inner: Vec::new(),
        }
    }

    fn get_next_token(&mut self) -> Option<Spanned<Token, usize, LexerError>> {
        while let Some(inner) = self.inner.last_mut() {
            if let Some(token) = inner.next() {
                return Some(token);
            } else {
                self.inner.pop();
            }
        }

        self.token_stream.next().map(|(token, span)| match token {
            Ok(t) => Ok((span.start, t, span.end)),
            Err(LexerError::UnknownToken) => Err(LexerError::InvalidToken(span)),
            Err(e) => Err(e),
        })
    }

    fn peek(&mut self) -> Option<Spanned<Token, usize, LexerError>> {
        if let Some(inner) = self.inner.last_mut() {
            if let Some(token) = inner.peek() {
                return Some(token);
            }
        }

        self.token_stream
            .peek()
            .cloned()
            .map(|(token, span)| match token {
                Ok(t) => Ok((span.start, t, span.end)),
                Err(LexerError::UnknownToken) => Err(LexerError::InvalidToken(span)),
                Err(e) => Err(e),
            })
    }
}

impl<T: Iterator<Item = (Result<Token, LexerError>, Span)>> Iterator for Lexer<T> {
    type Item = Spanned<Token, usize, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Unpleasant hack to get around LR(1) and a bug in Logos.
        // Keep a token stored, and if we encounter ) =>, combine the two tokens.

        loop {
            let tok = self.get_next_token();

            match &tok {
                Some(t) => match t {
                    // Skip comments. We can add other ignored tokens here.
                    Ok((_, Token::Comment, _)) => continue,
                    Ok((start, Token::CloseParenthesis, _)) => match self.peek() {
                        Some(Ok((_, Token::Arrow, end))) => {
                            self.get_next_token();
                            break Some(Ok((*start, Token::CombinedArrow, end)));
                        }
                        _ => break tok,
                    },
                    Ok((start, Token::RawTemplateString(s), end)) => {
                        if self.inner.len() + 1 >= MAX_TEMPLATE_DEPTH {
                            break Some(Err(LexerError::TemplateDepthExceeded(*start..*end)));
                        }
                        match TemplateExpansionState::parse(s, *start + 2) {
                            Ok(state) => {
                                self.inner.push(state);
                                continue;
                            }
                            Err(e) => break Some(Err(e)),
                        }
                    }
                    _ => {
                        break tok;
                    }
                },
                None => break tok,
            }
        }
    }
}
