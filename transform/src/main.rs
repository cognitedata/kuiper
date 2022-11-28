use std::collections::HashMap;

use logos::Logos;
use serde_json::json;

use crate::{
    expressions::{Expression, ExpressionExecutionState},
    lexer::Token,
    parse::Parser,
};

mod expressions;
mod lexer;
mod parse;

fn main() {
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
}
