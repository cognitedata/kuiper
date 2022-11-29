use std::fmt::Display;

use logos::{Lexer, Logos};

use crate::expressions::Operator;

fn parse_string(lexer: &mut Lexer<Token>) -> String {
    let mut raw = lexer.slice();
    if raw.starts_with('\'') {
        raw = &raw[1..];
    }
    if raw.ends_with('\'') {
        raw = &raw[..raw.len() - 1]
    }

    raw.to_string()
}

fn parse_bare_string(lexer: &mut Lexer<Token>) -> String {
    let mut raw = lexer.slice();
    if raw.starts_with('`') {
        raw = &raw[1..];
    }
    if raw.ends_with('`') {
        raw = &raw[..raw.len() - 1]
    }

    raw.to_string()
}

#[derive(Logos, Debug, PartialEq)]
pub enum Token {
    #[token(".")]
    Period,

    #[token("(")]
    OpenParenthesis,

    #[token(")")]
    CloseParenthesis,

    #[token(",")]
    Comma,

    #[regex(r#"[-]?(\d*\.)?\d+"#, |lex| lex.slice().parse(), priority = 2)]
    Float(f64),

    #[regex(r#"-(\d)+"#, |lex| lex.slice().parse(), priority = 3)]
    Integer(i64),

    #[regex(r#"(\d)+"#, |lex| lex.slice().parse(), priority = 4)]
    UInteger(u64),

    #[token("+", |_| Operator::Plus)]
    #[token("-", |_| Operator::Minus)]
    #[token("/", |_| Operator::Divide)]
    #[token("*", |_| Operator::Multiply)]
    Operator(Operator),

    #[regex(r#"'(?:[^'\\]|\\.)*'"#, parse_string)]
    String(String),

    #[regex(r#"[a-zA-Z0-9_]+"#, |s| s.slice().to_string())]
    #[regex(r#"`(?:[^`\\]|\\.)*`"#, parse_bare_string)]
    BareString(String),

    #[token("$")]
    SelectorStart,

    #[token("[")]
    OpenBracket,

    #[token("]")]
    CloseBracket,

    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Period => write!(f, "."),
            Token::OpenParenthesis => write!(f, "("),
            Token::CloseParenthesis => write!(f, ")"),
            Token::Comma => write!(f, ","),
            Token::Float(x) => write!(f, "{}", x),
            Token::Operator(x) => write!(f, "{}", x),
            Token::String(x) => write!(f, "'{}'", x),
            Token::BareString(x) => write!(f, "`{}`", x),
            Token::SelectorStart => write!(f, "$"),
            Token::OpenBracket => write!(f, "["),
            Token::CloseBracket => write!(f, "]"),
            Token::Error => write!(f, "unknown token"),
            Token::Integer(x) => write!(f, "{}", x),
            Token::UInteger(x) => write!(f, "{}", x),
        }
    }
}

#[cfg(test)]
mod test {
    use logos::Logos;

    use crate::expressions::Operator;

    use super::Token;

    #[test]
    pub fn test_lexer() {
        let mut lex = Token::lexer("123 +   $id.seg.`seg2 complex`/3-'some string here' + function_call($id, nested(3, 4))");

        assert_eq!(lex.next(), Some(Token::UInteger(123)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
        assert_eq!(lex.next(), Some(Token::SelectorStart));
        assert_eq!(lex.next(), Some(Token::BareString("id".to_string())));
        assert_eq!(lex.next(), Some(Token::Period));
        assert_eq!(lex.next(), Some(Token::BareString("seg".to_string())));
        assert_eq!(lex.next(), Some(Token::Period));
        assert_eq!(
            lex.next(),
            Some(Token::BareString("seg2 complex".to_string()))
        );
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Divide)));
        assert_eq!(lex.next(), Some(Token::UInteger(3)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Minus)));
        assert_eq!(
            lex.next(),
            Some(Token::String("some string here".to_string()))
        );
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
        assert_eq!(
            lex.next(),
            Some(Token::BareString("function_call".to_string()))
        );
        assert_eq!(lex.next(), Some(Token::OpenParenthesis));
        assert_eq!(lex.next(), Some(Token::SelectorStart));
        assert_eq!(lex.next(), Some(Token::BareString("id".to_string())));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::BareString("nested".to_string())));
        assert_eq!(lex.next(), Some(Token::OpenParenthesis));
        assert_eq!(lex.next(), Some(Token::UInteger(3)));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::UInteger(4)));
        assert_eq!(lex.next(), Some(Token::CloseParenthesis));
        assert_eq!(lex.next(), Some(Token::CloseParenthesis));
    }
}
