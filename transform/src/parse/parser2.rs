use lalrpop_util::lalrpop_mod;

lalrpop_mod!(jsontf);

#[cfg(test)]
mod tests {
    use logos::Span;

    use crate::{
        expressions::{Operator, UnaryOperator},
        lexer::{Lexer, LexerError, ParseError, Token},
        parse::ast::{Constant, Expression},
    };

    use super::jsontf::ExprParser;

    fn parse(dat: &str) -> Result<Expression, ParseError> {
        let p = ExprParser::new();
        let tokens = Lexer::new(dat);
        p.parse(tokens)
    }

    fn parse_fail(inp: &str) -> ParseError {
        match parse(inp) {
            Ok(_) => panic!("Expected parse to fail"),
            Err(x) => x,
        }
    }

    #[test]
    fn test_const_parse() {
        fn parse_const(dat: &str) -> Constant {
            let r = parse(dat);
            let r = r.unwrap();
            match r {
                Expression::Constant(c) => c,
                _ => panic!("Wrong type of result"),
            }
        }

        assert_eq!(Constant::PositiveInteger(123), parse_const("123"));
        assert_eq!(Constant::NegativeInteger(-123), parse_const("-123"));
        assert_eq!(Constant::Float(123.123), parse_const("123.123"));
        assert_eq!(Constant::Bool(true), parse_const("true"));
        assert_eq!(
            Constant::String("test".to_string()),
            parse_const("\"test\"")
        );
        assert_eq!(Constant::Null, parse_const("null"));
    }

    #[test]
    pub fn test_order_of_ops() {
        let expr = parse("2 + 2 * id.elem - 3 * 3 + pow(2, 2)").unwrap();
        // The parentheses indicate the order of operations, i.e. this expression is valid even if you ignore
        // normal order of operation rules.
        assert_eq!(
            "(((2 + (2 * id.elem)) - (3 * 3)) + pow(2, 2))",
            expr.to_string()
        );
    }

    #[test]
    pub fn test_empty_array() {
        parse("[] + []").unwrap();
    }

    #[test]
    pub fn test_complex_selector() {
        parse("test[0].foo.bar[0]").unwrap();
    }

    #[test]
    pub fn test_bad_selector() {
        let res = parse_fail("2 + id.+");
        match res {
            ParseError::UnrecognizedToken { token, expected } => {
                assert_eq!((7, Token::Operator(Operator::Plus), 8), token);
                assert_eq!(vec![r#""var""#.to_string()], expected);
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_bad_selector_2() {
        let res = parse_fail("2 + id..");
        match res {
            ParseError::UnrecognizedToken { token, expected } => {
                assert_eq!((7, Token::Period, 8), token);
                assert_eq!(vec![r#""var""#.to_string()], expected);
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_bad_selector_3() {
        let res = parse_fail("2 + id.[0]");
        match res {
            ParseError::UnrecognizedToken { token, expected } => {
                assert_eq!((7, Token::OpenBracket, 8), token);
                assert_eq!(vec![r#""var""#.to_string()], expected);
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_empty_expression() {
        let res = parse_fail("2 + ()");
        match res {
            ParseError::UnrecognizedToken { token, expected: _ } => {
                assert_eq!((5, Token::CloseParenthesis, 6), token);
                // Expect is a bunch of stuff here
                // assert_eq!(vec![r#""var""#.to_string()], expected);
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_missing_terminator() {
        let res = parse_fail("2 + (2 * ");
        match res {
            ParseError::UnrecognizedEof {
                location,
                expected: _,
            } => {
                assert_eq!(8, location);
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_unterminated_string() {
        let res = parse_fail("2 + 'test ");
        match res {
            ParseError::User {
                error: LexerError::InvalidToken(d),
            } => {
                assert_eq!(d, Span { start: 4, end: 10 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_misplaced_operator() {
        let res = parse_fail("2 + + 'test'");
        match res {
            ParseError::UnrecognizedToken { token, expected: _ } => {
                assert_eq!((4, Token::Operator(Operator::Plus), 5), token);
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_misplaced_expression() {
        let res = parse_fail("2 + 'test' 'test'");
        match res {
            ParseError::UnrecognizedToken { token, expected: _ } => {
                assert_eq!((11, Token::String("test".to_string()), 17), token);
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_negate_op() {
        let res = parse("2 + !!3").unwrap();
        assert_eq!("(2 + !!3)", res.to_string());
    }

    #[test]
    pub fn test_negate_expr() {
        let res = parse("2 + !(1 + !3 - 5)").unwrap();
        assert_eq!("(2 + !((1 + !3) - 5))", res.to_string());
    }

    #[test]
    pub fn test_misplaced_negate() {
        let res = parse_fail("2 + 3!");
        match res {
            ParseError::UnrecognizedToken { token, expected: _ } => {
                assert_eq!((5, Token::UnaryOperator(UnaryOperator::Negate), 6), token);
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_array_idx() {
        let res = parse("inp.test[0] + [0, 1, 2][2]").unwrap();
        assert_eq!("(inp.test[0] + [0, 1, 2][2])", res.to_string());
    }

    #[test]
    pub fn test_object_creation() {
        let res = parse(r#"{ "test": 1 + 2 + 3, 'wow' + 1: 45 * 3 }"#).unwrap();
        assert_eq!(
            r#"{"test": ((1 + 2) + 3), ("wow" + 1): (45 * 3)}"#,
            res.to_string()
        );
    }

    #[test]
    pub fn test_empty_object() {
        let res = parse("{}").unwrap();
        assert_eq!("{}", res.to_string());
    }

    #[test]
    pub fn test_index_object() {
        let res = parse("{ 'test': 'test' }['test']").unwrap();
        assert_eq!(r#"{"test": "test"}["test"]"#, res.to_string())
    }

    #[test]
    pub fn test_lambda() {
        let res = parse("map([], (arg1) => 1 + 1) + arg1").unwrap();
        assert_eq!(r#"(map([], (arg1) => (1 + 1)) + arg1)"#, res.to_string());
    }

    #[test]
    pub fn test_lambda_arg() {
        let res = parse("map([], (arg1) => 1 + 1)").unwrap();
        assert_eq!(r#"map([], (arg1) => (1 + 1))"#, res.to_string());
    }

    #[test]
    pub fn test_empty_lambda() {
        let res = parse_fail("() => ");
        match res {
            ParseError::UnrecognizedToken { token, expected: _ } => {
                assert_eq!((1, Token::CombinedArrow, 5), token);
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_unexpected_lambda() {
        let res = parse_fail("1 + () => 1 + 1");
        match res {
            ParseError::UnrecognizedToken { token, expected: _ } => {
                assert_eq!((5, Token::CombinedArrow, 9), token);
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_postfix_function() {
        let res = parse("1.pow(2)").unwrap();
        assert_eq!("pow(1, 2)", res.to_string());
    }

    #[test]
    pub fn test_deep_postfix_function() {
        let res = parse(r#"{ "test": [123] }.test[0].pow(2)"#).unwrap();
        assert_eq!(r#"pow({"test": [123]}.test[0], 2)"#, res.to_string());
    }

    #[test]
    pub fn test_nested_postfix_function() {
        let res = parse(r#"{ "test": [123] }.test.map((a) => a * 2)[0].pow(2)"#).unwrap();
        assert_eq!(
            r#"pow(map({"test": [123]}.test, (a) => (a * 2))[0], 2)"#,
            res.to_string()
        );
    }
}
