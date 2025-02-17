#![no_main]
#![cfg(feature = "nightly")]

use std::fmt::{Debug, Display};

use kuiper_lang::{
    lex::{compile_from_tokens, Operator, Token, TypeLiteral, UnaryOperator},
    CompilerConfig,
};
use libfuzzer_sys::{arbitrary::Arbitrary, fuzz_target};

fuzz_target!(|data: TokensWrapped| {
    let _ = compile_from_tokens(
        data.0.into_iter().map(|t| t.0),
        &["input"],
        &CompilerConfig::new(),
    );
});

fn get_identifier(
    u: &mut libfuzzer_sys::arbitrary::Unstructured<'_>,
) -> Result<String, libfuzzer_sys::arbitrary::Error> {
    Ok(match u.int_in_range(0u8..=70u8)? {
        0 => "pow",
        1 => "log",
        2 => "atan2",
        3 => "floor",
        4 => "ceil",
        5 => "round",
        6 => "concat",
        7 => "string",
        8 => "int",
        9 => "float",
        10 => "try_float",
        11 => "try_int",
        12 => "try_bool",
        13 => "if",
        14 => "to_unix_timestamp",
        15 => "format_timestamp",
        16 => "case",
        17 => "pairs",
        18 => "map",
        19 => "flatmap",
        20 => "reduce",
        21 => "filter",
        22 => "zip",
        23 => "length",
        24 => "chunk",
        25 => "now",
        26 => "join",
        27 => "except",
        28 => "select",
        29 => "distinct_by",
        30 => "substring",
        31 => "replace",
        32 => "split",
        33 => "trim_whitespace",
        34 => "slice",
        35 => "chars",
        36 => "tail",
        37 => "to_object",
        38 => "sum",
        39 => "any",
        40 => "all",
        41 => "contains",
        42 => "string_join",
        43 => "min",
        44 => "max",
        45 => "digest",
        46 => "coalesce",
        47 => "regex_is_match",
        48 => "regex_first_match",
        49 => "regex_all_matches",
        50 => "regex_first_captures",
        51 => "regex_all_captures",
        52 => "regex_replace",
        53 => "regex_replace_all",
        54 => "starts_with",
        55 => "ends_with",
        56 => "if_value",
        57 => "input",
        v @ 58..=70 => return Ok(((b'a' + v) as char).to_string()),
        _ => unreachable!(),
    }
    .to_owned())
}

#[derive(Debug)]
pub struct TokenWrap(Token);

#[derive(Arbitrary)]
pub struct TokensWrapped(Vec<TokenWrap>);

// Create a debug impl that is just display.
// This way the fuzz output contains the actual code
// that caused the error, instead of just a sequence of tokens.
impl Debug for TokensWrapped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for t in &self.0 {
            write!(f, "{}", t.0)?;
        }
        Ok(())
    }
}

impl<'a> Arbitrary<'a> for TokenWrap {
    fn arbitrary(
        u: &mut libfuzzer_sys::arbitrary::Unstructured<'a>,
    ) -> libfuzzer_sys::arbitrary::Result<Self> {
        let b: u8 = u.int_in_range(0..=25)?;
        Ok(TokenWrap(match b {
            0 => Token::Period,
            1 => Token::OpenParenthesis,
            2 => Token::CloseParenthesis,
            3 => Token::Comma,
            4 => Token::DotDot,
            5 => Token::Float(u.arbitrary()?),
            6 => Token::Integer(u.arbitrary()?),
            7 => Token::Boolean(u.arbitrary()?),
            8 => Token::Operator(match u.int_in_range(0u8..=13u8)? {
                0 => Operator::Plus,
                1 => Operator::Minus,
                2 => Operator::Divide,
                3 => Operator::Multiply,
                4 => Operator::GreaterThan,
                5 => Operator::LessThan,
                6 => Operator::GreaterThanEquals,
                7 => Operator::LessThanEquals,
                8 => Operator::Equals,
                9 => Operator::NotEquals,
                10 => Operator::And,
                11 => Operator::Or,
                12 => Operator::Modulo,
                13 => Operator::Is,
                _ => unreachable!(),
            }),
            9 => Token::UnaryOperator(UnaryOperator::Negate),
            10 => Token::String(u.arbitrary()?),
            11 => Token::TypeLiteral(match u.int_in_range(0u8..=7u8)? {
                0 => TypeLiteral::Null,
                1 => TypeLiteral::Int,
                2 => TypeLiteral::Bool,
                3 => TypeLiteral::Float,
                4 => TypeLiteral::String,
                5 => TypeLiteral::Array,
                6 => TypeLiteral::Object,
                7 => TypeLiteral::Number,
                _ => unreachable!(),
            }),
            12 => Token::Not,
            13 => Token::If,
            14 => Token::Else,
            15 => Token::Identifier(get_identifier(u)?),
            16 => Token::OpenBracket,
            17 => Token::CloseBracket,
            18 => Token::OpenBrace,
            19 => Token::Colon,
            20 => Token::Arrow,
            21 => Token::SemiColon,
            22 => Token::DefineEqual,
            23 => Token::DefineSym,
            24 => Token::CombinedArrow,
            25 => Token::Comment,
            _ => unreachable!(),
        }))
    }
}
