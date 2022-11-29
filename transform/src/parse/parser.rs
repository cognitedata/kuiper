use logos::{Lexer, Span};

use crate::{
    expressions::{
        get_function_expression, ArrayExpression, Constant, ExpressionType, OpExpression, Operator,
        SelectorElement, SelectorExpression,
    },
    lexer::Token,
};

use super::parse_error::ParserError;

pub struct Parser<'source> {
    tokens: Lexer<'source, Token>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ExprTerminator {
    Comma,
    CloseParenthesis,
    CloseBracket,
    End,
}

macro_rules! consume_token {
    ($slf:ident, $pt:pat) => {{
        let token = consume_token!($slf);
        match token {
            $pt => (),
            _ => {
                return Err(ParserError::incorrect_symbol($slf.tokens.span(), token));
            }
        }
        token
    }};

    ($slf:ident) => {{
        let token = match $slf.tokens.next() {
            Some(x) => x,
            None => return Err(ParserError::empty_expression($slf.tokens.span())),
        };
        token
    }};
}

impl<'source> Parser<'source> {
    pub fn new(stream: Lexer<'source, Token>) -> Self {
        Self { tokens: stream }
    }

    pub fn parse(&mut self) -> Result<ExpressionType, ParserError> {
        let (expr, term) = self.parse_expression()?;
        if term == ExprTerminator::End {
            Ok(expr)
        } else {
            Err(ParserError::empty_expression(self.tokens.span()))
        }
    }

    fn group_expressions(ops: Vec<(Operator, Span)>, exprs: Vec<ExpressionType>) -> ExpressionType {
        let mut lowest = 1000;
        let mut idx: i64 = -1;

        for (i, (op, _)) in ops.iter().enumerate() {
            if op.priority() <= lowest {
                lowest = op.priority();
                idx = i as i64;
            }
        }

        if idx < 0 {
            return exprs.into_iter().next().unwrap();
        }

        let mut lhs_ops = vec![];
        let mut lhs = vec![];
        let mut drain = exprs.into_iter();

        for i in 0..(idx + 1) {
            lhs.push(drain.next().unwrap());
            if i < idx {
                lhs_ops.push(ops[i as usize].clone());
            }
        }
        let rhs = drain.collect();
        let mut rhs_ops = vec![];
        for i in (idx + 1)..(ops.len() as i64) {
            rhs_ops.push(ops[i as usize].clone());
        }
        let lhs = Self::group_expressions(lhs_ops, lhs);
        let rhs = Self::group_expressions(rhs_ops, rhs);
        let (op, span) = ops[idx as usize].clone();
        ExpressionType::Operator(OpExpression::new(op, lhs, rhs, span))
    }

