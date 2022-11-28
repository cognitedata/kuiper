use logos::{Lexer, Span};

use crate::{
    expressions::{
        Constant, Expression, ExpressionType, FunctionExpression, FunctionType, OpExpression,
        Operator, PowFunction, SelectorExpression,
    },
    lexer::Token,
};

use super::parse_error::ParserError;

enum ParserState {
    Start,
    End,
}

pub struct Parser<'source> {
    tokens: Lexer<'source, Token>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ExprTerminator {
    Comma,
    CloseParenthesis,
    End,
}

fn get_function_expression(
    pos: Span,
    name: &str,
    args: Vec<ExpressionType>,
) -> Result<ExpressionType, ParserError> {
    let info = match name {
        "pow" => PowFunction::INFO,
        _ => return Err(ParserError::incorrect_symbol(pos, name.to_string())),
    };

    if !info.validate(args.len()) {
        return Err(ParserError::n_function_args(pos, &info.num_args_desc()));
    }

    let expr = match info.name {
        "pow" => {
            let mut iter = args.into_iter();
            FunctionType::Pow(PowFunction::new(iter.next().unwrap(), iter.next().unwrap()))
        }
        _ => unreachable!(),
    };
    Ok(ExpressionType::Function(expr))
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

    fn group_expressions(ops: Vec<Operator>, exprs: Vec<ExpressionType>) -> ExpressionType {
        let mut lowest = 1000;
        let mut idx: i64 = -1;

        for i in 0..ops.len() {
            if ops[i].priority() <= lowest {
                lowest = ops[i].priority();
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
                lhs_ops.push(ops[i as usize]);
            }
        }
        let rhs = drain.collect();
        let mut rhs_ops = vec![];
        for i in (idx + 1)..(ops.len() as i64) {
            rhs_ops.push(ops[i as usize]);
        }
        let lhs = Self::group_expressions(lhs_ops, lhs);
        let rhs = Self::group_expressions(rhs_ops, rhs);
        ExpressionType::Operator(OpExpression::new(ops[idx as usize], lhs, rhs))
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
            println!("Investigate symbol {}", token.to_string());
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
                        token.to_string(),
                    ))
                }
                Token::Operator(o) => {
                    /* if exprs.len() != 1 {
                        return Err(ParserError::invalid_expr(start, "Expected operator"));
                    }
                    let lhs = exprs.drain(..).next().unwrap();
                    let (rhs, term) = self.parse_expression()?;
                    let expr = OpExpression::new(o, lhs, rhs);
                    exprs.push(ExpressionType::Operator(expr)); */
                    ops.push(o)
                }
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
                    token = match self.tokens.next() {
                        Some(x) => x,
                        None => return Err(ParserError::empty_expression(self.tokens.span())),
                    };
                    match token {
                        Token::OpenParenthesis => (),
                        _ => {
                            return Err(ParserError::incorrect_symbol(
                                self.tokens.span(),
                                token.to_string(),
                            ));
                        }
                    }
                    let mut args = vec![];
                    loop {
                        let (expr, term) = self.parse_expression()?;
                        args.push(expr);
                        match term {
                            ExprTerminator::CloseParenthesis => break,
                            ExprTerminator::End => {
                                return Err(ParserError::empty_expression(self.tokens.span()))
                            }
                            ExprTerminator::Comma => (),
                        }
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

    fn parse_selector(&mut self) -> Result<(SelectorExpression, Option<Token>), ParserError> {
        let mut path = vec![];
        let final_token = loop {
            let next = match self.tokens.next() {
                Some(x) => x,
                None => break None,
            };
            println!("Investigate selector symbol {}", next);
            match next {
                Token::BareString(s) => path.push(s),
                _ => {
                    return Err(ParserError::incorrect_symbol(
                        self.tokens.span(),
                        next.to_string(),
                    ))
                }
            }
            let next = match self.tokens.next() {
                Some(x) => x,
                None => break None,
            };
            match next {
                Token::Period => (),
                _ => break Some(next),
            }
        };
        if path.is_empty() {
            return Err(ParserError::empty_expression(self.tokens.span()));
        }
        let expr = SelectorExpression::new(path.remove(0), path);
        println!("Got selector {}", expr.to_string());
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
        println!("{}", res.resolve(&state).unwrap());

        println!("{}", res);
        panic!("test");
    }
}
