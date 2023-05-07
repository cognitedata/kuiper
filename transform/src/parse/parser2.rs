use lalrpop_util::lalrpop_mod;

lalrpop_mod!(jsontf);

#[cfg(test)]
mod tests {
    use crate::{
        expressions::Operator,
        lexer::{Lexer, ParseError, Token},
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

    /* #[test]
    pub fn test_bad_selector_2() {
        let res = parse_fail("2 + id..");
        match res {
            ParserError::UnexpectedSymbol(d) => {
                assert_eq!(d.detail, Some("Unexpected symbol .".to_string()));
                assert_eq!(d.position, Span { start: 7, end: 8 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_bad_selector_3() {
        let res = parse_fail("2 + id.[0]");
        match res {
            ParserError::UnexpectedSymbol(d) => {
                assert_eq!(d.detail, Some("Unexpected symbol [".to_string()));
                assert_eq!(d.position, Span { start: 7, end: 8 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_weird_list() {
        let res = parse_fail("[1, 2,]");
        match res {
            ParserError::ExpectExpression(d) => {
                assert_eq!(d.position, Span { start: 6, end: 7 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_empty_expression() {
        let res = parse_fail("2 + ()");
        match res {
            ParserError::EmptyExpression(d) => {
                assert_eq!(d.position, Span { start: 4, end: 6 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_missing_terminator() {
        let res = parse_fail("2 + (2 * ");
        match res {
            ParserError::InvalidExpression(d) => {
                assert_eq!(d.detail, Some("Failed to parse expression".to_string()));
                assert_eq!(d.position, Span { start: 4, end: 8 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_unterminated_string() {
        let res = parse_fail("2 + 'test ");
        match res {
            ParserError::InvalidToken(d) => {
                assert_eq!(d.position, Span { start: 4, end: 10 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_misplaced_operator() {
        let res = parse_fail("2 + + 'test' 3");
        match res {
            ParserError::ExpectExpression(d) => {
                assert_eq!(d.position, Span { start: 4, end: 5 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_misplaced_expression() {
        let res = parse_fail("2 + 'test' 'test'");
        match res {
            ParserError::UnexpectedSymbol(d) => {
                assert_eq!(d.detail, Some("Unexpected symbol 'test'".to_string()));
                assert_eq!(d.position, Span { start: 11, end: 17 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_wrong_function_args() {
        let res = parse_fail("2 + pow(2)");
        match res {
            ParserError::NFunctionArgs(d) => {
                assert_eq!(
                    d.detail,
                    Some(
                        "Incorrect number of function args: function pow takes 2 arguments"
                            .to_string()
                    )
                );
                assert_eq!(d.position, Span { start: 4, end: 10 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_unrecognized_function() {
        let res = parse_fail("2 + bloop(34)");
        match res {
            ParserError::UnexpectedSymbol(d) => {
                assert_eq!(d.detail, Some("Unrecognized function: bloop".to_string()));
                assert_eq!(d.position, Span { start: 4, end: 13 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    } */

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

    /* #[test]
    pub fn test_misplaced_negate() {
        let res = parse_fail("2 + 3!");
        match res {
            ParserError::UnexpectedSymbol(d) => {
                assert_eq!(d.detail, Some("Unexpected symbol !".to_string()));
                assert_eq!(d.position, Span { start: 5, end: 6 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    } */

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

    /* #[test]
    pub fn test_empty_lambda() {
        let res = parse_fail("() => ");
        match res {
            ParserError::EmptyExpression(d) => {
                assert_eq!(d.detail, None);
                assert_eq!(d.position, Span { start: 3, end: 5 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_unexpected_lambda() {
        let res = parse_fail("1 + () => 1 + 1");
        match res {
            ParserError::UnexpectedLambda(d) => {
                assert_eq!(d.detail, None);
                assert_eq!(d.position, Span { start: 4, end: 9 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    } */

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