    fn parse_expression(&mut self) -> Result<(ExpressionType, ExprTerminator), ParserError> {
        let start = self.tokens.span();
        let mut exprs = vec![];
        let mut ops = vec![];
        let mut token = match self.tokens.next() {
            Some(x) => x,
            None => return Err(ParserError::empty_expression(self.tokens.span())),
        };
        let term = loop {
            // println!("Investigate symbol {}", token);
            match token {
                Token::Period => {
                    return Err(ParserError::incorrect_symbol(self.tokens.span(), token))
                }
                Token::Comma => break ExprTerminator::Comma,
                Token::Error => return Err(ParserError::invalid_token(self.tokens.span())),
                Token::Operator(o) => ops.push((o, self.tokens.span())),
                Token::OpenParenthesis => {
                    let (expr, term) = self.parse_expression()?;
                    match term {
                        ExprTerminator::CloseParenthesis => (),
                        _ => return Err(ParserError::expected_symbol(self.tokens.span(), ")")),
                    }
                    exprs.push(expr)
                }
                Token::CloseParenthesis => break ExprTerminator::CloseParenthesis,
                Token::Float(n) => exprs.push(ExpressionType::Constant(
                    Constant::try_new_f64(n)
                        .ok_or_else(|| ParserError::incorrect_symbol(self.tokens.span(), token))?,
                )),
                Token::Integer(n) => exprs.push(ExpressionType::Constant(
                    Constant::try_new_i64(n)
                        .ok_or_else(|| ParserError::incorrect_symbol(self.tokens.span(), token))?,
                )),
                Token::UInteger(n) => exprs.push(ExpressionType::Constant(
                    Constant::try_new_u64(n)
                        .ok_or_else(|| ParserError::incorrect_symbol(self.tokens.span(), token))?,
                )),
                Token::String(ref s) => exprs.push(ExpressionType::Constant(
                    Constant::try_new_string(s.clone()),
                )),
                Token::BareString(f) => {
                    let start = self.tokens.span();
                    consume_token!(self, Token::OpenParenthesis);
                    let (args, term) = self.parse_expression_list()?;
                    if !matches!(term, ExprTerminator::CloseParenthesis) {
                        return Err(ParserError::expected_symbol(self.tokens.span(), ")"));
                    }

                    let span = Span {
                        start: start.start,
                        end: self.tokens.span().end,
                    };
                    let func = get_function_expression(span, &f, args)?;
                    exprs.push(func);
                }
                Token::SelectorStart => {
                    let (expr, next) = self.parse_selector()?;
                    exprs.push(ExpressionType::Selector(expr));
                    match next {
                        Some(x) => token = x,
                        None => break ExprTerminator::End,
                    }
                    continue;
                }
                Token::OpenBracket => {
                    let start = self.tokens.span();
                    let (items, term) = self.parse_expression_list()?;
                    let span = Span {
                        start: start.start,
                        end: self.tokens.span().end,
                    };
                    if !matches!(term, ExprTerminator::CloseBracket) {
                        return Err(ParserError::expected_symbol(self.tokens.span(), "]"));
                    }

                    let expr = ArrayExpression::new(items, span);
                    exprs.push(ExpressionType::Array(expr));
                }
                Token::CloseBracket => break ExprTerminator::CloseBracket,
            }
            token = match self.tokens.next() {
                Some(x) => x,
                None => break ExprTerminator::End,
            };
        };
        let span = Span {
            start: start.start,
            end: self.tokens.span().end,
        };

        if exprs.len() != ops.len() + 1 {
            return Err(ParserError::invalid_expr(
                span,
                "Failed to parse expression",
            ));
        }

        let expr = Self::group_expressions(ops, exprs);
        Ok((expr, term))
    }

    // Parse comma separated list of expressions
    fn parse_expression_list(
        &mut self,
    ) -> Result<(Vec<ExpressionType>, ExprTerminator), ParserError> {
        let mut res = vec![];
        let term = loop {
            let (expr, term) = self.parse_expression()?;
            res.push(expr);
            match term {
                ExprTerminator::CloseParenthesis | ExprTerminator::CloseBracket => break term,
                ExprTerminator::End => {
                    return Err(ParserError::empty_expression(self.tokens.span()))
                }
                ExprTerminator::Comma => (),
            }
        };
        Ok((res, term))
    }

