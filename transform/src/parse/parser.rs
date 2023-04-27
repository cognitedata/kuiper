use logos::{Lexer, Span};

use crate::{
    expressions::{
        get_function_expression, ArrayExpression, Constant, ExpressionType, ObjectExpression,
        OpExpression, Operator, SelectorElement, SelectorExpression, SourceElement,
        UnaryOpExpression,
    },
    lexer::Token,
};

use super::parse_error::ParserError;

/// Construct an executable syntax tree from a token stream.
/// The parser itself has a lifetime tied to the lexer, whose lifetime is tied to
/// the source of the data, this is usually not a problem, it just means that
/// you need to run the parser to completion before the input that created it goes out
/// of scope.
/// The output of the parser is not tied to the input.
pub struct Parser<'source> {
    tokens: Lexer<'source, Token>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ExprTerminator {
    Comma,
    CloseParenthesis,
    CloseBracket,
    CloseBrace,
    End,
    Colon,
}

impl ExprTerminator {
    pub fn to_token(self) -> Token {
        match self {
            ExprTerminator::Comma => Token::Comma,
            ExprTerminator::CloseParenthesis => Token::CloseParenthesis,
            ExprTerminator::CloseBracket => Token::CloseBracket,
            ExprTerminator::CloseBrace => Token::CloseBrace,
            ExprTerminator::End => Token::Error,
            ExprTerminator::Colon => Token::Colon,
        }
    }
}

