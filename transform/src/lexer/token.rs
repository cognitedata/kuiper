use std::fmt::Display;

use logos::{Lexer, Logos};

use crate::expressions::{Operator, UnaryOperator};

fn parse_string(lexer: &mut Lexer<Token>) -> String {
    let raw = lexer.slice();
    if raw.starts_with('\'') || raw.starts_with('"') {
        raw[1..raw.len() - 1].to_string()
    } else {
        raw.to_string()
    }
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

/// The Token type is the entry point for expressions. The input is a string that is automatically tokenized by Logos.
/// Any new operators, special symbols, or behavior needs to be added here.
/// Aim to do all the actual text parsing here, so that the parser can operate purely on tokens.
#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
    /// Used inside selectors, which are on the form [SelectorStart][Period][BareString or OpenBracket anything CloseBracket]...
    #[token(".")]
    Period,

    /// Used in function calls and for operation ordering.
    #[token("(")]
    OpenParenthesis,

    /// Used in function calls and for operation ordering.
    #[token(")")]
    CloseParenthesis,

    /// Used in function calls and array expressions.
    #[token(",")]
    Comma,

    /// A floating point number. Strictly not an integer.
    #[regex(r#"[-]?(\d*\.)?\d+"#, |lex| lex.slice().parse(), priority = 2)]
    Float(f64),

    /// A negative integer.
    #[regex(r#"-(\d)+"#, |lex| lex.slice().parse(), priority = 3)]
    Integer(i64),

    /// A positive integer.
    #[regex(r#"(\d)+"#, |lex| lex.slice().parse(), priority = 4)]
    UInteger(u64),

    #[token("true", |_| true)]
    #[token("false", |_| false)]
    Boolean(bool),

    /// An operator. Each new operator should be added as a special token here.
    #[token("+", |_| Operator::Plus)]
    #[token("-", |_| Operator::Minus)]
    #[token("/", |_| Operator::Divide)]
    #[token("*", |_| Operator::Multiply)]
    #[token(">", |_| Operator::GreaterThan)]
    #[token("<", |_| Operator::LessThan)]
    #[token(">=", |_| Operator::GreaterThanEquals)]
    #[token("<=", |_| Operator::LessThanEquals)]
    #[token("==", |_| Operator::Equals)]
    #[token("!=", |_| Operator::NotEquals)]
    #[token("&&", |_| Operator::And)]
    #[token("||", |_| Operator::Or)]
    Operator(Operator),

    /// A unary operator, takes the following expression as argument.
    #[token("!", |_| UnaryOperator::Negate)]
    UnaryOperator(UnaryOperator),

    /// A quoted string. We use single quotes for string literals.
    #[regex(r#"'(?:[^'\\]|\\.)*'"#, parse_string)]
    #[regex(r#""(?:[^"\\]|\\.)*""#, parse_string)]
    String(String),

    /// A literal null
    #[token("null")]
    Null,

    /// A bare string, which is either part of a selector, or a function call.
    #[regex(r#"[a-zA-Z0-9_]+"#, |s| s.slice().to_string())]
    #[regex(r#"`(?:[^`\\]|\\.)*`"#, parse_bare_string)]
    BareString(String),

    /// Start of a dynamic selector expression, (i.e. $id['some-string'])
    /// or an array.
    #[token("[")]
    OpenBracket,

    /// End of a dynamic selector expression of an array.
    #[token("]")]
    CloseBracket,

    #[token("{")]
    OpenBrace,

    #[token("}")]
    CloseBrace,

    #[token(":")]
    Colon,

    #[token("=>")]
    Arrow,

    /// Anything else, and whitespace. If it's whitespace it is skipped silently.
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
            Token::Float(x) => write!(f, "{x}"),
            Token::Operator(x) => write!(f, "{x}"),
            Token::UnaryOperator(x) => write!(f, "{x}"),
            Token::String(x) => write!(f, "'{x}'"),
            Token::BareString(x) => write!(f, "`{x}`"),
            Token::OpenBracket => write!(f, "["),
            Token::CloseBracket => write!(f, "]"),
            Token::Error => write!(f, "unknown token"),
            Token::Integer(x) => write!(f, "{x}"),
            Token::UInteger(x) => write!(f, "{x}"),
            Token::Null => write!(f, "null"),
            Token::Boolean(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Token::OpenBrace => write!(f, "{{"),
            Token::CloseBrace => write!(f, "}}"),
            Token::Colon => write!(f, ":"),
            Token::Arrow => write!(f, "=>"),
        }
    }
}