    fn parse_selector(&mut self) -> Result<(SelectorExpression, Option<Token>), ParserError> {
        let mut path = vec![];
        let start = self.tokens.span();

        let mut require_symbol = true;

        let final_token = loop {
            let mut next = match self.tokens.next() {
                Some(x) => x,
                None => break None,
            };
            // println!("Investigate selector symbol {}", next);
            if require_symbol {
                match next {
                    Token::BareString(s) => path.push(SelectorElement::Constant(s)),
                    Token::UInteger(s) => path.push(SelectorElement::Constant(s.to_string())),
                    _ => {
                        return Err(ParserError::incorrect_symbol(self.tokens.span(), next));
                    }
                }
                next = match self.tokens.next() {
                    Some(x) => x,
                    None => break None,
                };
            }

            match next {
                Token::Period => require_symbol = true,
                Token::OpenBracket => {
                    require_symbol = false;
                    let (exprs, term) = self.parse_expression_list()?;
                    if exprs.len() != 1 {
                        return Err(ParserError::invalid_expr(
                            self.tokens.span(),
                            "Expected a single element inside [...] selector expression",
                        ));
                    }
                    if !matches!(term, ExprTerminator::CloseBracket) {
                        return Err(ParserError::expected_symbol(self.tokens.span(), "]"));
                    }
                    let expr = exprs.into_iter().next().unwrap();
                    path.push(SelectorElement::Expression(Box::new(expr)));
                }
                _ => break Some(next),
            }
        };
        let span = Span {
            start: start.start,
            end: self.tokens.span().end,
        };
        if path.is_empty() {
            return Err(ParserError::empty_expression(span));
        }
        let expr = SelectorExpression::new(path.remove(0), path, span);
        // println!("Got selector {}", expr);
        Ok((expr, final_token))
    }
}

#[cfg(test)]
pub mod test {
    use logos::{Logos, Span};

    use crate::{expressions::ExpressionType, lexer::Token, parse::ParserError};

    use super::Parser;

    fn parse(inp: &str) -> Result<ExpressionType, ParserError> {
        let lex = Token::lexer(inp);
        Parser::new(lex).parse()
    }

    fn parse_fail(inp: &str) -> ParserError {
        match parse(inp) {
            Ok(_) => panic!("Expected parse to fail"),
            Err(x) => x,
        }
    }

    #[test]
    pub fn test_order_of_ops() {
        let expr = parse("2 + 2 * $id.elem - 3 * 3 + pow(2, 2)").unwrap();
        // The parentheses indicate the order of operations, i.e. this expression is valid even if you ignore
        // normal order of operation rules.
        assert_eq!(
            "(((2 + (2 * $id.elem)) - (3 * 3)) + pow(2, 2))",
            expr.to_string()
        );
    }

    #[test]
    pub fn test_bad_selector() {
        let res = parse_fail("2 + $id.+");
        match res {
            ParserError::IncorrectSymbol(d) => {
                assert_eq!(d.detail, Some("Unexpected symbol +".to_string()));
                assert_eq!(d.position, Span { start: 8, end: 9 });
            }
            _ => panic!("Wrong type of response: {:?}", res),
        }
    }

    #[test]
    pub fn test_empty_expression() {
        let res = parse_fail("2 + ()");
        match res {
            ParserError::InvalidExpression(d) => {
                assert_eq!(d.detail, Some("Failed to parse expression".to_string()));
                assert_eq!(d.position, Span { start: 4, end: 6 });
            }
            _ => panic!("Wrong type of response: {:?}", res),
        }
    }

    #[test]
    pub fn test_missing_terminator() {
        let res = parse_fail("2 + (2 * ");
        match res {
            ParserError::InvalidExpression(d) => {
                assert_eq!(d.detail, Some("Failed to parse expression".to_string()));
                assert_eq!(d.position, Span { start: 4, end: 9 });
            }
            _ => panic!("Wrong type of response: {:?}", res),
        }
    }

    #[test]
    pub fn test_unterminated_string() {
        let res = parse_fail("2 + 'test ");
        match res {
            ParserError::InvalidToken(d) => {
                assert_eq!(d.position, Span { start: 4, end: 10 });
            }
            _ => panic!("Wrong type of response: {:?}", res),
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
            _ => panic!("Wrong type of response: {:?}", res),
        }
    }

    #[test]
    pub fn test_unrecognized_function() {
        let res = parse_fail("2 + bloop(34)");
        match res {
            ParserError::IncorrectSymbol(d) => {
                assert_eq!(d.detail, Some("Unrecognized function: bloop".to_string()));
                assert_eq!(d.position, Span { start: 4, end: 13 });
            }
            _ => panic!("Wrong type of response: {:?}", res),
        }
    }
}
