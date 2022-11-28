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
        "elem": 3,
        "elem2": 4,
        "arr": [1, 2, 3]
    });
    input.insert("id".to_string(), inp);

    // Fancy array selectors
    let lex = Token::lexer("1 + $id.elem + 2 + 3");
    let res = Parser::new(lex).parse().unwrap();
    let state = ExpressionExecutionState { data: input };
    println!("{}", res.resolve(&state).unwrap().as_ref());
    println!("{}", res);

    let lex = Token::lexer("[0, 1, 2, 3, $id.arr]");
    let res = Parser::new(lex).parse().unwrap();
    println!("{}", res.resolve(&state).unwrap().as_ref());

    println!("{}", res);
}
