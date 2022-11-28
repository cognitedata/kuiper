use logos::{Lexer, Span};

use crate::{
    expressions::{
        get_function_expression, ArrayExpression, Constant, ExpressionType, FunctionExpression,
        FunctionType, OpExpression, Operator, PowFunction, SelectorElement, SelectorExpression,
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
                return Err(ParserError::incorrect_symbol(
                    $slf.tokens.span(),
                    token.to_string(),
                ));
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
            println!("Investigate symbol {}", token);
            match token {
                Token::Period => {
                    return Err(ParserError::incorrect_symbol(
                        self.tokens.span(),
                        token.to_string(),
                    ))
                }
                Token::Comma => break ExprTerminator::Comma,
                Token::Error => {
                    return Err(ParserError::incorrect_symbol(
                        self.tokens.span(),
                        "WHITESPACE".to_string(),
                    ))
                }
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
                Token::Number(n) => exprs.push(ExpressionType::Constant(
                    Constant::try_new_f64(n).ok_or_else(|| {
                        ParserError::incorrect_symbol(self.tokens.span(), token.to_string())
                    })?,
                )),
                Token::String(ref s) => exprs.push(ExpressionType::Constant(
                    Constant::try_new_string(s.clone()),
                )),
                Token::BareString(f) => {
                    consume_token!(self, Token::OpenParenthesis);
                    let (args, term) = self.parse_expression_list()?;
                    if !matches!(term, ExprTerminator::CloseParenthesis) {
                        return Err(ParserError::expected_symbol(self.tokens.span(), ")"));
                    }

                    let func = get_function_expression(self.tokens.span(), &f, args)?;
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
                    let (items, term) = self.parse_expression_list()?;
                    if !matches!(term, ExprTerminator::CloseBracket) {
                        return Err(ParserError::expected_symbol(self.tokens.span(), "]"));
                    }

                    let expr = ArrayExpression::new(items, self.tokens.span());
                    exprs.push(ExpressionType::Array(expr));
                }
                Token::CloseBracket => break ExprTerminator::CloseBracket,
            }
            token = match self.tokens.next() {
                Some(x) => x,
                None => break ExprTerminator::End,
            };
        };

        if exprs.len() != ops.len() + 1 {
            return Err(ParserError::invalid_expr(
                start,
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

        let mut require_symbol = true;

        let final_token = loop {
            let next = match self.tokens.next() {
                Some(x) => x,
                None => break None,
            };
            println!("Investigate selector symbol {}", next);
            match next {
                Token::BareString(s) => path.push(SelectorElement::Constant(s)),
                _ => {
                    if require_symbol {
                        return Err(ParserError::incorrect_symbol(
                            self.tokens.span(),
                            next.to_string(),
                        ));
                    } else {
                        break Some(next);
                    }
                }
            }
            let next = match self.tokens.next() {
                Some(x) => x,
                None => break None,
            };
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
        if path.is_empty() {
            return Err(ParserError::empty_expression(self.tokens.span()));
        }
        let expr = SelectorExpression::new(path.remove(0), path);
        println!("Got selector {}", expr);
        Ok((expr, final_token))
    }
}

#[cfg(test)]
pub mod test {
    use std::collections::HashMap;

    use logos::Logos;
    use serde_json::json;

    use crate::{
        expressions::{Expression, ExpressionExecutionState},
        lexer::Token,
    };

    use super::Parser;

    #[test]
    pub fn test_parser() {
        let mut input = HashMap::new();
        let inp = json!({
            "elem": 3
        });
        input.insert("id".to_string(), inp);

        let lex = Token::lexer("2 + 2 * $id.elem - 3 * 3 + pow(2, 2)");

        let res = Parser::new(lex).parse().unwrap();
        let state = ExpressionExecutionState { data: input };
    }
}