#[cfg(test)]
mod test {
    use logos::Logos;

    use crate::expressions::{Operator, UnaryOperator};

    use super::Token;

    #[test]
    pub fn test_lexer() {
        let mut lex = Token::lexer(
            "123 +   id.seg.`seg2 complex`/3-'some string here' + function_call(id, nested(3, 4))",
        );

        assert_eq!(lex.next(), Some(Token::UInteger(123)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
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

    #[test]
    pub fn test_array_expr() {
        let mut lex = Token::lexer("['some', 123, 'array', [0, -1, 2.2]] + id['test'][0][1 + 1]");

        assert_eq!(lex.next(), Some(Token::OpenBracket));
        assert_eq!(lex.next(), Some(Token::String("some".to_string())));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::UInteger(123)));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::String("array".to_string())));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::OpenBracket));
        assert_eq!(lex.next(), Some(Token::UInteger(0)));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::Integer(-1)));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::Float(2.2f64)));
        assert_eq!(lex.next(), Some(Token::CloseBracket));
        assert_eq!(lex.next(), Some(Token::CloseBracket));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
        assert_eq!(lex.next(), Some(Token::BareString("id".to_string())));
        assert_eq!(lex.next(), Some(Token::OpenBracket));
        assert_eq!(lex.next(), Some(Token::String("test".to_string())));
        assert_eq!(lex.next(), Some(Token::CloseBracket));
        assert_eq!(lex.next(), Some(Token::OpenBracket));
        assert_eq!(lex.next(), Some(Token::UInteger(0)));
        assert_eq!(lex.next(), Some(Token::CloseBracket));
        assert_eq!(lex.next(), Some(Token::OpenBracket));
        assert_eq!(lex.next(), Some(Token::UInteger(1)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
        assert_eq!(lex.next(), Some(Token::UInteger(1)));
        assert_eq!(lex.next(), Some(Token::CloseBracket));
    }

    #[test]
    pub fn test_operators() {
        let mut lex = Token::lexer("1 + !!!2 - 3 * 4 / 5 != 6");

        assert_eq!(lex.next(), Some(Token::UInteger(1)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
        assert_eq!(
            lex.next(),
            Some(Token::UnaryOperator(UnaryOperator::Negate))
        );
        assert_eq!(
            lex.next(),
            Some(Token::UnaryOperator(UnaryOperator::Negate))
        );
        assert_eq!(
            lex.next(),
            Some(Token::UnaryOperator(UnaryOperator::Negate))
        );
        assert_eq!(lex.next(), Some(Token::UInteger(2)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Minus)));
        assert_eq!(lex.next(), Some(Token::UInteger(3)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Multiply)));
        assert_eq!(lex.next(), Some(Token::UInteger(4)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Divide)));
        assert_eq!(lex.next(), Some(Token::UInteger(5)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::NotEquals)));
        assert_eq!(lex.next(), Some(Token::UInteger(6)));
    }

    #[test]
    pub fn test_object() {
        let mut lex = Token::lexer(r#"{ "test": "test", 123: 'test' }"#);
        assert_eq!(lex.next(), Some(Token::OpenBrace));
        assert_eq!(lex.next(), Some(Token::String("test".to_string())));
        assert_eq!(lex.next(), Some(Token::Colon));
        assert_eq!(lex.next(), Some(Token::String("test".to_string())));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::UInteger(123)));
        assert_eq!(lex.next(), Some(Token::Colon));
        assert_eq!(lex.next(), Some(Token::String("test".to_string())));
        assert_eq!(lex.next(), Some(Token::CloseBrace));
    }
}