/// Handy macro to consume a token, optionally expecting it to match a pattern.
/// Usage is either `let token = consume_token!(self);`
/// or `consume_token!(self, Token::Comma);` The former will return `EmptyExpression`
/// and the latter will return `IncorrectSymbol`, if they fail.
macro_rules! consume_token {
    ($slf:ident, $pt:pat) => {{
        let token = consume_token!($slf);
        match token {
            $pt => (),
            _ => {
                return Err(ParserError::unexpected_symbol($slf.tokens.span(), token));
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

enum ParseTokenResult {
    Expression(ExpressionType),
    ExpressionAndNext((ExpressionType, Token)),
    Terminator(ExprTerminator),
    ExpressionAndTerminator((ExpressionType, ExprTerminator)),
    Operator((Operator, Span)),
    Selector((Vec<SelectorElement>, Option<Token>, Span)),
}

impl<'source> Parser<'source> {
    /// Construct a new parser from a token stream.
    pub fn new(stream: Lexer<'source, Token>) -> Self {
        Self { tokens: stream }
    }

    /// Entry point for the parser.
    pub fn parse(&mut self) -> Result<ExpressionType, ParserError> {
        let (expr, term) = self.parse_expression()?;
        // Expect the terminator to be `End`, otherwise something like 1 + 1) + 1 would yield (1 + 1).
        if term == ExprTerminator::End {
            if let Some(expr) = expr {
                Ok(expr)
            } else {
                Err(ParserError::empty_expression(self.tokens.span()))
            }
        } else {
            Err(ParserError::unexpected_symbol(
                self.tokens.span(),
                term.to_token(),
            ))
        }
    }

    fn next_expression(
        &mut self,
        expect_expression: bool,
        is_initial: bool,
        token: Token,
    ) -> Result<ParseTokenResult, ParserError> {
        // Do a simple sanity check on the next token. After an operator, or at the start of an expression
        // only expression operators like `Token::OpenParenthesis`, `Float`, `Integer`, `String`, `SelectorStart`,
        // etc. are valid.
        // After an expression the only valid symbols are operators, or terminators.
        if matches!(
            token,
            Token::Operator(_) | Token::Comma | Token::Period | Token::Colon
        ) || !is_initial
            && matches!(
                token,
                Token::CloseParenthesis | Token::CloseBracket | Token::CloseBrace
            )
        {
            if expect_expression {
                return Err(ParserError::expect_expression(self.tokens.span()));
            }
        } else if !expect_expression && !matches!(token, Token::OpenBracket) {
            return Err(ParserError::unexpected_symbol(self.tokens.span(), token));
        }

        // println!("Investigate symbol {}", token);
        match token {
            // A period is invalid in an expression on its own.
            Token::Period => {
                if !expect_expression {
                    Err(ParserError::unexpected_symbol(self.tokens.span(), token))
                } else {
                    Ok(ParseTokenResult::Selector(
                        self.parse_selector(Token::Period, false)?,
                    ))
                }
            }
            // A colon is invalid outside of a map
            Token::Colon => Ok(ParseTokenResult::Terminator(ExprTerminator::Colon)),
            // A comma should always terminate an expression.
            Token::Comma => Ok(ParseTokenResult::Terminator(ExprTerminator::Comma)),
            // An error is never valid.
            Token::Error => Err(ParserError::invalid_token(self.tokens.span())),
            // We have already checked that an operator is valid in this position, so just add it to the operator list
            // along with the "span": where it was encountered.
            Token::Operator(o) => Ok(ParseTokenResult::Operator((o, self.tokens.span()))),

            Token::UnaryOperator(o) => {
                let token = consume_token!(self);
                let span = self.tokens.span();
                let expr = self.next_expression(true, false, token)?;
                match expr {
                    ParseTokenResult::Expression(x) => Ok(ParseTokenResult::Expression(
                        ExpressionType::UnaryOperator(UnaryOpExpression::new(o, x, span)),
                    )),
                    ParseTokenResult::ExpressionAndNext((x, next)) => {
                        Ok(ParseTokenResult::ExpressionAndNext((
                            ExpressionType::UnaryOperator(UnaryOpExpression::new(o, x, span)),
                            next,
                        )))
                    }
                    ParseTokenResult::Terminator(_) => {
                        Err(ParserError::expect_expression(self.tokens.span()))
                    }
                    ParseTokenResult::ExpressionAndTerminator((x, term)) => {
                        Ok(ParseTokenResult::ExpressionAndTerminator((
                            ExpressionType::UnaryOperator(UnaryOpExpression::new(o, x, span)),
                            term,
                        )))
                    }
                    ParseTokenResult::Operator(_) | ParseTokenResult::Selector(_) => {
                        Err(ParserError::expect_expression(self.tokens.span()))
                    }
                }
            }
            // OpenParenthesis indicates the start of a new expression when encountered here.
            // The terminator must be CloseParenthesis.
            Token::OpenParenthesis => {
                let start = self.tokens.span();
                let (expr, term) = self.parse_expression()?;
                match term {
                    ExprTerminator::CloseParenthesis => (),
                    _ => return Err(ParserError::expected_symbol(self.tokens.span(), ")")),
                };
                let span = Span {
                    start: start.start,
                    end: self.tokens.span().end,
                };
                expr.map(ParseTokenResult::Expression)
                    .ok_or_else(|| ParserError::empty_expression(span))
            }
            // CloseParenthesis terminates an expression. We don't care about what opened the expression here,
            // if a terminator is encountered we stop, the parent expression can handle checking if it is valid.
            Token::CloseParenthesis => Ok(ParseTokenResult::Terminator(
                ExprTerminator::CloseParenthesis,
            )),
            // Float, Integer, UInteger, Null and String are all constants, which is a type of expression.
            Token::Float(n) => Ok(ParseTokenResult::Expression(ExpressionType::Constant(
                Constant::try_new_f64(n)
                    .ok_or_else(|| ParserError::unexpected_symbol(self.tokens.span(), token))?,
            ))),
            Token::Integer(n) => Ok(ParseTokenResult::Expression(ExpressionType::Constant(
                Constant::try_new_i64(n)
                    .ok_or_else(|| ParserError::unexpected_symbol(self.tokens.span(), token))?,
            ))),
            Token::UInteger(n) => Ok(ParseTokenResult::Expression(ExpressionType::Constant(
                Constant::try_new_u64(n)
                    .ok_or_else(|| ParserError::unexpected_symbol(self.tokens.span(), token))?,
            ))),
            Token::String(s) => Ok(ParseTokenResult::Expression(ExpressionType::Constant(
                Constant::new_string(s),
            ))),
            Token::Null => Ok(ParseTokenResult::Expression(ExpressionType::Constant(
                Constant::new_null(),
            ))),
            Token::Boolean(b) => Ok(ParseTokenResult::Expression(ExpressionType::Constant(
                Constant::new_bool(b),
            ))),
            // A BareString encountered here is a function call, it must be followed by OpenParenthesis,
            // a (potentially empty) expression list, and a CloseParenthesis.
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
                Ok(ParseTokenResult::Expression(func))
            }
            // SelectorStart indicates the start of an expression, which has its own method for parsing.
            Token::SelectorStart => {
                let (selectors, next, span) = self.parse_selector(Token::SelectorStart, true)?;
                let expr = SelectorExpression::new(SourceElement::Input, selectors, span);
                let expr = ExpressionType::Selector(expr);
                match next {
                    Some(x) => Ok(ParseTokenResult::ExpressionAndNext((expr, x))),
                    None => Ok(ParseTokenResult::ExpressionAndTerminator((
                        expr,
                        ExprTerminator::End,
                    ))),
                }
            }
            // OpenBracket indicates the start of an array, which contains a (potentially empty) expression list,
            // and a CloseBracket.
            Token::OpenBracket => {
                if !expect_expression {
                    Ok(ParseTokenResult::Selector(
                        self.parse_selector(Token::OpenBracket, false)?,
                    ))
                } else {
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
                    Ok(ParseTokenResult::Expression(ExpressionType::Array(expr)))
                }
            }
            // CloseBracket is a terminator for arrays.
            Token::CloseBracket => Ok(ParseTokenResult::Terminator(ExprTerminator::CloseBracket)),
            Token::OpenBrace => {
                let start = self.tokens.span();
                let pairs = self.parse_map_contents()?;
                let expr = ObjectExpression::new(
                    pairs,
                    Span {
                        start: start.start,
                        end: self.tokens.span().end,
                    },
                );
                Ok(ParseTokenResult::Expression(ExpressionType::Object(expr)))
            }
            Token::CloseBrace => Ok(ParseTokenResult::Terminator(ExprTerminator::CloseBrace)),
        }
    }

    /// Convert a group of expressions and operators into an expression tree.
    /// The input to this is a list of expressions and operators built from something like
    /// `1 + (1 / 1) + 1 - 1 * 1`. This would yield operators `+, +, -, *` and expressions
    /// `1, (1 / 1), 1, 1, 1`, note how the parenthesized bit is considered an expression on its own.
    ///
    /// The output of this method should be a tree that correctly handles operator precedence.
    /// The way it works is by recursively splitting the expression on an operator. So 1 + 1 + 1
    /// might split the expression into (1 + 1) and 1, note how there is one fewer total operator,
    /// running out of operators to split on is what terminates the recursion.
    ///
    /// We get operator precedence by how we choose which operators to split on first. The first we split on
    /// in the expression is executed _last_, so in order to get correct precedence, we split on the
    /// _lowest priority, latest_ operator.
    ///
    /// In the example above we first split on `-`, which has precedence 1 along with +, this yields
    /// (1 + (1 / 1) + 1) and (1 * 1). In the right part of the tree we only have one operator to split on,
    /// which yields 1 and 1, and terminates the recursion there, since those expressions contain no more operators.
    /// In the left tree we get (1 + (1 / 1)) and 1, right tree terminates, left tree yields 1 and (1 / 1).
    /// We do not delve into the (1 / 1) tree, since that is a separate expression that was evaluated earlier.
    ///
    /// The resulting tree is
    /// ```ignore
    ///       -
    ///    +     *
    ///   + 1   1 1
    ///  1 (1/1)
    /// ```
    /// Which we would typically express as (((1 + (1 / 1)) + 1) - (1 * 1)). Note how if you calculate that
    /// expression, ignoring normal operator priority rules except for parentheses, it still comes out correct.
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

    /// Parses tokens until we reach the end of the current expression.
    /// An expression consists of a list of operators and expressions.
    /// For example, `1 + 1, 2 + 2` is two separate expressions, since `,` is an expression terminator.
    ///
    /// Moving through the tokens, an expression is always followed by either an operator (`Token::Operator`)
    /// or a terminator `Token::Comma, Token::CloseParenthesis, Token::CloseBracket, None`
    ///
    /// Certain tokens signal the start of one or more sub-expressions. A `Token::BareString` is a function call,
    /// and must be followed by `Token::OpenParenthesis`. In this case, and when a `Token::OpenBracket` is encountered,
    /// we move into parsing a list of comma separated expressions.
    ///
    /// When `Token::SelectorStart` is encountered, we build a selector, which is an expression. When `Token::OpenParenthesis`
    /// is encountered outside of a function call, we build a sub-expression.
    ///
    /// Certain tokens are automatic errors inside an expression. `Token::Period` and `Token::Error`.
    ///
    /// This method returns an expression, and an `ExprTerminator`, which is either `CloseParenthesis`, `CloseBracket`,
    /// `Comma`, or `End`, i.e. the valid symbols that may end an expression. When parsing a sub-expression
    /// we always have to check that the terminator matches the opening token. If the expression is empty it returns None,
    /// leaving to the caller to handle the case where that's wrong. For example, [] is valid, but () is not (unless it's for a function)
    fn parse_expression(
        &mut self,
    ) -> Result<(Option<ExpressionType>, ExprTerminator), ParserError> {
        let start = self.tokens.span();
        // The list of expressions and operators passed to `group_expressions`.
        let mut exprs: Vec<ExpressionType> = vec![];
        let mut ops = vec![];

        let mut token = consume_token!(self);

        let mut expect_expression = true;
        let mut initial = true;
        let term = loop {
            let previous = self.tokens.span();
            match self.next_expression(expect_expression, initial, token)? {
                ParseTokenResult::Expression(x) => {
                    exprs.push(x);
                    expect_expression = false;
                }
                ParseTokenResult::ExpressionAndNext((x, next)) => {
                    exprs.push(x);
                    token = next;
                    expect_expression = false;
                    initial = false;
                    continue;
                }
                ParseTokenResult::Terminator(t) => break t,
                ParseTokenResult::Operator(p) => {
                    ops.push(p);
                    expect_expression = true;
                }
                ParseTokenResult::ExpressionAndTerminator((x, t)) => {
                    exprs.push(x);
                    break t;
                }
                ParseTokenResult::Selector((selectors, next, span)) => {
                    let root_expr = exprs.pop();
                    let Some(root) = root_expr else {
                        return Err(ParserError::expect_expression(start));
                    };
                    exprs.push(ExpressionType::Selector(SelectorExpression::new(
                        SourceElement::Expression(Box::new(root)),
                        selectors,
                        Span {
                            start: previous.start,
                            end: span.end,
                        },
                    )));
                    initial = false;
                    if let Some(t) = next {
                        token = t;
                    } else {
                        token = match self.tokens.next() {
                            Some(x) => x,
                            None => break ExprTerminator::End,
                        };
                    }
                    continue;
                }
            }
            initial = false;

            token = match self.tokens.next() {
                Some(x) => x,
                None => break ExprTerminator::End,
            };
        };
        let span = Span {
            start: start.start,
            end: self.tokens.span().end,
        };

        if exprs.is_empty() {
            return Ok((None, term));
        }

        if exprs.len() != ops.len() + 1 {
            return Err(ParserError::invalid_expr(
                span,
                "Failed to parse expression",
            ));
        }

        let expr = Self::group_expressions(ops, exprs);
        Ok((Some(expr), term))
    }

    // Parse comma separated list of expressions. The list may be empty, if the first returned expression is null.
    fn parse_expression_list(
        &mut self,
    ) -> Result<(Vec<ExpressionType>, ExprTerminator), ParserError> {
        let mut res = vec![];
        let mut initial = true;
        let term = loop {
            let (expr, term) = self.parse_expression()?;
            let is_some = expr.is_some();
            if let Some(expr) = expr {
                res.push(expr);
            } else if !initial {
                return Err(ParserError::expect_expression(self.tokens.span()));
            }
            initial = false;
            match term {
                ExprTerminator::End => {
                    return Err(ParserError::empty_expression(self.tokens.span()))
                }
                ExprTerminator::Comma if is_some => (),
                ExprTerminator::Comma => {
                    return Err(ParserError::unexpected_symbol(
                        self.tokens.span(),
                        Token::Comma,
                    ))
                }
                term => break term,
            }
        };
        Ok((res, term))
    }

    fn parse_map_contents(&mut self) -> Result<Vec<(ExpressionType, ExpressionType)>, ParserError> {
        let mut pairs = vec![];
        let mut initial = true;
        loop {
            let (expr, term) = self.parse_expression()?;
            let key = if let Some(expr) = expr {
                expr
            } else {
                if initial && matches!(term, ExprTerminator::CloseBrace) {
                    return Ok(pairs);
                }
                return Err(ParserError::expect_expression(self.tokens.span()));
            };
            initial = false;

            if !matches!(term, ExprTerminator::Colon) {
                return Err(ParserError::unexpected_symbol(
                    self.tokens.span(),
                    term.to_token(),
                ));
            }

            let (expr, term) = self.parse_expression()?;
            let value = if let Some(expr) = expr {
                expr
            } else {
                return Err(ParserError::expect_expression(self.tokens.span()));
            };

            pairs.push((key, value));

            match term {
                ExprTerminator::Comma => (),
                ExprTerminator::CloseBrace => return Ok(pairs),
                term => {
                    return Err(ParserError::unexpected_symbol(
                        self.tokens.span(),
                        term.to_token(),
                    ))
                }
            }
        }
    }

    // Parse a selector, on the form `$id.some.json.path` or `$id.array[0][1]`, or `$['dynamic']['selectors'][1 + 1].field`
    fn parse_selector(
        &mut self,
        token: Token,
        no_initial_period: bool,
    ) -> Result<(Vec<SelectorElement>, Option<Token>, Span), ParserError> {
        let mut path = vec![];
        let start = self.tokens.span();

        let mut last_period = false;
        let mut initial = no_initial_period;

        let mut next = match token {
            Token::SelectorStart => match self.tokens.next() {
                Some(x) => x,
                None => {
                    return Err(ParserError::empty_expression(Span {
                        start: start.start,
                        end: self.tokens.span().end,
                    }))
                }
            },
            x => x,
        };

        let final_token = loop {
            match next {
                Token::BareString(s) if last_period || initial => {
                    last_period = false;
                    path.push(SelectorElement::Constant(s));
                }
                Token::UInteger(s) if last_period || initial => {
                    last_period = false;
                    path.push(SelectorElement::Constant(s.to_string()));
                }
                Token::Null if last_period || initial => {
                    last_period = false;
                    path.push(SelectorElement::Constant("null".to_string()))
                }
                Token::Period if !last_period && !initial => {
                    last_period = true;
                }
                Token::OpenBracket if !last_period => {
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
                _ => {
                    if last_period {
                        return Err(ParserError::unexpected_symbol(self.tokens.span(), next));
                    }
                    break Some(next);
                }
            }
            initial = false;
            next = match self.tokens.next() {
                Some(x) => x,
                None => break None,
            };
        };
        let span = Span {
            start: start.start,
            end: self.tokens.span().end,
        };
        if path.is_empty() {
            return Err(ParserError::empty_expression(span));
        }
        Ok((path, final_token, span))
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
    pub fn test_empty_array() {
        parse("[] + []").unwrap();
    }

    #[test]
    pub fn test_complex_selector() {
        parse("$['test'][0].foo.bar[0]").unwrap();
    }

    #[test]
    pub fn test_bad_selector() {
        let res = parse_fail("2 + $id.+");
        match res {
            ParserError::UnexpectedSymbol(d) => {
                assert_eq!(d.detail, Some("Unexpected symbol +".to_string()));
                assert_eq!(d.position, Span { start: 8, end: 9 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_bad_selector_2() {
        let res = parse_fail("2 + $id..");
        match res {
            ParserError::UnexpectedSymbol(d) => {
                assert_eq!(d.detail, Some("Unexpected symbol .".to_string()));
                assert_eq!(d.position, Span { start: 8, end: 9 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_bad_selector_3() {
        let res = parse_fail("2 + $id.[0]");
        match res {
            ParserError::UnexpectedSymbol(d) => {
                assert_eq!(d.detail, Some("Unexpected symbol [".to_string()));
                assert_eq!(d.position, Span { start: 8, end: 9 });
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
                assert_eq!(d.position, Span { start: 4, end: 9 });
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
            ParserError::UnexpectedSymbol(d) => {
                assert_eq!(d.detail, Some("Unexpected symbol !".to_string()));
                assert_eq!(d.position, Span { start: 5, end: 6 });
            }
            _ => panic!("Wrong type of response: {res:?}"),
        }
    }

    #[test]
    pub fn test_array_idx() {
        let res = parse("$inp.test[0] + [0, 1, 2][2]").unwrap();
        assert_eq!("($inp.test[0] + [0, 1, 2][2])", res.to_string());
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
}
