use std::collections::HashMap;

use logos::Logos;
use program::{Program, TransformInput};
use serde_json::json;

use crate::{
    expressions::{Expression, ExpressionExecutionState},
    lexer::Token,
    parse::Parser,
};

mod expressions;
mod lexer;
mod parse;
mod program;

fn main() {
    let raw: Vec<TransformInput> = serde_json::from_value(json!([
        {
            "id": "step1",
            "inputs": ["input"],
            "transform": {
                "externalId": "$input.id",
                "value": "$input.value",
                "timestamp": "$input.timestamp"
            }
        },
        {
            "id": "unused",
            "inputs": ["input"],
            "transform": {
                "some-unused-transform": "This can contain syntax errors",
                "since": "it won't be compiled"
            }
        },
        {
            "id": "step2",
            "inputs": ["input", "step1"],
            "transform": {
                "externalId": "$input.id2",
                "nested": "$step1"
            }
        }
    ]))
    .unwrap();

    let program = Program::compile(raw).unwrap();
    let input = json!({
        "id": "my-id",
        "id2": "my-other-id",
        "value": 123.321,
        "timestamp": 12395184235i64
    });
    let res = program.execute(input).unwrap();
    println!("{}", res);

    /*let mut input = HashMap::new();
    let inp = json!({
        "elem": 3,
        "elem2": 4,
        "arr": [1, 2, 3],
        "deep": [{
            "nested": 10
        }]
    });
    input.insert("id".to_string(), &inp);

    // Fancy array selectors
    let lex = Token::lexer("1 + $id.elem + 2 + 3 + floor(2.5)");
    let res = Parser::new(lex).parse().unwrap();
    let state = ExpressionExecutionState { data: input };
    println!("{}", res.resolve(&state).unwrap().as_ref());
    println!("{}", res);

    // let lex = Token::lexer("[0, 1, 2, 3, $id.arr]");
    let lex = Token::lexer("$id.deep[0].nested");
    let res = Parser::new(lex).parse().unwrap();
    println!("{}", res.resolve(&state).unwrap().as_ref());

    println!("{}", res); */
}
