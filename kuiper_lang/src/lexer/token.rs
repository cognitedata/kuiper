use std::fmt::Display;

use logos::{Logos, Span};

use crate::expressions::{Operator, TypeLiteral, UnaryOperator};

use crate::lexer::LexerError;

fn parse_string(mut raw: &str, border_char: char, start: usize) -> Result<String, LexerError> {
    raw = &raw[1..raw.len() - 1];
    let mut res = String::with_capacity(raw.len());

    let mut pos = start;
    let mut escaping = false;
    for c in raw.chars() {
        if c == '\\' {
            if escaping {
                res.push(c);
                escaping = false;
            } else {
                escaping = true;
            }
        } else if escaping {
            if c == border_char {
                res.push(c);
            } else if c == 'n' {
                res.push('\n');
            } else if c == 't' {
                res.push('\t');
            } else {
                return Err(LexerError::InvalidEscapeChar((
                    c,
                    Span {
                        start: pos,
                        end: pos + 1,
                    },
                )));
            }
            escaping = false;
        } else {
            res.push(c);
        }
        pos += 1;
    }
    Ok(res)
}

/// The Token type is the entry point for expressions. The input is a string that is automatically tokenized by Logos.
/// Any new operators, special symbols, or behavior needs to be added here.
/// Aim to do all the actual text parsing here, so that the parser can operate purely on tokens.
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+", error = LexerError)]
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

    #[token("...")]
    DotDot,

    /// A floating point number. Strictly not an integer.
    #[regex(r#"(\d*\.)?\d+"#, |lex| lex.slice().parse().map_err(|e| LexerError::ParseFloat((e, lex.span()))))]
    #[regex(r#"(\d*\.)?\d+[eE][+-]?(\d)"#, |lex| lex.slice().parse().map_err(|e| LexerError::ParseFloat((e, lex.span()))))]
    Float(f64),

    /// A positive integer.
    #[regex(r#"(\d)+"#, |lex| lex.slice().parse().map_err(|e| LexerError::ParseInt((e, lex.span()))), priority = 2)]
    Integer(u64),

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
    #[token("%", |_| Operator::Modulo)]
    #[token("is", |_| Operator::Is)]
    Operator(Operator),

    /// A unary operator, takes the following expression as argument.
    #[token("!", |_| UnaryOperator::Negate)]
    UnaryOperator(UnaryOperator),

    /// A quoted string. We use single quotes for string literals.
    #[regex(r#"'(?:[^'\\]|\\.)*'"#, |s| parse_string(s.slice(), '\'', s.span().start))]
    #[regex(r#""(?:[^"\\]|\\.)*""#, |s| parse_string(s.slice(), '\"', s.span().start))]
    String(String),

    /// A literal refering to a type, also includes the null literal
    #[token("null", |_| TypeLiteral::Null)]
    #[token("int", |_| TypeLiteral::Int)]
    #[token("bool", |_| TypeLiteral::Bool)]
    #[token("float", |_| TypeLiteral::Float)]
    #[token("string", |_| TypeLiteral::String)]
    #[token("array", |_| TypeLiteral::Array)]
    #[token("object", |_| TypeLiteral::Object)]
    #[token("number", |_| TypeLiteral::Number)]
    TypeLiteral(TypeLiteral),

    /// A bare string, which is either part of a selector, or a function call.
    #[regex(r#"\p{XID_Start}\p{XID_Continue}*"#, |s| s.slice().to_string())]
    #[regex(r#"[_a-zA-Z][_0-9a-zA-Z]*"#, |s| s.slice().to_string(), priority = 2)]
    #[regex(r#"`(?:[^`\\]|\\.)*`"#, |s| parse_string(s.slice(), '`', s.span().start))]
    Identifier(String),

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

    CombinedArrow,

    #[token("/*", |lex| {
        let len = lex.remainder().find("*/")?;
        lex.bump(len + 2); // include len of `*/`
        Some(())
    })]
    #[regex("//[^\n]*")]
    Comment,
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
            Token::Identifier(x) => write!(f, "`{x}`"),
            Token::OpenBracket => write!(f, "["),
            Token::CloseBracket => write!(f, "]"),
            Token::Integer(x) => write!(f, "{x}"),
            Token::TypeLiteral(x) => write!(f, "{x}"),
            Token::Boolean(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Token::OpenBrace => write!(f, "{{"),
            Token::CloseBrace => write!(f, "}}"),
            Token::Colon => write!(f, ":"),
            Token::Arrow => write!(f, "=>"),
            Token::CombinedArrow => write!(f, ") =>"),
            Token::Comment => Ok(()),
            Token::DotDot => write!(f, ".."),
        }
    }
}

