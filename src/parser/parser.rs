use logos::{Lexer, Span};

use crate::{
    expressions::{
        Constant, Expression, ExpressionType, FunctionExpression, FunctionType, OpExpression,
        PowFunction, SelectorExpression,
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
    state: ParserState,
    pos: usize,
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
        Self {
            tokens: stream,
            state: ParserState::Start,
            pos: 0,
        }
    }

    pub fn parse(&mut self) -> Result<ExpressionType, ParserError> {
        let (expr, term) = self.parse_expression()?;
        if term == ExprTerminator::End {
            Ok(expr)
        } else {
            Err(ParserError::empty_expression(self.tokens.span()))
        }
    }

    fn parse_expression(&mut self) -> Result<(ExpressionType, ExprTerminator), ParserError> {
        let start = self.tokens.span();
        let mut exprs = vec![];
        self.pos += self.tokens.slice().len();
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
                    if exprs.len() != 1 {
                        return Err(ParserError::invalid_expr(start, "Expected operator"));
                    }
                    let lhs = exprs.drain(..).next().unwrap();
                    let (rhs, term) = self.parse_expression()?;
                    let expr = OpExpression::new(o, lhs, rhs);
                    exprs.push(ExpressionType::Operator(expr));
                    break term;
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
                    self.pos += self.tokens.slice().len();
                    match next {
                        Some(x) => token = x,
                        None => break ExprTerminator::End,
                    }
                    continue;
                }
            }
            self.pos += self.tokens.slice().len();
            token = match self.tokens.next() {
                Some(x) => x,
                None => break ExprTerminator::End,
            };
        };

        if exprs.len() != 1 {
            return Err(ParserError::empty_expression(self.tokens.span()));
        }
        let expr = exprs.drain(..).next().unwrap();
        Ok((expr, term))
    }

    fn parse_selector(&mut self) -> Result<(SelectorExpression, Option<Token>), ParserError> {
        let mut path = vec![];
        let final_token = loop {
            self.pos += self.tokens.slice().len();
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
            self.pos += self.tokens.slice().len();
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
            "elem": 321
        });
        input.insert("id".to_string(), inp);

        let lex = Token::lexer("((123 + 4) * $id.elem) + (321 * 123) + pow(1, 2)");

        let res = Parser::new(lex).parse().unwrap();
        let state = ExpressionExecutionState { data: input };
        println!("{}", res.resolve(&state).unwrap());

        println!("{}", res);
        panic!("test");
    }
}