#[cfg(test)]
mod test {
    use logos::{Logos, Span};

    use crate::expressions::{Operator, UnaryOperator};

    use super::Token;

    #[test]
    pub fn test_lexer() {
        let mut lex = Token::lexer(
            "123 +   id.seg.`seg2 complex`/3-'some string here' + function_call(id, nested(3, 4))",
        )
        .map(|t| t.unwrap());

        assert_eq!(lex.next(), Some(Token::Integer(123)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
        assert_eq!(lex.next(), Some(Token::Identifier("id".to_string())));
        assert_eq!(lex.next(), Some(Token::Period));
        assert_eq!(lex.next(), Some(Token::Identifier("seg".to_string())));
        assert_eq!(lex.next(), Some(Token::Period));
        assert_eq!(
            lex.next(),
            Some(Token::Identifier("seg2 complex".to_string()))
        );
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Divide)));
        assert_eq!(lex.next(), Some(Token::Integer(3)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Minus)));
        assert_eq!(
            lex.next(),
            Some(Token::String("some string here".to_string()))
        );
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
        assert_eq!(
            lex.next(),
            Some(Token::Identifier("function_call".to_string()))
        );
        assert_eq!(lex.next(), Some(Token::OpenParenthesis));
        assert_eq!(lex.next(), Some(Token::Identifier("id".to_string())));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::Identifier("nested".to_string())));
        assert_eq!(lex.next(), Some(Token::OpenParenthesis));
        assert_eq!(lex.next(), Some(Token::Integer(3)));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::Integer(4)));
        assert_eq!(lex.next(), Some(Token::CloseParenthesis));
        assert_eq!(lex.next(), Some(Token::CloseParenthesis));
    }

    #[test]
    pub fn test_array_expr() {
        let mut lex = Token::lexer("['some', 123, 'array', [0, -1, 2.2]] + id['test'][0][1 + 1]")
            .map(|t| t.unwrap());

        assert_eq!(lex.next(), Some(Token::OpenBracket));
        assert_eq!(lex.next(), Some(Token::String("some".to_string())));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::Integer(123)));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::String("array".to_string())));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::OpenBracket));
        assert_eq!(lex.next(), Some(Token::Integer(0)));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Minus)));
        assert_eq!(lex.next(), Some(Token::Integer(1)));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::Float(2.2f64)));
        assert_eq!(lex.next(), Some(Token::CloseBracket));
        assert_eq!(lex.next(), Some(Token::CloseBracket));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
        assert_eq!(lex.next(), Some(Token::Identifier("id".to_string())));
        assert_eq!(lex.next(), Some(Token::OpenBracket));
        assert_eq!(lex.next(), Some(Token::String("test".to_string())));
        assert_eq!(lex.next(), Some(Token::CloseBracket));
        assert_eq!(lex.next(), Some(Token::OpenBracket));
        assert_eq!(lex.next(), Some(Token::Integer(0)));
        assert_eq!(lex.next(), Some(Token::CloseBracket));
        assert_eq!(lex.next(), Some(Token::OpenBracket));
        assert_eq!(lex.next(), Some(Token::Integer(1)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
        assert_eq!(lex.next(), Some(Token::Integer(1)));
        assert_eq!(lex.next(), Some(Token::CloseBracket));
    }

    #[test]
    pub fn test_operators() {
        let mut lex = Token::lexer("1 + !!!2 - 3 * 4 / 5 != 6").map(|t| t.unwrap());

        assert_eq!(lex.next(), Some(Token::Integer(1)));
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
        assert_eq!(lex.next(), Some(Token::Integer(2)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Minus)));
        assert_eq!(lex.next(), Some(Token::Integer(3)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Multiply)));
        assert_eq!(lex.next(), Some(Token::Integer(4)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Divide)));
        assert_eq!(lex.next(), Some(Token::Integer(5)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::NotEquals)));
        assert_eq!(lex.next(), Some(Token::Integer(6)));
    }

    #[test]
    pub fn test_object() {
        let mut lex = Token::lexer(r#"{ "test": "test", 123: 'test' }"#).map(|t| t.unwrap());
        assert_eq!(lex.next(), Some(Token::OpenBrace));
        assert_eq!(lex.next(), Some(Token::String("test".to_string())));
        assert_eq!(lex.next(), Some(Token::Colon));
        assert_eq!(lex.next(), Some(Token::String("test".to_string())));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::Integer(123)));
        assert_eq!(lex.next(), Some(Token::Colon));
        assert_eq!(lex.next(), Some(Token::String("test".to_string())));
        assert_eq!(lex.next(), Some(Token::CloseBrace));
    }

    #[test]
    pub fn test_lambda() {
        let mut lex = Token::lexer("test => 1, (test, test) => 2").map(|t| t.unwrap());
        assert_eq!(lex.next(), Some(Token::Identifier("test".to_string())));
        assert_eq!(lex.next(), Some(Token::Arrow));
        assert_eq!(lex.next(), Some(Token::Integer(1)));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::OpenParenthesis));
        assert_eq!(lex.next(), Some(Token::Identifier("test".to_string())));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::Identifier("test".to_string())));
        assert_eq!(lex.next(), Some(Token::CloseParenthesis));
        assert_eq!(lex.next(), Some(Token::Arrow));
        assert_eq!(lex.next(), Some(Token::Integer(2)));
    }

    #[test]
    pub fn test_lambda_2() {
        let mut lex = Token::lexer("map([], (arg1) => 1 + 1) + arg1").map(|t| t.unwrap());
        assert_eq!(lex.next(), Some(Token::Identifier("map".to_string())));
        assert_eq!(lex.next(), Some(Token::OpenParenthesis));
        assert_eq!(lex.next(), Some(Token::OpenBracket));
        assert_eq!(lex.next(), Some(Token::CloseBracket));
        assert_eq!(lex.next(), Some(Token::Comma));
        assert_eq!(lex.next(), Some(Token::OpenParenthesis));
        assert_eq!(lex.next(), Some(Token::Identifier("arg1".to_string())));
        assert_eq!(lex.next(), Some(Token::CloseParenthesis));
        assert_eq!(lex.next(), Some(Token::Arrow));
        assert_eq!(lex.next(), Some(Token::Integer(1)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
        assert_eq!(lex.next(), Some(Token::Integer(1)));
        assert_eq!(lex.next(), Some(Token::CloseParenthesis));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
        assert_eq!(lex.next(), Some(Token::Identifier("arg1".to_string())));
    }

    #[test]
    pub fn test_identifiers() {
        let mut lex = Token::lexer("æøå_123 _123").map(|t| t.unwrap());
        assert_eq!(lex.next(), Some(Token::Identifier("æøå_123".to_string())));
        assert_eq!(lex.next(), Some(Token::Identifier("_123".to_string())));
    }

    #[test]
    pub fn test_comments() {
        let mut lex = Token::lexer(
            "
            // some line comment
            abc
            /* some block comment */ test +
            // last line comment",
        )
        .map(|t| t.unwrap());
        assert_eq!(lex.next(), Some(Token::Comment));
        assert_eq!(lex.next(), Some(Token::Identifier("abc".to_string())));
        assert_eq!(lex.next(), Some(Token::Comment));
        assert_eq!(lex.next(), Some(Token::Identifier("test".to_string())));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Plus)));
        assert_eq!(lex.next(), Some(Token::Comment));
        assert_eq!(lex.next(), None);
    }

    #[test]
    pub fn test_numbers() {
        let mut lex = Token::lexer("123.456 123.0e6 321e5 -4e2 14e-3").map(|t| t.unwrap());
        assert_eq!(lex.next(), Some(Token::Float(123.456)));
        assert_eq!(lex.next(), Some(Token::Float(123000000.0)));
        assert_eq!(lex.next(), Some(Token::Float(32100000.0)));
        assert_eq!(lex.next(), Some(Token::Operator(Operator::Minus)));
        assert_eq!(lex.next(), Some(Token::Float(400.0)));
        assert_eq!(lex.next(), Some(Token::Float(0.014)));
    }

    #[test]
    pub fn test_escapes() {
        let mut lex = Token::lexer(r#" 'test"test' "test''''test" "test\\\"\"" 'test\''"#)
            .map(|t| t.unwrap());
        assert_eq!(lex.next(), Some(Token::String(r#"test"test"#.to_string())));
        assert_eq!(
            lex.next(),
            Some(Token::String(r#"test''''test"#.to_string()))
        );
        assert_eq!(lex.next(), Some(Token::String(r#"test\"""#.to_string())));
        assert_eq!(lex.next(), Some(Token::String(r#"test'"#.to_string())));

        let mut lex = Token::lexer(r"'test\b'");
        assert_eq!(
            lex.next(),
            Some(Err(crate::lexer::LexerError::InvalidEscapeChar((
                'b',
                Span { start: 5, end: 6 }
            ))))
        );
    }
}
